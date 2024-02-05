#![feature(strict_provenance)]
#![warn(fuzzy_provenance_casts)]
use std::{default, sync::atomic::Ordering::{Relaxed, SeqCst}};
mod atomic_tagged;
use atomic_tagged::{AtomicTagged, TaggedPointer};

#[derive(Default)]
struct Node {
    value: i32,
    next: AtomicTagged<Node>
}

impl Node {
    pub fn new(value: i32) -> Node {
        Node {
            value,
            next: AtomicTagged::default()
        }
    }
}
pub struct Queue {
    head: AtomicTagged<Node>,
    tail: AtomicTagged<Node>,
    first: AtomicTagged<Node>
}

impl Drop for Queue {
    fn drop(&mut self) {
        // Iterate through the list and free any remaining nodes
        let mut current = self.first.load(Relaxed);
        while !current.ptr().is_null() {
            let node = unsafe {
                Box::from_raw(current.ptr())
            };
            current = node.next.load(Relaxed);
        }
    }
}

impl Queue {
    pub fn new() -> Queue {
        let dummy = Box::into_raw(Box::new(Node::new(-1)));
        
        Queue {
            head: AtomicTagged::new(dummy, 0),
            tail: AtomicTagged::new(dummy, 0),
            first: AtomicTagged::new(dummy, 0)
        }
    }

    pub fn enqueue(&self, value: i32) {
        
        let node_ptr = Box::into_raw(Box::new(Node::new(value)));

        loop {                                                                    
            // Snapshot
            let tagged_tail = self.tail.load(SeqCst);
            
            let tagged_next = unsafe {
                &*tagged_tail.ptr()
            }.next.load(Relaxed);
            
            // Check tail snapshot is still the queue's tail
            if tagged_tail == self.tail.load(SeqCst) {         
                
                // Tail was pointing to the last node
                if tagged_next.ptr().is_null() {                                                
                    // Try link node at the end of linked list         
                    match unsafe{ &*tagged_tail.ptr() }.next.compare_exchange(   
                        &tagged_next, 
                        &TaggedPointer::new(node_ptr, tagged_next.tag() + 1),
                        SeqCst, 
                        Relaxed, 
                    ) {
                        Ok(_) => {
                            // Try update tail to inserted node
                            let _ = self.tail.compare_exchange(             
                                &tagged_tail,
                                &TaggedPointer::new(node_ptr, tagged_tail.tag() + 1),
                                SeqCst,
                                Relaxed, 
                        );
                            break;                                          
                        },
                        Err(_) => continue
                    }
                }
                /* Try to swing tail "forward", i.e. to the "next" node, 
                 this will be done until the tail is corrected */
                let _ = self.tail.compare_exchange(                         
                    &tagged_tail,
                    &TaggedPointer::new(tagged_next.ptr(), tagged_tail.tag() + 1),
                    SeqCst, 
                    Relaxed, 
            );
            }         
        }
    }

    pub fn dequeue(&self) -> Option<i32> {
        loop {
            // Snapshots
            let tagged_head = self.head.load(SeqCst);
            let tagged_tail = self.tail.load(SeqCst);

            let tagged_next = unsafe{ 
                &*tagged_head.ptr()
            }.next.load(SeqCst); 

            // Are head, tail, and next consistent?
            if tagged_head == self.head.load(SeqCst) {

                // Is queue empty or Tail falling behind?
                if tagged_head.ptr() == tagged_tail.ptr() {
                    
                    // Empty queue
                    if tagged_next.ptr().is_null() {
                        return None;
                    }
                    // Tail is falling behind. Try to advance it
                    let _ = self.tail.compare_exchange(
                        &tagged_tail, 
                        &TaggedPointer::new(tagged_next.ptr(), tagged_tail.tag() + 1),
                        SeqCst, 
                        Relaxed, 
                );
                } else {

                    // Read value before CAS
                    let dequeued_value = unsafe{
                        (*tagged_next.ptr()).value
                    };
                    
                    match self.head.compare_exchange(
                        &tagged_head,
                        &TaggedPointer::new(tagged_next.ptr(), tagged_head.tag() + 1),
                        SeqCst, 
                        Relaxed, 
                    ) {
                        Ok(_) => {
                            return Some(dequeued_value);
                        },
                        Err(_) => continue
                    }

                }

            }
        }
    }
}

fn main() {

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