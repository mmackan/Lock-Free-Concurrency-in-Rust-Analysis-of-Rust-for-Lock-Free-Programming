
use std::{ptr, sync::atomic::{AtomicPtr, Ordering}};

#[derive(Copy)]
pub struct TaggedPointer<T> {
    data: *mut T
}

impl<T> Clone for TaggedPointer<T> {
    fn clone(&self) -> Self {
        Self { data: self.data.clone() }
    }
}

impl<T> PartialEq for TaggedPointer<T> {
    fn eq(&self, other: &Self) -> bool {
        ptr::addr_eq(self.data, other.data)
    }
}

impl<T> TaggedPointer<T>  {
    const TAG_MASK: usize = (0xFFFF << 48);
    const ADDR_MASK: usize = ! TaggedPointer::<T>::TAG_MASK;

    pub fn new(in_ptr: *mut T, tag: u16) -> Self {
        let shifted_tag = (tag as usize) << 48;
        let tagged = in_ptr.map_addr(|addr| (addr | shifted_tag));

        TaggedPointer {
            data: tagged
        }
    }

    pub fn ptr(&self) -> *mut T {
        self.data.map_addr(| tagged_addr | (tagged_addr & AtomicTagged::<T>::ADDR_MASK))
    }

    pub fn tag(&self) -> u16 {
        let tag = self.data.addr() >> 48;
        tag as u16
    }

    pub fn with_tag(&self, tag: u16) -> TaggedPointer<T> {
        Self::new(self.ptr(), tag)
    }
}

#[derive(Debug, Default)]
pub struct AtomicTagged<T> {
    data: AtomicPtr<T>,
}

impl<T> AtomicTagged<T> {
    const TAG_MASK: usize = (0xFFFF << 48);
    const ADDR_MASK: usize = ! AtomicTagged::<T>::TAG_MASK;

    pub fn new(in_ptr: *mut T, tag: u16) -> Self {
        let tagged = TaggedPointer::new(in_ptr, tag);

        AtomicTagged {
            data: AtomicPtr::new(tagged.data)
        }
    }

    pub fn load(&self, order: Ordering) -> TaggedPointer<T> {
        TaggedPointer { 
            data : self.data.load(order)
        } 
    }

    pub fn compare_exchange(
        &self, 
        current: &TaggedPointer<T>, 
        new: &TaggedPointer<T>, 
        success: Ordering, 
        failure: Ordering
    ) -> Result<TaggedPointer<T>, TaggedPointer<T>> {

        match self.data.compare_exchange(current.data, new.data, success, failure)  {
            Ok(previous) => Ok(TaggedPointer { data: previous }),
            Err(current) => Err(TaggedPointer { data: current }),
        }
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

        let tagged = AtomicTagged::new(p, 311);

        let p2 = tagged.load(std::sync::atomic::Ordering::Relaxed);

        assert_eq!(p, p2.ptr());
        unsafe {
            assert_eq!(*p2.ptr(), 42);
        }

        unsafe {
            p2.ptr().write(10);
            assert_eq!(*p2.ptr(), 10);
        }
        assert_eq!(p2.tag(), 311);
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