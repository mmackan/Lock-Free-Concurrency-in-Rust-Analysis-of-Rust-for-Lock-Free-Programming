use std::sync::Arc;

use aarc::{AtomicArc, Snapshot};

use crossbeam_utils::CachePadded;

use crate::shared_queue::SharedQueue;

use super::prq::PRQ;

pub struct SharedLPRQ<T: 'static, const N: usize> {
    queue: Arc<LPRQ<T, N>>,
}

impl<T, const N: usize> SharedQueue<T> for SharedLPRQ<T, N> {
    fn new() -> Self {
        Self {
            queue: Arc::new(LPRQ::<T, N>::new()),
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

struct LPRQ<T: 'static, const N: usize> {
    head: CachePadded<AtomicArc<PRQ<T, N>>>,
    tail: CachePadded<AtomicArc<PRQ<T, N>>>,
}

impl<T: 'static, const N: usize> Drop for LPRQ<T, N> {
    fn drop(&mut self) {
        // Empty the queue to drop any leftover items
        while let Some(_) = self.dequeue() {}
    }
}

impl<T: 'static, const N: usize> LPRQ<T, N> {
    fn new() -> Self {
        let initial: Arc<PRQ<T, N>> = Arc::new(PRQ::new());
        Self {
            head: CachePadded::new((&initial).into()),
            tail: CachePadded::new((&initial).into()),
        }
    }
    fn enqueue(&self, val: *const T) {
        loop {
            // fast path: Add item to current PRQ
            let queue: Arc<PRQ<T, N>> = self.tail.load().unwrap();
            match queue.enqueue(val) {
                Ok(_) => return,
                Err(_) => {
                    // Slow path: Tail is full, allocate and add a new crq
                    let new_tail: Arc<PRQ<T, N>> = Arc::new(PRQ::new_with_item(val));
                    match queue
                        .next
                        .compare_exchange::<Arc<_>, Arc<_>, Snapshot<_>>(None, Some(&new_tail))
                    {
                        Ok(_) => {
                            // Next successfully inserted, update tail to point to that
                            let _ = self.tail.compare_exchange::<Arc<_>, Arc<_>, Snapshot<_>>(
                                Some(&queue),
                                Some(&new_tail),
                            );
                            return;
                        }
                        Err(next) => {
                            let _ = self
                                .tail
                                .compare_exchange::<Arc<_>, Snapshot<_>, Snapshot<_>>(
                                    Some(&queue),
                                    next.as_ref(),
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
            let queue: Arc<PRQ<T, N>> = self.head.load().unwrap();
            match queue.dequeue() {
                Some(v) => {
                    return Some(v);
                }
                None => {
                    // Failed, is the queue empty?
                    match queue.next.load::<Snapshot<_>>() {
                        Some(next) => {
                            // Not empty, try to dequeue again
                            match queue.dequeue() {
                                Some(v) => {
                                    return Some(v);
                                }
                                None => {
                                    // PRQ is empty, update head and restart
                                    let _ = self
                                        .head
                                        .compare_exchange::<Arc<_>, Snapshot<_>, Snapshot<_>>(
                                            Some(&queue),
                                            Some(&next),
                                        );
                                    continue;
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
// Disabeling tests for now
/*
#[cfg(test)]
mod test {
    use std::{sync::Arc, thread};

    use super::LPRQ;

    #[test]
    fn basic() {
        let queue: LPRQ<i32, 10> = LPRQ::new();
        for i in 0..123 {
            queue.enqueue(i);
        }
        for i in 0..123 {
            assert_eq!(queue.dequeue(), Some(i));
        }
    }

    #[test]
    fn basic_concurrent() {
        let queue: Arc<LPRQ<i32, 10>> = Arc::new(LPRQ::new());

        let mut handles = vec![];

        for i in 0..10 {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                for j in 0..23 {
                    queue.enqueue(j + i)
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
                for _j in 0..23 {
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
                for j in 0..2 {
                    queue.enqueue(j + i)
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
                for _j in 0..1 {
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
*/
