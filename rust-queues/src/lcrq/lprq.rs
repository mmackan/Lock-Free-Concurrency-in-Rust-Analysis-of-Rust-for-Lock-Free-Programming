use std::ptr;

use haphazard::{AtomicPtr, HazardPointer};

use super::crq::PRQ;



struct LPRQ<T, const N: usize> {
    head: AtomicPtr<PRQ<T, N>>,
    tail: AtomicPtr<PRQ<T, N>>
}

impl<T, const N: usize> LPRQ<T, N> {
    fn new() -> Self {
        let initial: *mut PRQ<T, N> = Box::into_raw(Box::new(PRQ::new()));
        Self {
            head: unsafe{AtomicPtr::new(initial)},
            tail: unsafe{AtomicPtr::new(initial)},
        }
    }
    fn enqueue(&self, val: Box<T>, hazard: &mut HazardPointer) {
        let value = Box::into_raw(val);

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
    fn dequeue(&self, hazard: &mut HazardPointer) -> Option<T> {
        todo!()
    }
}