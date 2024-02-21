use std::{array, ptr::null_mut, sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, AtomicUsize, Ordering}};

use haphazard;

// Make sure cells are on different cache lines
#[repr(align(64))]
struct Cell<T> {
    safe_and_epoch: AtomicU64,
    // NOTE: This is a std::sync AtomicPtr and not the haphazard one
    value: AtomicPtr<T> // When the value is being written to this holds the thread token
}
impl<T> Default for Cell<T> {
    fn default() -> Self {
        Self { safe_and_epoch: Cell::<T>::SAFE_BIT_MASK.into(), value: Default::default() }
    }
}

impl<T> Cell<T> {
    const SAFE_BIT_MASK: u64 = (1 << 63);
    const EPOCH_MASK: u64 = !Cell::<T>::SAFE_BIT_MASK;
    fn load_safe_and_epoch(&self, order: Ordering) -> (bool, u64) {
        let raw = self.safe_and_epoch.load(order);
        ((raw & Cell::<T>::SAFE_BIT_MASK) != 0, raw & Cell::<T>::EPOCH_MASK)
    }
}

pub struct PRQ<T, const N: usize> {
    closed: AtomicBool,
    head: AtomicUsize,
    array: [Cell<T>; N],
    //Tail placed below the array to make (very) sure head and tail are on different cache lines
    tail: AtomicUsize,
    next: haphazard::AtomicPtr<PRQ<T, N>>,

}

impl<T,const N: usize> PRQ<T, N> {
    fn new() -> Self {
        PRQ { 
            closed: false.into(), 
            head: N.into(), 
            array: array::from_fn(|_| Default::default()), 
            tail: N.into(), 
            next: unsafe {haphazard::AtomicPtr::new(null_mut())} 
        }
    }

    // Returns Ok() if enqueue was succesfull, Err() if the queue is closed
    fn enqueue(&self, val: T) -> Result<(), ()> {
        loop {
            let tail_val: usize = self.tail.fetch_add(1, Ordering::Relaxed).try_into().unwrap();
            // Labeled block to allow breaking to the check step without a function call
            'main_body: {
                if self.closed.load(Ordering::Relaxed) {
                    return Err(())
                }
                let cycle = tail_val / N;
                let index = tail_val % N;

                let (safe, epoch) = self.array[index].load_safe_and_epoch(Ordering::Relaxed);


            }

            // Check if the queue is full
            if tail_val - self.head.load(Ordering::Relaxed) >= N.try_into().unwrap() {

            }
        }
    }

    fn deqeue(&self) -> Option<T> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::{Cell, PRQ};
    use std::sync::atomic::Ordering;

    #[test]
    fn basic_cell() {
        let cell: Cell<i32> = Cell::default();
        let (safe, epoch) = cell.load_safe_and_epoch(Ordering::Relaxed);
        assert!(safe);
        assert_eq!(epoch, 0)
    }
    #[test]
    fn basic_prq() {
        let prq: PRQ<i32, 64> = PRQ::new();
        assert!(!prq.closed.load(Ordering::Relaxed))
    }
}