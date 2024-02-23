use std::ptr;

use haphazard::{AtomicPtr, HazardPointer, Replaced};

use super::crq::PRQ;



struct LPRQ<T, const N: usize> {
    head: AtomicPtr<PRQ<T, N>>,
    tail: AtomicPtr<PRQ<T, N>>
}

impl<T, const N: usize> Drop for LPRQ<T, N> {
    fn drop(&mut self) {
        let head = self.head.load_ptr();
        let tail = self.tail.load_ptr();
        // If head and tail point to the same PRQ make sure to only drop it once
        if head == tail {
            let old =  unsafe { self.head.swap_ptr(ptr::null_mut())}.expect("A LPRQ with both head and tail as null was dropped. This should never happen and indicates a bug or memory corruption");
            unsafe { old.retire()};
        } else {
            // Head and tail point to different queues, so both need to be dropped
            let head = unsafe { self.head.swap_ptr(ptr::null_mut())}.expect("LPRQ dropped with a null head! Should never happpen");
            let tail = unsafe { self.tail.swap_ptr(ptr::null_mut())}.expect("LPRQ dropped with a null tail! Should never happpen");
            unsafe {
                head.retire();
                tail.retire();
            }
        }
    }
}

impl<T, const N: usize> LPRQ<T, N> {
    fn new() -> Self {
        let initial: *mut PRQ<T, N> = Box::into_raw(Box::new(PRQ::new()));
        Self {
            head: unsafe{AtomicPtr::new(initial)},
            tail: unsafe{AtomicPtr::new(initial)},
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
                    let new_tail: Box<PRQ<T, N>> = Box::new(PRQ::new());
                    new_tail.enqueue(value).expect("New tail somehow failed to enqueue a single item despite the fact that it shouold be empty");
                    let new_tail_ptr = Box::into_raw(new_tail);
                    match unsafe {queue.next.compare_exchange_ptr(ptr::null_mut(), new_tail_ptr)} {
                        Ok(_) => {
                            // Next successfully inserted, update tail to point to that
                            let _ = unsafe{self.tail.compare_exchange_ptr(queue_ptr.cast_mut(), new_tail_ptr)};
                            return;
                        },
                        Err(next) => {
                            let _ = unsafe {
                                self.tail.compare_exchange_ptr(queue_ptr.cast_mut(), next)
                            };
                            continue
                        },
                    }
                },
            }
        }
    }
    fn dequeue(&self, hazard1: &mut HazardPointer, hazard2: &mut HazardPointer) -> Option<T> {
        loop {
            let queue = self.head.safe_load(hazard1).unwrap();
            match queue.dequeue() {
                Some(v) => {
                    let value = unsafe {Box::from_raw(v)};
                    return Some(*value);
                },
                None => {
                    // Failed, is this queue empty?
                    match queue.next.safe_load(hazard2) {
                        Some(next) => {
                            // LPRQ is not empty, try to dequeue again
                            match queue.dequeue() {
                                Some(value) => {
                                    let value = unsafe {Box::from_raw(value)};
                                    return Some(*value);
                                },
                                None => {
                                    // PRQ is empty, update head and restart
                                    let next_ptr: *const PRQ<T,N> = next;
                                    let queue_ptr: *const PRQ<T,N> = queue;
                                    match unsafe { self.head.compare_exchange_ptr(queue_ptr.cast_mut(), next_ptr.cast_mut()) } {
                                        Ok(Some(old)) => {
                                            // The old PRQ is now empty, so we retire it
                                            unsafe { old.retire() };
                                            continue;
                                        },
                                        Ok(None) => {
                                            // Null ptr somehow made it here, should be impossible
                                            panic!("Queue somehow turned into a null pointer despite it being used before, should be impossible")
                                        }
                                        Err(_) => {
                                            // Update failed, we are entierly out of sync so just restart
                                            continue
                                        },
                                    }
                                },
                            }
                        },
                        None => {
                            // Queue is empty
                            return None
                        },
                    }
                },
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
                for j in 0..234 {
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
                for _j in 0..234 {
                    queue.dequeue(&mut hazard1, &mut hazard2).unwrap();
                }
            });
            handles.push(handle);
        }
        drop(queue);
        Domain::global().eager_reclaim();
    }
}