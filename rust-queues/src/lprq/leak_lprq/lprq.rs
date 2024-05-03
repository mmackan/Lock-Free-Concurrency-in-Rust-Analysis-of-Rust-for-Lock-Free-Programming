use std::{ptr, sync::atomic::AtomicPtr, sync::atomic::Ordering::SeqCst, sync::Arc};

use crossbeam_utils::CachePadded;

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
    head: CachePadded<AtomicPtr<PRQ<T, N>>>,
    tail: CachePadded<AtomicPtr<PRQ<T, N>>>,
}

impl<T, const N: usize> Drop for LPRQ<T, N> {
    fn drop(&mut self) {
        // Empty the queue to drop any leftover items
        while let Some(_) = self.dequeue() {}

        let head = self.head.load(SeqCst);
        let tail = self.tail.load(SeqCst);
        // The queue should be empty now, but dubblecheck for safety
        if head == tail {
            let _old = self.head.swap(ptr::null_mut(), SeqCst);
        } else {
            panic!("Drop for LPRQ somehow failed to dequeue all its items")
        }
    }
}

impl<T, const N: usize> LPRQ<T, N> {
    fn new() -> Self {
        let initial: *mut PRQ<T, N> = Box::into_raw(Box::new(PRQ::new()));
        Self {
            head: AtomicPtr::new(initial).into(),
            tail: AtomicPtr::new(initial).into(),
        }
    }
    fn enqueue(&self, val: *const T) {
        loop {
            // fast path: Add item to current PRQ
            let queue_ptr: *const PRQ<T, N> = self.tail.load(SeqCst);
            let queue: &PRQ<T, N> = unsafe { queue_ptr.as_ref().unwrap() };
            match queue.enqueue(val) {
                Ok(_) => return,
                Err(_) => {
                    // Slow path: Tail is full, allocate and add a new crq
                    let new_tail_ptr: *mut PRQ<T, N> =
                        Box::into_raw(Box::new(PRQ::new_with_item(val)));
                    match queue
                        .next
                        .compare_exchange(ptr::null_mut(), new_tail_ptr, SeqCst, SeqCst)
                    {
                        Ok(_) => {
                            // Next successfully inserted, update tail to point to that
                            let _ = self.tail.compare_exchange(
                                queue_ptr.cast_mut(),
                                new_tail_ptr,
                                SeqCst,
                                SeqCst,
                            );
                            return;
                        }
                        Err(next) => {
                            let _ = self.tail.compare_exchange(
                                queue_ptr.cast_mut(),
                                next,
                                SeqCst,
                                SeqCst,
                            );
                            continue;
                        }
                    }
                }
            }
        }
    }
    fn dequeue(&self) -> Option<*const T> {
        loop {
            let queue = unsafe { self.head.load(SeqCst).as_ref().unwrap() };
            match queue.dequeue() {
                Some(v) => {
                    return Some(v);
                }
                None => {
                    // Failed, is this queue empty?
                    let next_ptr = queue.next.load(SeqCst);
                    if !next_ptr.is_null() {
                        // LPRQ is not empty, try to dequeue again
                        match queue.dequeue() {
                            Some(value) => {
                                return Some(value);
                            }
                            None => {
                                // PRQ is empty, update head and restart
                                let queue_ptr: *const PRQ<T, N> = queue;
                                let _ = self.head.compare_exchange(
                                    queue_ptr.cast_mut(),
                                    next_ptr,
                                    SeqCst,
                                    SeqCst,
                                );
                            }
                        }
                    } else {
                        // Queue is empty
                        return None;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{sync::Arc, thread};

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
        let queue: LPRQ<i32, 10> = LPRQ::new();
        for i in NUMBERS {
            queue.enqueue((&NUMBERS[i as usize]) as *const _);
        }
        for i in NUMBERS {
            let v = queue.dequeue().unwrap();
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
                    queue.enqueue(&NUMBERS[j + i])
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
                    queue.dequeue().unwrap();
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            let _ = handle.join();
        }
        drop(queue);
    }
    #[test]
    fn dropping_with_non_empty() {
        let queue: Arc<LPRQ<i32, 10>> = Arc::new(LPRQ::new());

        let mut handles = vec![];

        for i in 0..10 {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    queue.enqueue(&NUMBERS[j + i])
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
                    queue.dequeue().unwrap();
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
