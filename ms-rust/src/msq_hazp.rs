use std::ptr;
use haphazard::{AtomicPtr, HazardPointer};

struct Node {
    value: i32,
    next: AtomicPtr<Node>
}

impl Node {
    pub fn new(value: i32) -> Node {
        Node {
            value,
            next: unsafe {
                AtomicPtr::new(ptr::null_mut())
            }
        }
    }
}
pub struct Queue {
    head: AtomicPtr<Node>,
    tail: AtomicPtr<Node>,
}

impl Queue {
    pub fn new() -> Queue {
        let dummy = Box::into_raw(Box::new(Node::new(-1)));
        
        Queue {
            head: unsafe {
                AtomicPtr::new(dummy)
            },
            tail: unsafe {
                AtomicPtr::new(dummy)
            },
        }
    }

    pub fn enqueue(&self, value: i32, hazp: &mut HazardPointer) {
        
        let node_ptr: AtomicPtr<Node> = AtomicPtr::from(Box::new(Node::new(value)));

        loop {                                                                    
            // Safety: Will always point to at least a dummy node
            let tail_node = self.tail.safe_load(hazp).unwrap();
            
            // Snapshot
            let tail_ptr = self.tail.load_ptr();

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
                    self.tail.compare_exchange_ptr(tail_ptr, next_ptr)
                };                                    
            }

            // Try link node at the end of linked list    
            match unsafe {
                tail_node.next.compare_exchange_ptr(next_ptr, node_ptr.load_ptr())
            } {
                Ok(_) => {

                    // Try update tail to inserted node
                    let _ = unsafe {
                        self.tail.compare_exchange_ptr(tail_ptr, node_ptr.load_ptr())
                    };
                    break;                                          
                },
                Err(_) => {
                    continue
                }
            }
        }
    }

    pub fn dequeue(&self, hazp_head: &mut HazardPointer, hazp_next: &mut HazardPointer) -> Option<i32> {
        
        loop {
            // Safety: Will always point to at least a dummy node
            let head_node = self.head.safe_load(hazp_head).unwrap();

            let head_ptr = self.head.load_ptr();
            let tail_ptr = self.tail.load_ptr();
            
            let next_node = head_node.next.safe_load(hazp_next);
            let next_ptr = head_node.next.load_ptr();

            // Are head, tail, and next not consistent?
            if head_ptr != self.head.load_ptr() {
                continue;
            }
            
            // Empty queue
            if next_node.is_none() {
                return None
            }

            // Is queue empty or Tail falling behind?
            if head_ptr == tail_ptr {                 
                
                // Tail is falling behind. Try to advance it
                let _ = unsafe {
                    self.tail.compare_exchange_ptr(tail_ptr, next_ptr)
                };
                continue;
            }

            // Read value before CAS
            let val = next_node.unwrap().value;

            match unsafe {
                self.head.compare_exchange_ptr(head_ptr, next_ptr)
            } {
                Ok(Some(p)) => {
                    // The node is node dequeued, so we can retire the pointer
                    unsafe {
                        p.retire();
                    }
                    return Some(val);
                },
                Ok(None) => {
                    // This should not happen, as it would have required a null pointer to somehow make it to this point.
                    // Since this means a unrecoverable bug somewhere else we just panic
                    panic!("Somehow after a successful dequeue the pointer was null: Here be dragons")
                }
                Err(_) => continue
            }
        }
    }

    /// Debug function to print the queue's current state
    pub fn debug_print(&self) {
        unsafe {
            let mut current = self.head.load_ptr();

            // Check if the queue is empty
            if (*current).next.load_ptr().is_null() {
                println!("Queue is empty");
                return;
            }

            // Skip dummy node
            // current = (*current).next.load_ptr();

            while !current.is_null() {
                println!("Value: {}, Pointer: {:?}", (*current).value, current as *const _);
                current = (*current).next.load_ptr();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use core::time;
    use std::sync::Arc;
    use std::thread;
    use super::Queue;
    use haphazard::HazardPointer;
    use rand::Rng;

    #[test]
    fn basics() {
        let queue = Queue::new();
        let mut hazp = HazardPointer::new();
        let mut hazp2 = HazardPointer::new();

        // Populate list
        queue.enqueue(1, &mut hazp);
        queue.enqueue(2, &mut hazp);
        queue.enqueue(3, &mut hazp);
                
        // Normal removal
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), Some(1));
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), Some(2));

        // Dequeue after dequeues
        queue.enqueue(4, &mut hazp);
        queue.enqueue(5, &mut hazp);

        // Normal removal to exhaustion
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), Some(3));
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), Some(4));
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), Some(5));
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), None);

        // Check the exhaustion case fixed the pointer right
        queue.enqueue(6, &mut hazp);
        queue.enqueue(7, &mut hazp);

        // Normal removal again
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), Some(6));
        assert_eq!(queue.dequeue(&mut hazp, &mut hazp2), Some(7));
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
                queue.enqueue(i, &mut hazp)
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
            dequeue_sum += value;
        }

        // Sum of first n natural numbers (0 to n-1)
        let expected_sum = n * (n - 1) / 2;

        assert_eq!(expected_sum, dequeue_sum, "Sums do not match!");
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
                queue.enqueue(i, &mut hazp);
                thread::sleep(dur);
                queue.dequeue(&mut hazp, &mut hazp2);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should be empty
        // assert_eq!(queue.dequeue(), None);
    }
}