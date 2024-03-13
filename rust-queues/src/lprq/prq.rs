use crossbeam_utils::CachePadded;
use std::{
    array,
    fmt::Debug,
    ptr::{self, null_mut},
    sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering},
    thread,
};

use haphazard;

// Make sure cells are on different cache lines
#[repr(align(64))]
struct Cell<T> {
    safe_and_epoch: AtomicUsize,
    // NOTE: This is a std::sync AtomicPtr and not the haphazard one
    value: AtomicPtr<T>, // When the value is being written to this holds the thread token
}
impl<T> Default for Cell<T> {
    fn default() -> Self {
        Self {
            safe_and_epoch: Cell::<T>::SAFE_BIT_MASK.into(),
            value: Default::default(),
        }
    }
}
impl<T> Debug for Cell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (safe, epoch) = Self::sae_from_usize(self.safe_and_epoch.load(Ordering::Relaxed));
        f.debug_struct("Cell")
            .field("safe", &safe)
            .field("epoch", &epoch)
            .field("value", &self.value)
            .finish()
    }
}

impl<T> Cell<T> {
    const SAFE_BIT_MASK: usize = (1 << 63);
    const EPOCH_MASK: usize = !Cell::<T>::SAFE_BIT_MASK;

    const TOKEN_MASK: usize = (1 << 63);

    fn load_safe_and_epoch(&self, order: Ordering) -> (bool, usize) {
        let raw = self.safe_and_epoch.load(order);
        Self::sae_from_usize(raw)
    }

    fn compare_exchange_safe_and_epoch(
        &self,
        current: (bool, usize),
        new: (bool, usize),
        success: Ordering,
        failure: Ordering,
    ) -> Result<(bool, usize), (bool, usize)> {
        let current_packed = Self::usize_from_sae(current);
        let new_packed = Self::usize_from_sae(new);

        match self
            .safe_and_epoch
            .compare_exchange(current_packed, new_packed, success, failure)
        {
            Ok(new_packed) => Ok(Self::sae_from_usize(new_packed)),
            Err(old_packed) => Err(Self::sae_from_usize(old_packed)),
        }
    }

    fn make_token(thread_id: usize) -> *mut T {
        let tagged = thread_id & Self::TOKEN_MASK;
        ptr::invalid_mut(tagged)
    }

    // Block of utility functions for bitmasking that should all be inlined
    #[inline]
    fn is_token(ptr: usize) -> bool {
        let raw = (ptr & Self::TOKEN_MASK) >> 63;
        raw == 1
    }

    #[inline]
    fn sae_from_usize(raw: usize) -> (bool, usize) {
        ((raw & Self::SAFE_BIT_MASK) != 0, raw & Self::EPOCH_MASK)
    }
    #[inline]
    fn usize_from_sae((safe, epoch): (bool, usize)) -> usize {
        ((safe as usize) << 63) | epoch
    }
}

#[derive(Debug)]
pub struct PRQ<T, const N: usize> {
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
    closed: CachePadded<AtomicBool>,
    array: [Cell<T>; N],
    pub next: haphazard::AtomicPtr<PRQ<T, N>>,
    // In the reference this is stored as the top bit of tail
}

impl<T, const N: usize> PRQ<T, N> {
    pub fn new() -> Self {
        PRQ {
            closed: AtomicBool::new(false).into(),
            head: AtomicUsize::new(N).into(),
            array: array::from_fn(|_| Default::default()),
            tail: AtomicUsize::new(N).into(),
            next: unsafe { haphazard::AtomicPtr::new(null_mut()) },
        }
    }

    pub fn new_with_item(value_ptr: *const T) -> Self {
        let prq = PRQ {
            closed: AtomicBool::new(false).into(),
            head: AtomicUsize::new(N).into(),
            tail: AtomicUsize::new(N).into(),
            array: array::from_fn(|_| Default::default()),
            next: unsafe { haphazard::AtomicPtr::new(null_mut()) },
        };
        let _ = prq
            .enqueue(value_ptr)
            .expect("Failed to enqueue an item in a new and empty PRQ, Should not happen ever");
        return prq;
    }

    // Returns Ok() if enqueue was succesfull, Err() if the queue is closed
    pub fn enqueue(&self, value_ptr: *const T) -> Result<(), ()> {
        // Get a unique thread token
        let thread_id: usize = thread::current().id().as_u64().get().try_into().unwrap();
        loop {
            let tail_val: usize = self.tail.fetch_add(1, Ordering::SeqCst);
            if self.closed.load(Ordering::SeqCst) {
                return Err(());
            }
            let cycle = tail_val / N;
            let index = tail_val % N;

            let (safe, epoch) = self.array[index].load_safe_and_epoch(Ordering::SeqCst);
            let value = self.array[index].value.load(Ordering::SeqCst);

            if (value.is_null() || Cell::<T>::is_token(value.addr())) // Not occupied
                && epoch < cycle && (safe || self.head.load(Ordering::SeqCst) <= tail_val)
            {
                // Enqueue has not been overtaken

                // Lock the cell using our thread token, ptr::invalid_mut makes sure the thread token "pointer" does not have any provinance so it can not be dereferenced without MIRI complaining
                if let Ok(_) = self.array[index].value.compare_exchange(
                    value,
                    Cell::make_token(thread_id),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    // Advance the epoch
                    if let Ok(_) = self.array[index].compare_exchange_safe_and_epoch(
                        (safe, epoch),
                        (true, cycle),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        // Attempt to publish the value
                        if let Ok(_) = self.array[index].value.compare_exchange(
                            Cell::make_token(thread_id),
                            value_ptr.cast_mut(),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            return Ok(());
                        }
                    } else {
                        // Clean up if the safe and epoch CAS fails
                        let _ = self.array[index].value.compare_exchange(
                            Cell::make_token(thread_id),
                            ptr::null_mut(),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        );
                    }
                }
            }

            // Check if the queue is full
            if tail_val >= self.head.load(Ordering::SeqCst) + N {
                self.closed.store(true, Ordering::SeqCst);
                return Err(());
            }
        }
    }

    pub fn dequeue(&self) -> Option<*mut T> {
        loop {
            let head_val = self.head.fetch_add(1, Ordering::SeqCst);
            let cycle = head_val / N;
            let index = head_val % N;
            let cell = &self.array[index];
            loop {
                // Update cell state
                let (safe, epoch) = cell.load_safe_and_epoch(Ordering::SeqCst);
                let value = cell.value.load(Ordering::SeqCst);

                if epoch == cycle && (!value.is_null() || Cell::<T>::is_token(value.addr())) {
                    // Dequeue transition
                    cell.value.store(ptr::null_mut(), Ordering::SeqCst);
                    return Some(value);
                }
                if epoch <= cycle && (value.is_null() || Cell::<T>::is_token(value.addr())) {
                    // Empty transition
                    // Unlock the cell
                    if Cell::<T>::is_token(value.addr()) {
                        if let Err(_) = cell.value.compare_exchange(
                            value,
                            ptr::null_mut(),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            continue;
                        }
                    }
                    // Advance the epoch
                    if let Ok(_) = cell.compare_exchange_safe_and_epoch(
                        (safe, epoch),
                        (safe, cycle),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        break;
                    }
                    // If I understand the paper correctly this continue is implied by their wierd when block
                    continue;
                }
                if epoch < cycle && (!value.is_null() || Cell::<T>::is_token(value.addr())) {
                    // Unsafe transition
                    if let Ok(_) = cell.compare_exchange_safe_and_epoch(
                        (safe, epoch),
                        (false, epoch),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        break;
                    }
                    // Same deal here as in the empty transition
                    continue;
                }
                // epoch > cycle, deq is overtaken
                break;
            }
            // Is the queue empty?
            if self.tail.load(Ordering::SeqCst) <= head_val + 1 {
                self.fix_state();
                return None;
            }
        }
    }
    fn fix_state(&self) {
        loop {
            let tail = self.tail.load(Ordering::SeqCst);
            let head = self.head.load(Ordering::SeqCst);
            if tail != self.tail.load(Ordering::SeqCst) {
                continue;
            }
            if head > tail {
                if let Ok(_) = self.tail.compare_exchange(tail, head, Ordering::SeqCst, Ordering::SeqCst) {
                    break;
                }
                continue;
            }
            break;
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Cell, PRQ};
    use std::{
        sync::{atomic::Ordering, Arc},
        thread,
    };

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
            let item = Box::into_raw(Box::new(i));
            assert_eq!(prq.enqueue(item), Ok(()));
        }
        // PRQ is now full, should fail
        let item = Box::into_raw(Box::new(5));
        assert_eq!(prq.enqueue(item), Err(()));
        let _ = unsafe { Box::from_raw(item) };

        for i in 0..5 {
            let value = unsafe { Box::from_raw(prq.dequeue().unwrap()) };
            assert_eq!(value, Box::new(i));
        }
        assert_eq!(prq.dequeue(), None);
    }

    #[test]
    fn prq_concurrent() {
        const N: usize = 10;
        let prq: Arc<PRQ<usize, N>> = Arc::new(PRQ::new());

        let mut handles = vec![];

        for i in 0..N {
            let queue = Arc::clone(&prq);
            let handle = thread::spawn(move || {
                let v = Box::into_raw(Box::new(i));
                queue.enqueue(v)
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle
                .join()
                .expect("Enqueue that should have succeded failed");
        }

        let mut dequeue_sum = 0;
        while let Some(ptr) = prq.dequeue() {
            let value = unsafe { Box::from_raw(ptr) };
            dequeue_sum += *value;
        }

        // Sum of first n natural numbers (0 to n-1)
        let expected_sum = N * (N - 1) / 2;

        assert_eq!(expected_sum, dequeue_sum, "Sums do not match!");
    }
}
