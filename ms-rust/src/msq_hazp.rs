use std::ptr;
use haphazard::{AtomicPtr, Domain, HazardPointer};

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

    pub fn enqueue(&self, value: i32) {
        
        let node_ptr: AtomicPtr<Node> = AtomicPtr::from(Box::new(Node::new(value)));
        let mut hazp_tail = HazardPointer::new();

        loop {                                                                    
            // Snapshot
            let tail = &self.tail;
            let tail_ptr = tail.load_ptr();

            // Safety: Will always point to at least a dummy node
            let tail_node = tail.safe_load(&mut hazp_tail).unwrap();

            let next = &tail_node.next;
            let next_ptr = next.load_ptr();
            
            // Check tail snapshot is still the queue's tail    
            if tail_ptr == self.tail.load_ptr() {     
                
                // Tail was pointing to the last node
                if next_ptr.is_null() {
                                                          
                    // Try link node at the end of linked list    
                    match unsafe {
                        (*tail_ptr).next.compare_exchange_ptr(next_ptr, node_ptr.load_ptr())
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

                /* Try to swing tail "forward", i.e. to the "next" node, 
                 this will be done until the tail is corrected */
                let _ = unsafe {
                    self.tail.compare_exchange_ptr(tail_ptr, next_ptr)
                };
            }         
        }
    }

    pub fn dequeue(&self) -> Option<i32> {
        
        let mut hazp_head = HazardPointer::new();
        let mut hazp_next = HazardPointer::new();

        loop {

            let head_ptr = self.head.load_ptr();
            let tail_ptr = self.tail.load_ptr();
            
            // Safety: Will always point to at least a dummy node
            let head_node = self.head.safe_load(&mut hazp_head).unwrap();
            
            let next = &head_node.next;
            let next_ptr = next.load_ptr();

            // Are head, tail, and next consistent?
            if head_ptr == self.head.load_ptr() {
    
                // Is queue empty or Tail falling behind?
                if head_ptr == tail_ptr {                 
                    
                    // Empty queue
                    if next_ptr.is_null() {
                        return None;
                    }

                    // Tail is falling behind. Try to advance it
                    let _ = unsafe {
                        self.tail.compare_exchange_ptr(tail_ptr, next_ptr)
                    };
                } else {
            
                    // Safety: At this point, next can't be null
                    let next_node = next.safe_load(&mut hazp_next).unwrap();

                    // Read value before CAS
                    let val = next_node.value;

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
    use rand::Rng;

    #[test]
    fn basics() {
        let queue = Queue::new();

        // Populate list
        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);
                
        // Normal removal
        assert_eq!(queue.dequeue(), Some(1));
        assert_eq!(queue.dequeue(), Some(2));

        // Dequeue after dequeues
        queue.enqueue(4);
        queue.enqueue(5);

        // Normal removal to exhaustion
        assert_eq!(queue.dequeue(), Some(3));
        assert_eq!(queue.dequeue(), Some(4));
        assert_eq!(queue.dequeue(), Some(5));
        assert_eq!(queue.dequeue(), None);

        // Check the exhaustion case fixed the pointer right
        queue.enqueue(6);
        queue.enqueue(7);

        // Normal removal again
        assert_eq!(queue.dequeue(), Some(6));
        assert_eq!(queue.dequeue(), Some(7));
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn basic_concurrent() {
        let queue = Arc::new(Queue::new());
        let mut handles = vec![];

        let n = 10;

        for i in 0..n {
            let queue = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                queue.enqueue(i)
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let mut dequeue_sum = 0;
        while let Some(value) = queue.dequeue() {
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
                queue.enqueue(i);
                thread::sleep(dur);
                queue.dequeue();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should be empty
        assert_eq!(queue.dequeue(), None);
    }
}