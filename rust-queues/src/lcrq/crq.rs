use std::{array, ptr::null_mut, sync::atomic::{AtomicBool, AtomicU64, Ordering}};

use haphazard::AtomicPtr;

// Make sure cells are on different cache lines
#[repr(align(64))]
struct Cell<T> {
    safe_and_epoch: AtomicU64,
    value: Option<T>
}
impl<T> Default for Cell<T> {
    fn default() -> Self {
        Self { safe_and_epoch: Cell::<T>::SAFE_BIT_MASK.into(), value: None }
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

pub struct CRQ<T, const N: usize> {
    closed: AtomicBool,
    head: AtomicU64,
    array: [Cell<T>; N],
    //Tail placed below the array to make (very) sure head and tail are on different cache lines
    tail: AtomicU64,
    next: AtomicPtr<CRQ<T, N>>
}

impl<T: Copy, const N: usize> CRQ<T, N> {
    fn new() -> Self {
        CRQ { 
            closed: false.into(), 
            head: (N as u64).into(), 
            array: array::from_fn(|_| Default::default()), 
            tail: (N as u64).into(), 
            next: unsafe {AtomicPtr::new(null_mut())} 
        }
    }
}

#[cfg(test)]
mod test {
    use super::Cell;

    #[test]
    fn basic_cell() {
        let cell: Cell<i32> = Cell::default();
        let (safe, epoch) = cell.load_safe_and_epoch(std::sync::atomic::Ordering::Relaxed);
        assert!(safe);
        assert_eq!(epoch, 0)
    }
}