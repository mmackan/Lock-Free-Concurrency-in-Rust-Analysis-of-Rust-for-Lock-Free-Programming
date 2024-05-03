use crate::shared_queue::SharedQueue;
use haphazard::{AtomicPtr, HazardPointer};
use std::{fmt::Debug, ptr, sync::Arc};

pub struct MSQueue<'a, T> {
    queue: Arc<Queue<T>>,
    hazard1: HazardPointer<'a>,
    hazard2: HazardPointer<'a>,
}

impl<T> Clone for MSQueue<'_, T> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            hazard1: HazardPointer::new(),
            hazard2: HazardPointer::new(),
        }
    }
}

impl<T> SharedQueue<T> for MSQueue<'_, T>
where
    T: Clone + Copy + Send + Sync,
{
    fn new() -> Self {
        MSQueue {
            queue: Arc::new(Queue::new()),
            hazard1: HazardPointer::new(),
            hazard2: HazardPointer::new(),
        }
    }
    fn enqueue(&mut self, val: *const T) {
        self.queue.enqueue(val, &mut self.hazard1)
    }

    fn dequeue(&mut self) -> Option<*const T> {
        self.queue.dequeue(&mut self.hazard1, &mut self.hazard2)
    }
}

struct Node<T> {
    value: *const T,
    next: AtomicPtr<Node<T>>,
}
// Unsafe impls of send and sync, user is responsible that the raw pointer enqueued is actually
// valid
unsafe impl<T> Sync for Node<T> {}
unsafe impl<T> Send for Node<T> {}

impl<T> Node<T> {
    pub fn new(value: *const T) -> Node<T> {
        Node {
            value: value,
            next: unsafe { AtomicPtr::new(ptr::null_mut()) },
        }
    }
    fn empty() -> Node<T> {
        Node {
            value: ptr::null_mut(),
            next: unsafe { AtomicPtr::new(ptr::null_mut()) },
        }
    }
}
pub struct Queue<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
}

impl<T> Queue<T>
where
    T: Clone + Copy + Send + Sync,
{
    pub fn new() -> Queue<T> {
        let dummy = Box::into_raw(Box::new(Node::empty()));

        Queue {
            head: unsafe { AtomicPtr::new(dummy) },
            tail: unsafe { AtomicPtr::new(dummy) },
        }
    }

    pub fn enqueue(&self, value: *const T, hazp: &mut HazardPointer) {
        let node_ptr: AtomicPtr<Node<T>> = AtomicPtr::from(Box::new(Node::new(value)));
        let node_raw = node_ptr.load_ptr();

        loop {
            // Safety: Will always point to at least a dummy node
            let tail_node = self.tail.safe_load(hazp).unwrap();

            // Snapshot
            let tail_ptr: *const Node<T> = tail_node;

            let next_ptr = tail_node.next.load_ptr();

            // Check tail snapshot
            if tail_ptr != self.tail.load_ptr() {
                continue;
            }

            // Tail was not pointing to the last node
            if !next_ptr.is_null() {
                /* Try to swing tail "forward", i.e. to the "next" node,
                this will be done until the tail is corrected */
                let _ = unsafe {
                    self.tail
                        .compare_exchange_ptr(tail_ptr.cast_mut(), next_ptr)
                };
                continue;
            }

            // Try link node at the end of linked list
            match unsafe { tail_node.next.compare_exchange_ptr(next_ptr, node_raw) } {
                Ok(_) => {
                    // Try update tail to inserted node
                    let _ = unsafe {
                        self.tail
                            .compare_exchange_ptr(tail_ptr.cast_mut(), node_raw)
                    };
                    break;
                }
                Err(_) => continue,
            }
        }
    }

    pub fn dequeue(
        &self,
        hazp_head: &mut HazardPointer,
        hazp_next: &mut HazardPointer,
    ) -> Option<*const T> {
        loop {
            // Safety: Will always point to at least a dummy node
            let head_node = self.head.safe_load(hazp_head).unwrap();

            let head_ptr: *const Node<T> = head_node;
            let tail_ptr = self.tail.load_ptr();

            let next_node = head_node.next.safe_load(hazp_next);

            // Are head, tail, and next not consistent?
            if head_ptr != self.head.load_ptr() {
                continue;
            }

            // Empty queue
            if next_node.is_none() {
                return None;
            }
            let next_ptr: *const Node<T> = next_node.unwrap();

            // Is queue empty or Tail falling behind?
            if head_ptr == tail_ptr {
                // Tail is falling behind. Try to advance it
                let _ = unsafe {
                    self.tail
                        .compare_exchange_ptr(tail_ptr, next_ptr.cast_mut())
                };
                continue;
            }

            assert!(head_ptr != next_ptr);

            // Read value before CAS
            let val = next_node.unwrap().value;

            match unsafe {
                self.head
                    .compare_exchange_ptr(head_ptr.cast_mut(), next_ptr.cast_mut())
            } {
                Ok(Some(p)) => {
                    // The node is node dequeued, so we can retire the pointer
                    unsafe {
                        p.retire();
                    }
                    return Some(val);
                }
                Ok(None) => {
                    // This should not happen, as it would have required a null pointer to somehow make it to this point.
                    // Since this means a unrecoverable bug somewhere else we just panic
                    panic!(
                        "Somehow after a successful dequeue the pointer was null: Here be dragons"
                    )
                }
                Err(_) => continue,
            }
        }
    }
}

impl<T: Debug> Queue<T> {
    /// Debug function to print the queue's current state
    pub fn debug_print(&self) {
        unsafe {
            let mut current = self.head.load_ptr();

            // Check if the queue is empty
            if (*current).next.load_ptr().is_null() {
                println!("Queue is empty");
                return;
            }

            while !current.is_null() {
                println!(
                    "Value: {:?}, Pointer: {:?}",
                    (*current).value,
                    current as *const _
                );
                current = (*current).next.load_ptr();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Queue;
    use core::time;
    use haphazard::HazardPointer;
    use rand::Rng;
    use std::sync::Arc;
    use std::thread;

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
    fn basics() {
        let queue = Queue::new();
        let mut hazp = HazardPointer::new();
        let mut hazp2 = HazardPointer::new();

        // Populate queue
        queue.enqueue(&NUMBERS[0], &mut hazp);
        queue.enqueue(&NUMBERS[1] as *const _, &mut hazp);
        queue.enqueue(&NUMBERS[2] as *const _, &mut hazp);

        // Normal removal
        assert_eq!(unsafe { *queue.dequeue(&mut hazp, &mut hazp2).unwrap() }, 0);
        assert_eq!(unsafe { *queue.dequeue(&mut hazp, &mut hazp2).unwrap() }, 1);

        // Dequeue after dequeues
        queue.enqueue(&NUMBERS[3] as *const _, &mut hazp);
        queue.enqueue(&NUMBERS[4] as *const _, &mut hazp);

        // Normal removal to exhaustion
        assert_eq!(unsafe { *queue.dequeue(&mut hazp, &mut hazp2).unwrap() }, 2);
        assert_eq!(unsafe { *queue.dequeue(&mut hazp, &mut hazp2).unwrap() }, 3);
        assert_eq!(unsafe { *queue.dequeue(&mut hazp, &mut hazp2).unwrap() }, 4);
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), None);

        // Check the exhaustion case fixed the pointer right
        queue.enqueue(&NUMBERS[5] as *const _, &mut hazp);
        queue.enqueue(&NUMBERS[6] as *const _, &mut hazp);

        // Normal removal again
        assert_eq!(unsafe { *queue.dequeue(&mut hazp, &mut hazp2).unwrap() }, 5);
        assert_eq!(unsafe { *queue.dequeue(&mut hazp, &mut hazp2).unwrap() }, 6);
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), None);
    }

    #[test]
    fn basic_concurrent() {
        let queue = Arc::new(Queue::new());
        let mut handles = vec![];

        let n = 10;

        for i in 0..n {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                let mut hazp = HazardPointer::new();
                queue.enqueue(&NUMBERS[i], &mut hazp)
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let mut hazp = HazardPointer::new();
        let mut hazp2 = HazardPointer::new();
        let mut dequeue_sum = 0;
        while let Some(value) = queue.dequeue(&mut hazp, &mut hazp2) {
            dequeue_sum += unsafe {*value};
        }

        // Sum of first n natural numbers (0 to n-1)
        let expected_sum = n * (n - 1) / 2;

        assert_eq!(expected_sum, dequeue_sum.try_into().unwrap(), "Sums do not match!");
    }

    #[test]
    fn concurrent_dequeue_enqueue() {
        let queue = Arc::new(Queue::new());
        let mut handles = vec![];
        let mut rng = rand::thread_rng();

        let n = 10;

        for i in 0..n {
            // Random number to simulate "do other work" time
            let rt = rng.gen_range(50..150);
            let dur = time::Duration::from_nanos(rt);

            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                let mut hazp = HazardPointer::new();
                let mut hazp2 = HazardPointer::new();
                queue.enqueue(&NUMBERS[i], &mut hazp);
                thread::sleep(dur);
                let _v = queue.dequeue(&mut hazp, &mut hazp2).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should be empty
        let mut hazp = HazardPointer::new();
        let mut hazp2 = HazardPointer::new();
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), None);
    }
}
