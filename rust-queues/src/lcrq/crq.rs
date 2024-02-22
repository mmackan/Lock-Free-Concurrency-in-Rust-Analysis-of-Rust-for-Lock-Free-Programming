use std::{array, fmt::Debug, ptr::{self, null_mut}, sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering}, thread};

use haphazard;

// Make sure cells are on different cache lines
#[repr(align(64))]
struct Cell<T> {
    safe_and_epoch: AtomicUsize,
    // NOTE: This is a std::sync AtomicPtr and not the haphazard one
    value: AtomicPtr<T> // When the value is being written to this holds the thread token
}
impl<T> Default for Cell<T> {
    fn default() -> Self {
        Self { safe_and_epoch: Cell::<T>::SAFE_BIT_MASK.into(), value: Default::default() }
    }
}
impl<T> Debug for Cell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (safe, epoch) = Self::sae_from_usize(self.safe_and_epoch.load(Ordering::Relaxed));
        f.debug_struct("Cell")
            .field("safe", &safe)
            .field("epoch", &epoch)
            .field("value", &self.value).finish()
    }
}

impl<T> Cell<T> {
    const SAFE_BIT_MASK: usize = (1 << 63);
    const EPOCH_MASK: usize = !Cell::<T>::SAFE_BIT_MASK;

    fn load_safe_and_epoch(&self, order: Ordering) -> (bool, usize) {
        let raw = self.safe_and_epoch.load(order);
        Self::sae_from_usize(raw)
    }

    fn compare_exchange_safe_and_epoch(&self, current: (bool, usize), new: (bool, usize), success: Ordering, failure: Ordering) -> Result<(bool, usize), (bool, usize)> {
        let current_packed = Self::usize_from_sae(current);
        let new_packed = Self::usize_from_sae(new);

        match self.safe_and_epoch.compare_exchange(current_packed, new_packed, success, failure) {
            Ok(new_packed) => Ok(Self::sae_from_usize(new_packed)),
            Err(old_packed) => Err(Self::sae_from_usize(old_packed)),
        }
    }

    #[inline]
    fn sae_from_usize(raw: usize) -> (bool, usize) {
        ((raw & Self::SAFE_BIT_MASK) != 0, raw & Self::EPOCH_MASK)
    }
    #[inline]
    fn usize_from_sae((safe, epoch) : (bool, usize)) -> usize {
        ((safe as usize) << 63) | epoch
    }

}

#[derive(Debug)]
pub struct PRQ<T, const N: usize> {
    // Not sure if these two need to be on separate cache lines, TODO: benchmark and check
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
        let value_ptr = Box::into_raw(Box::new(val));
        // Get a unique thread token
        let thread_token: usize = thread::current().id().as_u64().get().try_into().unwrap();
        loop {
            let tail_val: usize = self.tail.fetch_add(1, Ordering::Relaxed).try_into().unwrap();
            if self.closed.load(Ordering::Relaxed) {
                // Drop the allocated value since it can no longer be enqueued
                let _ = unsafe {
                    Box::from_raw(value_ptr)
                };
                return Err(())
            }
            let cycle = tail_val / N;
            let index = tail_val % N;

            let (safe, epoch) = self.array[index].load_safe_and_epoch(Ordering::Relaxed);
            let value = self.array[index].value.load(Ordering::Relaxed);

            if (value.is_null() || value.addr() == thread_token) // Not occupied
                && epoch < cycle && (safe || self.head.load(Ordering::Relaxed) <= tail_val) { // Enqueue has not been overtaken
                
                // Lock the cell using our thread token, ptr::invalid_mut makes sure the thread token "pointer" does not have any provinance so it can not be dereferenced without MIRI complaining
                if let Ok(_) = self.array[index].value.compare_exchange(value, ptr::invalid_mut(thread_token), Ordering::Relaxed, Ordering::Relaxed) {
                    // Advance the epoch
                    if let Ok(_) = self.array[index].compare_exchange_safe_and_epoch((safe, epoch), (true, cycle), Ordering::Relaxed, Ordering::Relaxed) {
                        // Attempt to publish the value
                        if let Ok(_) = self.array[index].value.compare_exchange(ptr::invalid_mut(thread_token), value_ptr, Ordering::Relaxed, Ordering::Relaxed) {
                            return Ok(());
                        }
                    } else {
                        // Clean up if the safe and epoch CAS fails
                        let _ = self.array[index].value.compare_exchange(ptr::invalid_mut(thread_token), ptr::null_mut(), Ordering::Relaxed, Ordering::Relaxed);
                    }
                }
            }

            // Check if the queue is full
            if tail_val - self.head.load(Ordering::Relaxed) >= N.try_into().unwrap() {
                self.closed.store(true, Ordering::Relaxed);

                // Drop the allocated value since it can no longer be enqueued
                let _ = unsafe {
                    Box::from_raw(value_ptr)
                };
                return Err(())
            }
        }
    }

    fn deqeue(&self) -> Option<T> {
        let thread_token: usize = thread::current().id().as_u64().get().try_into().unwrap();
        loop {

            let head_val = self.head.fetch_add(1, Ordering::Relaxed);
            let cycle = head_val / N;
            let index = head_val % N;
            loop {
                // Update cell state
                let cell = &self.array[index];
                let (safe, epoch) = cell.load_safe_and_epoch(Ordering::Relaxed);
                let value = cell.value.load(Ordering::Relaxed);

                if (safe, epoch) != cell.load_safe_and_epoch(Ordering::Relaxed) {
                    // Cell snapshot is inconsistant, retry
                    continue;
                }

                if epoch == cycle && (!value.is_null() || value.addr() == thread_token) {
                    // Dequeue transition
                    cell.value.store(ptr::null_mut(), Ordering::Relaxed);
                    return Some(*unsafe {Box::from_raw(value)});
                }
                if epoch <= cycle && (value.is_null() || value.addr() == thread_token) {
                    // Empty transition
                    // Unlock the cell
                    if value.addr() == thread_token {
                        if let Err(_) = cell.value.compare_exchange(value, ptr::null_mut(), Ordering::Relaxed, Ordering::Relaxed) {
                            continue;
                        }
                    }
                    // Advance the epoch
                    if let Ok(_) = cell.compare_exchange_safe_and_epoch((safe, epoch), (safe, cycle), Ordering::Relaxed, Ordering::Relaxed) {
                        break;
                    }
                    // If I understand the paper correctly this continue is implied by their wierd when block
                    continue;
                }
                if epoch < cycle && (!value.is_null() || value.addr() == thread_token) {
                    // Unsafe transition
                    if let Ok(_) = cell.compare_exchange_safe_and_epoch((safe, epoch), (false, epoch), Ordering::Relaxed, Ordering::Relaxed) {
                        break;
                    }
                    // Same deal here as in the empty transition
                    continue;
                }
                // epoch > cycle, deq is overtaken
                break;
            }
            // Is the queue empty?
            if self.tail.load(Ordering::Relaxed) <= head_val + 1 {
                return None;
            }
        }

    }
}

impl<T, const N: usize> Drop for PRQ<T, N> {
    fn drop(&mut self) {
        for cell in &self.array {
            let val = cell.value.load(Ordering::Relaxed);
            if !val.is_null() {
                // Since val is not null, we have a non dequeued value that needs to be cleaned up
                let _ = unsafe {Box::from_raw(val)};
            }
        }
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
        let prq: PRQ<i32, 5> = PRQ::new();
        assert!(!prq.closed.load(Ordering::Relaxed));

        for i in 0..5 {
            assert_eq!(prq.enqueue(i), Ok(()));
        }
        // PRQ is now full, should fail
        assert_eq!(prq.enqueue(5), Err(()));

        for i in 0..5 {
            assert_eq!(prq.deqeue(), Some(i));
        }
        assert_eq!(prq.deqeue(), None);
    }
}