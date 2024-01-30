use crossbeam::epoch::{self, Atomic, Owned};
use std::sync::atomic::Ordering::{Relaxed};

struct Node {
    value: i32,
    next: Atomic<Node>
}

impl Node {
    pub fn new(value: i32) -> Node {
        Node {
            value,
            next: Atomic::null()
        }
    }
}
pub struct Queue {
    head: Atomic<Node>,
    tail: Atomic<Node> 
}

impl Queue {
    pub fn new() -> Queue {
        // SAFETY: dummy will never be modified, only read
        let dummy = unsafe { 
            Owned::new(Node::new(-1)).
            into_shared(epoch::unprotected()) 
        };
        
        Queue {
            head: Atomic::from(dummy),
            tail: Atomic::from(dummy)
        }
    }

    pub fn enqueue(&self, value: i32) {
        
        // Mark thread active in current epoch
        let guard = epoch::pin();
        
        let node = Owned::new(Node::new(value)).into_shared(&guard);

        loop {                                                                    
            // Snapshots
            let tail_ptr = self.tail.load(Relaxed, &guard);                        
            let next_ptr = unsafe{ &tail_ptr.deref() }.next.load(Relaxed, &guard); 
            
            // Tail snapshot still the queue's tail
            if tail_ptr == 
                self.tail.load(Relaxed, &guard) {         
                
                // Tail was pointing to the last node
                if next_ptr.is_null() {                                                
                    // Try link node at the end of linked list         
                    match unsafe{ tail_ptr.deref() }.next.compare_exchange(   
                        next_ptr, 
                        node.with_tag(next_ptr.tag() + 1), 
                        Relaxed, 
                        Relaxed, 
                        &guard
                    ) {
                        Ok(_) => {
                            // Try update tail to inserted node
                            let _ = self.tail.compare_exchange(             
                                tail_ptr,
                                node.with_tag(tail_ptr.tag() + 1),
                                Relaxed,
                                Relaxed, 
                                &guard);
                            break;                                          
                        },
                        Err(_) => continue
                    }
                }
                /* Try to swing tail "forward", i.e. to the "next" node, 
                 this will be done until the tail is corrected */
                let _ = self.tail.compare_exchange(                         
                    tail_ptr, 
                    next_ptr.with_tag(tail_ptr.tag() + 1), 
                    Relaxed, 
                    Relaxed, 
                    &guard);
            }         
        }
    }

    pub fn dequeue(&self) -> Option<i32> {

        // Mark thread active in current epoch
        let guard = epoch::pin();

        loop {
            // Snapshots
            let head_ptr = self.head.load(Relaxed, &guard);
            let tail_ptr = self.tail.load(Relaxed, &guard);
            let next_ptr = unsafe{ &head_ptr.deref() }.next.load(Relaxed, &guard); 

            let head_count = head_ptr.tag();
            let tail_count = tail_ptr.tag();

            // Are head, tail, and next consistent?
            if head_ptr == 
                self.head.load(Relaxed, &guard) {

                // Is queue empty or Tail falling behind?
                if head_ptr == tail_ptr {
                    
                    // Empty queue
                    if next_ptr.is_null() {
                        return None;
                    }
                    // Tail is falling behind. Try to advance it
                    let _ = self.tail.compare_exchange(
                        tail_ptr, 
                        next_ptr.with_tag(tail_count + 1), 
                        Relaxed, 
                        Relaxed, 
                        &guard);
                } else {

                    /* TODO(?): Read value before CAS, 
                    perhaps not relevant for Rust */
                    
                    match self.head.compare_exchange(
                        head_ptr, 
                        next_ptr.with_tag(head_count + 1),
                        Relaxed, 
                        Relaxed, 
                        &guard
                    ) {
                        Ok(prev) => {
                            let ret = unsafe{ prev.as_raw().read() };
                            return Some(ret.value);
                        },
                        Err(_) => continue
                    }

                }

            }
        }
    }

    pub fn print_queue(&self) {
        let guard = epoch::pin();
        let mut count = 1;
        let mut current = self.head.load(Relaxed, &guard);
        while !current.is_null() {
            let next = unsafe{current.as_raw().read().next};
            let val = unsafe{current.as_raw().read().value};

            if current == self.head.load(Relaxed, &guard) {
                print!("HEAD: ")
            }
            if current == self.tail.load(Relaxed, &guard) {
                print!("TAIL: ")
            }
            println!("Pointer: {:?}, Next: {:?}, Value: {}, count: {}", current, next, val, count);
            count += 1;
            let current_t = unsafe{current.as_raw().read().next.try_into_owned()};

            match current_t {
                Some(node) => current = node.into_shared(&guard),
                None => {
                    print!("\n");
                    break
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::thread;
    use super::Queue;

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
}