#![feature(strict_provenance)]
#![warn(fuzzy_provenance_casts)]

use std::{sync::atomic::AtomicPtr, sync::{atomic::{Ordering::Relaxed, fence}, Arc}, thread};
mod experimental_msq;
mod atomic_tagged;

struct Node {
    value: u64,
    next: AtomicPtr<Node>
}
impl Node {
    fn new_raw(value: u64) -> AtomicPtr<Node> {
        let nullptr: AtomicPtr<Node> = Default::default();
        let node = Box::new(Node {
            value : value,
            next : nullptr,
        });
        AtomicPtr::new(Box::into_raw(node))
    }
}

struct Queue {
    head: AtomicPtr<Node>,
    tail: AtomicPtr<Node>
}

impl Queue {
    fn new() -> Self {
        let node = Node::new_raw(0);
        Self {
            head : AtomicPtr::new(node.load(Relaxed)),
            tail: AtomicPtr::new(node.load(Relaxed))
        }
    }

    fn print_queue(&self) {
        let mut count = 1;
        let mut current = self.head.load(Relaxed);
        while !current.is_null() {
            let next = unsafe {
                (*current).next.load(Relaxed)
            };
            let val = unsafe {
                (*current).value
            };
            println!("Pointer: {:?}, Next: {:?}, Value: {}, count: {}", current, next, val, count);
            count += 1;
            current =  unsafe {
                (*current).next.load(Relaxed)
            };
        }
    }

    fn enqueue(&self, value: u64) {
        let new_node = Node::new_raw(value).into_inner();
        loop {
            let tail_ptr = self.tail.load(Relaxed);
            let next_ptr = unsafe {
                (*tail_ptr).next.load(Relaxed)
            };
            
            if tail_ptr == self.tail.load(Relaxed) {
                if next_ptr.is_null() {
                    let res = unsafe {
                        (*tail_ptr).next.compare_exchange(next_ptr, new_node, Relaxed, Relaxed)
                    };
                    match res {
                        Ok(_) => {
                            let _ = self.tail.compare_exchange(tail_ptr, new_node, Relaxed, Relaxed);
                            break;
                        },
                        Err(_) => continue,
                    }
                } else {
                    let _ = self.tail.compare_exchange(tail_ptr, next_ptr, Relaxed, Relaxed);
                }
            }
        }
    }

    fn dequeue(&self) -> Option<u64> {
        loop {
            let head_ptr = self.head.load(Relaxed);
            let tail_ptr = self.tail.load(Relaxed);
            let next_ptr = unsafe {
                (*head_ptr).next.load(Relaxed)
            };

            if head_ptr == self.head.load(Relaxed) {
                if head_ptr == tail_ptr {
                    if next_ptr.is_null() {
                        return None;
                    }
                    let _ = self.tail.compare_exchange(tail_ptr, next_ptr, Relaxed, Relaxed);

                } else {
                    let res = self.head.compare_exchange(head_ptr, next_ptr, Relaxed, Relaxed);
                    match res {
                        Ok(previous_head) => {
                            let ret = unsafe { *Box::from_raw(previous_head) };    
                            return Some(ret.value);
                        }
                        Err(_) => continue
                    }
                }
            } 
        }
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        // Iterate through the list and free any remaining nodes
        let mut current = self.head.load(Relaxed);
        while !current.is_null() {
            let node = unsafe {
                Box::from_raw(current)
            };
            current = node.next.load(Relaxed);
        }
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    let queue = Arc::new(Queue::default());

    println!("{:?}", queue.tail);

    let mut handles = vec![];

    for i in 0..4 {
        let q_ref = queue.clone();
        handles.push(thread::spawn(move || {
            for j in 0..9 {
                q_ref.enqueue(i*10 + j);
            }
        }));
    }

    queue.print_queue();

    for handle in handles {
        let _ = handle.join();
    }
}

#[cfg(test)]
mod test {
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
}
