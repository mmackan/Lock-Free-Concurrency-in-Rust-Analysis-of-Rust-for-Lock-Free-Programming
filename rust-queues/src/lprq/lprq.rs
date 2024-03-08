use std::{
    ptr,
    sync::{atomic::AtomicUsize, Arc},
};

use haphazard::{AtomicPtr, HazardPointer};

use crossbeam_utils::CachePadded;

use crate::shared_queue::SharedQueue;

use super::prq::PRQ;

pub struct SharedLPRQ<'a, T, const N: usize> {
    queue: Arc<LPRQ<T, N>>,
    hazard1: HazardPointer<'a>,
    hazard2: HazardPointer<'a>,
}

impl<'a, T, const N: usize> SharedQueue<T> for SharedLPRQ<'a, T, N> {
    fn new() -> Self {
        Self {
            queue: Arc::new(LPRQ::new()),
            hazard1: HazardPointer::new(),
            hazard2: HazardPointer::new(),
        }
    }

    fn enqueue(&mut self, val: T) {
        self.queue.enqueue(val, &mut self.hazard1)
    }

    fn dequeue(&mut self) -> Option<T> {
        self.queue.dequeue(&mut self.hazard1, &mut self.hazard2)
    }
}

impl<'a, T, const N: usize> Clone for SharedLPRQ<'a, T, N> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            hazard1: HazardPointer::new(),
            hazard2: HazardPointer::new(),
        }
    }
}

struct LPRQ<T, const N: usize> {
    head: CachePadded<AtomicPtr<PRQ<T, N>>>,
    tail: CachePadded<AtomicPtr<PRQ<T, N>>>,
}

impl<T, const N: usize> Drop for LPRQ<T, N> {
    fn drop(&mut self) {
        // Empty the queue to drop any leftover items
        let mut hazard1 = HazardPointer::new();
        let mut hazard2 = HazardPointer::new();
        while let Some(_) = self.dequeue(&mut hazard1, &mut hazard2) {}

        let head = self.head.load_ptr();
        let tail = self.tail.load_ptr();
        // The queue should be empty now, but dubblecheck for safety
        if head == tail {
            let old =  unsafe { self.head.swap_ptr(ptr::null_mut())}.expect("A LPRQ with both head and tail as null was dropped. This should never happen and indicates a bug or memory corruption");
            unsafe { old.retire() };
        } else {
            panic!("Drop for LPRQ somehow failed to dequeue all its items")
        }
    }
}

impl<T, const N: usize> LPRQ<T, N> {
    fn new() -> Self {
        let initial: *mut PRQ<T, N> = Box::into_raw(Box::new(PRQ::new()));
        Self {
            head: unsafe { AtomicPtr::new(initial) }.into(),
            tail: unsafe { AtomicPtr::new(initial) }.into(),
        }
    }
    fn enqueue(&self, val: T, hazard: &mut HazardPointer) {
        let boxed_val = Box::new(val);
        let value = Box::into_raw(boxed_val);
        loop {
            // fast path: Add item to current PRQ
            let queue = self.tail.safe_load(hazard).unwrap();
            let queue_ptr: *const PRQ<T, N> = queue;
            match queue.enqueue(value) {
                Ok(_) => return,
                Err(_) => {
                    // Slow path: Tail is full, allocate and add a new crq
                    let new_tail: AtomicPtr<PRQ<T, N>> =
                        AtomicPtr::from(Box::new(PRQ::new_with_item(value)));
                    let new_tail_ptr = new_tail.load_ptr();
                    match unsafe {
                        queue
                            .next
                            .compare_exchange_ptr(ptr::null_mut(), new_tail_ptr)
                    } {
                        Ok(_) => {
                            // Next successfully inserted, update tail to point to that
                            let _ = unsafe {
                                self.tail
                                    .compare_exchange_ptr(queue_ptr.cast_mut(), new_tail_ptr)
                            };
                            return;
                        }
                        Err(next) => {
                            let _ = unsafe {
                                self.tail.compare_exchange_ptr(queue_ptr.cast_mut(), next)
                            };
                            // Drop the failed new tail so it does not leak
                            let _ = unsafe { new_tail.retire() };
                            continue;
                        }
                    }
                }
            }
        }
    }
    fn dequeue(&self, hazard1: &mut HazardPointer, hazard2: &mut HazardPointer) -> Option<T> {
        loop {
            let queue = self.head.safe_load(hazard1).unwrap();
            match queue.dequeue() {
                Some(v) => {
                    let value = unsafe { Box::from_raw(v) };
                    return Some(*value);
                }
                None => {
                    // Failed, is this queue empty?
                    match hazard2.protect_ptr(unsafe { queue.next.as_std() }) {
                        Some(next_ptr) => {
                            // LPRQ is not empty, try to dequeue again
                            match queue.dequeue() {
                                Some(value) => {
                                    let value = unsafe { Box::from_raw(value) };
                                    return Some(*value);
                                }
                                None => {
                                    // PRQ is empty, update head and restart
                                    let queue_ptr: *const PRQ<T, N> = queue;
                                    match unsafe {
                                        self.head.compare_exchange_ptr(
                                            queue_ptr.cast_mut(),
                                            next_ptr.0.as_ptr(),
                                        )
                                    } {
                                        Ok(Some(old)) => {
                                            // The old PRQ is now empty, so we retire it
                                            unsafe { old.retire() };
                                            continue;
                                        }
                                        Ok(None) => {
                                            // Null ptr somehow made it here, should be impossible
                                            panic!("Queue somehow turned into a null pointer despite it being used before, should be impossible")
                                        }
                                        Err(_) => {
                                            // Update failed, we are entierly out of sync so just restart
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            // Queue is empty
                            return None;
                        }
                    }
                }
            }
        }
    }
}

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
            queue.enqueue(i, &mut hazard);
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
