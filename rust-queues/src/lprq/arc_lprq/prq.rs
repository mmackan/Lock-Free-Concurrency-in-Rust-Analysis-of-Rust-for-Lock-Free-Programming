use crossbeam_utils::CachePadded;
use std::{
    array,
    fmt::Debug,
    ptr::{self},
    sync::{
        atomic::{AtomicPtr, AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use aarc::AtomicArc;

// Make sure cells are on different cache lines
#[repr(align(128))]
struct Cell<T> {
    safe_and_epoch: AtomicUsize,
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

pub struct PRQ<T: 'static, const N: usize> {
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>, // Top bit here is set if the queue is closed
    array: [Cell<T>; N],
    pub next: AtomicArc<Self>,
}

impl<T, const N: usize> PRQ<T, N> {
    pub fn new() -> Self {
        PRQ {
            head: AtomicUsize::new(N).into(),
            array: array::from_fn(|_| Default::default()),
            tail: AtomicUsize::new(N).into(),
            next: AtomicArc::new(None),
        }
    }

    pub fn new_with_item(value_ptr: *const T) -> Self {
        let prq = PRQ {
            head: AtomicUsize::new(N).into(),
            tail: AtomicUsize::new(N).into(),
            array: array::from_fn(|_| Default::default()),
            next: AtomicArc::new(None),
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
        let thread_token = Cell::<T>::make_token(thread_id);
        loop {
            let tail_ticket: usize = self.tail.fetch_add(1, Ordering::SeqCst);
            let tail_val: usize = (!(1 << 63)) & tail_ticket;
            let closed = tail_ticket & (1 << 63) != 0;
            if closed {
                return Err(());
            }
            let cycle = tail_val / N;
            let index = tail_val % N;

            let cell = &self.array[index];

            let (safe, epoch) = cell.load_safe_and_epoch(Ordering::SeqCst);
            let value = cell.value.load(Ordering::SeqCst);

            if value.is_null()
                && epoch <= cycle
                && (safe || self.head.load(Ordering::SeqCst) <= cycle)
            {
                if let Ok(_) = cell.value.compare_exchange(
                    value,
                    thread_token,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    if let Ok(_) = cell.compare_exchange_safe_and_epoch(
                        (safe, epoch),
                        (true, cycle),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        if let Ok(_) = cell.value.compare_exchange(
                            thread_token,
                            value_ptr.cast_mut(),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            return Ok(());
                        }
                    } else {
                        let _ = cell.value.compare_exchange(
                            thread_token,
                            ptr::null_mut(),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        );
                    }
                }
            }

            // Check if the queue is full
            if tail_val >= self.head.load(Ordering::SeqCst) + N {
                // Set the top bit of the tail to indicate that the queue is closed
                self.tail.fetch_or(1 << 63, Ordering::SeqCst);
                return Err(());
            }
        }
    }

    pub fn dequeue(&self) -> Option<*mut T> {
        loop {
            let head_val = self.head.fetch_add(1, Ordering::SeqCst);
            let index = head_val % N;
            let cycle = head_val / N;
            let cell = &self.array[index];

            let mut r: u64 = 0;
            let mut tail = 0;
            let mut closed = false;
            loop {
                // Update cell state
                let (safe, epoch) = cell.load_safe_and_epoch(Ordering::SeqCst);
                let value = cell.value.load(Ordering::SeqCst);

                if epoch > head_val + N {
                    break;
                }

                if (!value.is_null()) && (!Cell::<T>::is_token(value.addr())) {
                    if epoch == cycle {
                        cell.value.store(ptr::null_mut(), Ordering::SeqCst);
                        return Some(value);
                    }
                    if !safe {
                        let new: (bool, usize) = cell.load_safe_and_epoch(Ordering::SeqCst);
                        if new == (safe, epoch) {
                            break;
                        }

                        if let Ok(_) = cell.compare_exchange_safe_and_epoch(
                            (safe, epoch),
                            (false, epoch),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            break;
                        }
                    }
                }
                if (r % 255) == 0 {
                    let tail_ticket = self.tail.load(Ordering::SeqCst);
                    tail = tail_ticket & (!(1 << 63));
                    closed = tail_ticket & (1 << 63) != 0;
                }

                if !safe || tail < head_val + 1 || closed || r > (4 * N).try_into().unwrap() {
                    if Cell::<T>::is_token(value.addr()) {
                        if let Ok(_) = cell.value.compare_exchange(
                            value,
                            ptr::null_mut(),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            continue;
                        }
                    }
                    if let Ok(_) = cell.compare_exchange_safe_and_epoch(
                        (safe, epoch),
                        (false, head_val),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        break;
                    }
                }
                r = r + 1;
            }
            // Is the queue empty?
            let tail_ticket = self.tail.load(Ordering::SeqCst);
            if ((!(1 << 63)) & tail_ticket) <= head_val + 1 {
                self.fix_state();
                return None;
            }
        }
    }
    fn fix_state(&self) {
        loop {
            let tail_ticket = self.tail.load(Ordering::SeqCst);
            let head = self.head.load(Ordering::SeqCst);
            if tail_ticket != self.tail.load(Ordering::SeqCst) {
                continue;
            }
            if head > ((!(1 << 63)) & tail_ticket) {
                if let Ok(_) = self.tail.compare_exchange(
                    tail_ticket,
                    head,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
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
        let tail_ticket = prq.tail.load(Ordering::SeqCst);
        assert_eq!(false, tail_ticket & (1 << 63) != 0);

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
