use std::{ptr, sync::atomic::Ordering::SeqCst, sync::Arc};

use crossbeam_utils::CachePadded;

use crossbeam_epoch::{self as epoch, Atomic, CompareExchangeError, Shared, Guard};

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
        let guard = epoch::pin();
        self.queue.enqueue(val, &guard)
    }

    fn dequeue(&mut self) -> Option<*const T> {
        let guard = epoch::pin();
        self.queue.dequeue(&guard)
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
        let mut guard = epoch::pin();
        // Empty the queue to drop any leftover items
        while self.dequeue(&guard).is_some() {}

        guard.repin();
        guard.flush();
        guard.repin();
        guard.flush();
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
    fn enqueue(&self, val: *const T, guard: &Guard) {
        loop {
            // fast path: Add item to current PRQ
            let queue_shared = self.tail.load(SeqCst, guard);
            let queue = unsafe { queue_shared.deref() };
            match queue.enqueue(val) {
                Ok(_) => return,
                Err(_) => {
                    // Slow path: Tail is full, allocate and add a new crq
                    let new_tail: Atomic<PRQ<T, N>> = Atomic::new(PRQ::new_with_item(val));
                    // load_consume is ok here, should not make a big difference on x86, but it
                    // seems correct according to the docs
                    let new_tail_shared = new_tail.load(SeqCst, guard);
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
                            unsafe {guard.defer_destroy(new_tail_shared)};
                            continue;
                        }
                    }
                }
            }
        }
    }
    fn dequeue(&self, guard: &Guard) -> Option<*const T> {
        loop {
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
                                        unsafe {
                                            guard.defer_destroy(old);
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


#[cfg(test)]
mod test {
    use std::{sync::Arc, thread};
    use crossbeam_epoch::{self as epoch};

    use super::LPRQ;
    const NUMBERS: [i32;100] = {
        let mut output = [0;100];
        let mut i = 0;
        while i < 100 {
            output[i as usize] = i;
            i += 1;
        }
        output
    };


    #[test]
    fn basic() {
        let guard = epoch::pin();
        let queue: LPRQ<i32, 9> = LPRQ::new();
        for i in NUMBERS {
            queue.enqueue((&NUMBERS[i as usize]) as *const _, &guard);
        }
        for i in NUMBERS {
            let v = queue.dequeue(&guard).unwrap();
            assert_eq!(unsafe {*v}, NUMBERS[i as usize]);
        }
    }

    #[test]
    fn basic_concurrent() {
        let queue: Arc<LPRQ<i32, 10>> = Arc::new(LPRQ::new());

        let mut handles = vec![];

        for i in 0..10 {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let guard = epoch::pin();
                    queue.enqueue(&NUMBERS[j + i], &guard)
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
                for _j in 0..10 {
                    let guard = epoch::pin();
                    queue.dequeue(&guard).unwrap();
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            let _ = handle.join();
        }
    }
    #[test]
    fn dropping_with_non_empty() {
        let queue: Arc<LPRQ<i32, 10>> = Arc::new(LPRQ::new());

        let mut handles = vec![];

        for i in 0..10 {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let guard = epoch::pin();
                    queue.enqueue(&NUMBERS[j + i], &guard)
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
                for _j in 0..5 {
                    let guard = epoch::pin();
                    queue.dequeue(&guard).unwrap();
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            let _ = handle.join();
        }
        drop(queue);
    }
}
