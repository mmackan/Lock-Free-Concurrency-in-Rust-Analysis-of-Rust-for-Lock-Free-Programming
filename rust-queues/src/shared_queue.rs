

// Trait for a single queue that can be shared between threads
pub trait SharedQueue<T> {
    fn new() -> Self;
    fn enqueue(&mut self, val: T);
    fn dequeue(&mut self) -> Option<T>;
}
