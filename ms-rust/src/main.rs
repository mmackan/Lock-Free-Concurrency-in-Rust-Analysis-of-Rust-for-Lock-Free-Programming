use std::{sync::atomic::AtomicPtr, sync::atomic::Ordering::Relaxed};


struct Node {
    value: u64,
    next: AtomicPtr<Node>
}
impl Node {
    fn new_raw(value: u64) -> *mut Node {
        Box::into_raw(Box::new(Node {
            value : 0,
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
        println!("Sanity");
        let mut count = 1;
        let mut current = self.head.load(Relaxed);
        while !current.is_null() {
            println!("Pointer: {:?}, count: {}", current, count);
            count += 1;
            current =  unsafe {
                (*current).next.load(Relaxed)
            };
        }
    }

    fn enqueue(&self, value: u64) {
        loop {
            let tail_ptr = self.tail.load(Relaxed);
            let next_ptr = unsafe {
                (*tail_ptr).next.load(Relaxed)
            };
            
            if tail_ptr == self.tail.load(Relaxed) {
                if next_ptr.is_null() {
                    let res = unsafe {
                        (*tail_ptr).next.compare_exchange(next_ptr, Node::new_raw(value), Relaxed, Relaxed)
                    };
                    match res {
                        Ok(_) => {
                            let _ = self.tail.compare_exchange(tail_ptr, next_ptr, Relaxed, Relaxed);
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

    fn dequeue(self) -> u64 {
        todo!()
    }
}

fn main() {
    let queue = Queue::new();

    println!("{:?}", queue.tail);

    for i in 1..10 {
        queue.enqueue(i);
    }
    queue.print_queue();
}
