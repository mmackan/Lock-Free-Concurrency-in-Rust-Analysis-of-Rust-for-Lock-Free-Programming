use std::{sync::atomic::AtomicPtr, sync::{atomic::Ordering::Relaxed, Arc}};


struct Node {
    value: u64,
    next: AtomicPtr<Node>
}
impl Node {
    fn new_raw(value: u64) -> *mut Node {
        Box::into_raw(Box::new(Node {
            value : value,
            next : Default::default(),
        }))
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
            head : AtomicPtr::new(node),
            tail: AtomicPtr::new(node)
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
        let new_node = Node::new_raw(value);
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
                    let _ = self.tail.compare_exchange(tail_ptr, new_node, Relaxed, Relaxed);
                }
            }
        }
    }

    fn dequeue(self) -> u64 {
        todo!()
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

    for i in 1..10 {
        queue.enqueue(i);
    }
    queue.print_queue();
}
