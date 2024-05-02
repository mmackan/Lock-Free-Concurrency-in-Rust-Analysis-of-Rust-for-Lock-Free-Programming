use std::{ptr, sync::atomic::Ordering::SeqCst, sync::Arc};

use crossbeam_utils::CachePadded;

use crossbeam_epoch::{self as epoch, Atomic, CompareExchangeError, Shared};

use crate::shared_queue::SharedQueue;

use super::prq::PRQ;

pub struct SharedLPRQ<T, const N: usize> {
    queue: Arc<LPRQ<T, N>>,
}

impl<T, const N: usize> SharedQueue<T> for SharedLPRQ<T, N> {
    fn new() -> Self {
        Self {
            queue: Arc::new(LPRQ::new()),
        }
    }

    fn enqueue(&mut self, val: *const T) {
        self.queue.enqueue(val)
    }

    fn dequeue(&mut self) -> Option<*const T> {
        self.queue.dequeue()
    }
}

impl<T, const N: usize> Clone for SharedLPRQ<T, N> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
        }
    }
}

struct LPRQ<T, const N: usize> {
    head: CachePadded<Atomic<PRQ<T, N>>>,
    tail: CachePadded<Atomic<PRQ<T, N>>>,
}

impl<T, const N: usize> Drop for LPRQ<T, N> {
    fn drop(&mut self) {
        // Empty the queue to drop any leftover items
        while let Some(_) = self.dequeue() {}

        let guard = &epoch::pin();

        let head = self.head.load(SeqCst, guard);
        let tail = self.tail.load(SeqCst, guard);
        // The queue should be empty now, but dubblecheck for safety
        if head == tail {
            let _ = unsafe { head.into_owned() };
        } else {
            panic!("Drop for LPRQ somehow failed to dequeue all its items")
        }
    }
}

impl<T, const N: usize> LPRQ<T, N> {
    fn new() -> Self {
        let initial = Atomic::new(PRQ::new());
        Self {
            head: initial.clone().into(),
            tail: initial.into(),
        }
    }
    fn enqueue(&self, val: *const T) {
        loop {
            // fast path: Add item to current PRQ
            let guard = &epoch::pin();
            let queue_shared = self.tail.load(SeqCst, guard);
            let queue = unsafe { queue_shared.deref() };
            match queue.enqueue(val) {
                Ok(_) => return,
                Err(_) => {
                    // Slow path: Tail is full, allocate and add a new crq
                    let new_tail: Atomic<PRQ<T, N>> = Atomic::new(PRQ::new_with_item(val));
                    // load_consume is ok here, should not make a big difference on x86, but it
                    // seems correct according to the docs
                    let new_tail_shared = new_tail.load_consume(guard);
                    match queue.next.compare_exchange(
                        Shared::null(),
                        new_tail_shared,
                        SeqCst,
                        SeqCst,
                        guard,
                    ) {
                        Ok(_) => {
                            // Next successfully inserted, update tail to point to that
                            let _ = self.tail.compare_exchange(
                                queue_shared,
                                new_tail_shared,
                                SeqCst,
                                SeqCst,
                                guard,
                            );
                            return;
                        }
                        Err(CompareExchangeError {
                            current: next,
                            new: _,
                        }) => {
                            let _ = self.tail.compare_exchange(
                                queue_shared,
                                next,
                                SeqCst,
                                SeqCst,
                                guard,
                            );
                            // Drop the failed new tail so it does not leak
                            // Since the failed new tail did not get written anywhere, we can save
                            // some time by not defering a deallocation and instead removing it
                            // directly
                            let _ = unsafe { new_tail.into_owned() };
                            continue;
                        }
                    }
                }
            }
        }
    }
    fn dequeue(&self) -> Option<*const T> {
        loop {
            let guard = &epoch::pin();
            let queue_shared = self.head.load(SeqCst, guard);
            let queue = unsafe { queue_shared.deref() };
            match queue.dequeue() {
                Some(v) => {
                    return Some(v);
                }
                None => {
                    // Failed, is this queue empty?
                    let next = queue.next.load(SeqCst, guard);
                    if !next.is_null() {
                        // LPRQ is not empty, try to dequeue again
                        match queue.dequeue() {
                            Some(value) => {
                                return Some(value);
                            }
                            None => {
                                // PRQ is empty, update head and restart
                                match self.head.compare_exchange(
                                    queue_shared,
                                    next,
                                    SeqCst,
                                    SeqCst,
                                    guard,
                                ) {
                                    Ok(old) => {
                                        // The old PRQ is now empty, so we defer deleting it
                                        // Safety: The pointer is send, but rust can't prove it
                                        // so we have to use defer_unchecked
                                        unsafe {
                                            guard.defer_unchecked(move || old.into_owned());
                                        }
                                        continue;
                                    }
                                    Err(_) => {
                                        // Update failed, we are entierly out of sync so just restart
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    // Queue is empty
                    return None;
                }
            }
        }
    }
}

// Turn off the tests for now
/*
#[cfg(test)]
mod test {
    use std::{sync::Arc, thread};

    use haphazard::{Domain, HazardPointer};

    use super::LPRQ;

    #[test]
    fn basic() {
        let queue: LPRQ<i32, 10> = LPRQ::new();
        let mut hazard = HazardPointer::new();
        for i in 0..123 {
            queue.enqueue((&i) as *const _, &mut hazard);
        }
        let mut hazard2 = HazardPointer::new();
        for i in 0..123 {
            assert_eq!(queue.dequeue(&mut hazard, &mut hazard2), Some(i));
        }
    }

    #[test]
    fn basic_concurrent() {
        let queue: Arc<LPRQ<i32, 10>> = Arc::new(LPRQ::new());

        let mut handles = vec![];

        for i in 0..10 {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                let mut hazard = HazardPointer::new();
                for j in 0..23 {
                    queue.enqueue(j + i, &mut hazard)
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        handles = vec![];

        for _i in 0..10 {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                let mut hazard1 = HazardPointer::new();
                let mut hazard2 = HazardPointer::new();
                for _j in 0..23 {
                    queue.dequeue(&mut hazard1, &mut hazard2).unwrap();
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            let _ = handle.join();
        }
        drop(queue);
        Domain::global().eager_reclaim();
    }
    #[test]
    fn dropping_with_non_empty() {
        let queue: Arc<LPRQ<i32, 10>> = Arc::new(LPRQ::new());

        let mut handles = vec![];

        for i in 0..10 {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                let mut hazard = HazardPointer::new();
                for j in 0..2 {
                    queue.enqueue(j + i, &mut hazard)
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        handles = vec![];

        for _i in 0..10 {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                let mut hazard1 = HazardPointer::new();
                let mut hazard2 = HazardPointer::new();
                for _j in 0..1 {
                    queue.dequeue(&mut hazard1, &mut hazard2).unwrap();
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            let _ = handle.join();
        }
        drop(queue);
        Domain::global().eager_reclaim();
    }
}
*/
