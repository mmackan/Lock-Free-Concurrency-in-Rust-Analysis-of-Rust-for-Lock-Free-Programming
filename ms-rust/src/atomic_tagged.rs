
use std::sync::atomic::{AtomicPtr, Ordering};


#[derive(Debug, Default)]
pub struct AtomicTagged<T> {
    data: AtomicPtr<T>,
}

impl<T> AtomicTagged<T> {
    const TAG_MASK: usize = (0xFF << 48);
    const ADDR_MASK: usize = ! AtomicTagged::<T>::TAG_MASK;

    pub fn new(in_ptr: *mut T, tag: u16) -> Self {
        let shifted_tag = (tag as usize) << 48;
        let tagged = in_ptr.map_addr(|addr| (addr | shifted_tag));

        AtomicTagged {
            data: AtomicPtr::new(tagged)
        }
    }

    pub fn load(&self, order: Ordering) -> *mut T {
        let tagged = self.data.load(order);
        tagged.map_addr(| tagged_addr | (tagged_addr & AtomicTagged::<T>::ADDR_MASK))
    }

    pub fn load_tag(&self, order: Ordering) -> u16 {
        let tagged = self.data.load(order);
        let tag = tagged.addr() >> 48;
        tag as u16
    }
}


#[cfg(test)]
mod test {
    use std::alloc::{alloc, dealloc, Layout};

    use super::AtomicTagged;

    #[test]
    fn basic() {
        let mut target : usize = 42;

        let p = &mut target as *mut usize;

        let tagged = AtomicTagged::new(p, 2);

        let p2 = tagged.load(std::sync::atomic::Ordering::Relaxed);

        assert_eq!(p, p2);
        unsafe {
            assert_eq!(*p2, 42);
        }

        unsafe {
            p2.write(10);
            assert_eq!(*p2, 10);
        }

        let p2_tag = tagged.load_tag(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(p2_tag, 2);
    }

    #[test]
    fn manual_alloc() {
        let layout = Layout::new::<u32>();
        let ptr = unsafe { alloc(layout) as *mut u32 };

        assert!(!ptr.is_null());

        unsafe {
            ptr.write(123);
            assert_eq!(*ptr, 123);
        }

        let tagged = AtomicTagged::new(ptr, 42); 

        let p2 = tagged.load(std::sync::atomic::Ordering::Relaxed);



        // Free the memory
        unsafe {
            dealloc(ptr as *mut u8, layout);
        }
    }

}