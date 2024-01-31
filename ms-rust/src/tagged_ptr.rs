use std::{fmt::Debug, marker::PhantomData, sync::atomic::{AtomicPtr, AtomicUsize}};
use std::sync::atomic::Ordering;


pub struct TaggedPointer<T> {
    ptr: AtomicUsize,
    _marker: PhantomData<T>
}

impl<T> Debug for TaggedPointer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaggedPointer").field("ptr", &self.ptr).finish()
    }
}

impl<T> Default for TaggedPointer<T> {
    fn default() -> Self {
        Self { ptr: AtomicUsize::new(0), _marker: PhantomData }
    }
}

impl<T> TaggedPointer<T> {

    pub fn new(ptr: *mut T, tag: u16) -> TaggedPointer<T> {
        let shifted_tag : usize = (tag as usize) << 48;
        let tagged_ptr = (ptr as usize) | shifted_tag;

        TaggedPointer {
            ptr: AtomicUsize::new(tagged_ptr),
            _marker: PhantomData
        }
    }

    pub fn is_null(&self) -> bool {
        self.load(Ordering::Relaxed) == 0
    }

    pub fn compare_exchange(
        &self, 
        current: usize, 
        new: usize, 
        success: Ordering, 
        failure: Ordering
    ) -> Result<usize, usize> {

        self.ptr.compare_exchange(current, new, success, failure) 
    }

    pub fn remove_tag(ptr: usize) -> *mut T {
        let mask : usize = 0x0000_FFFF_FFFF_FFFF;
        (ptr & mask) as *mut T
    }

    pub fn tag(ptr: usize) -> u16 {
        let shifted = ptr >> 48;
        let ret = shifted.try_into().unwrap();

        ret
    }

    pub fn set_tag(old: usize, tag: u16) -> usize {
        let shifted_one = 1 << 48;
        old + shifted_one
    }

    pub fn load(&self, ord: Ordering) -> usize {
        self.ptr.load(ord)
    }
}


// #[cfg(test)]
// mod test {
//     use std::sync::atomic::Ordering::{Relaxed};

//     use super::TaggedPointer;

//     #[test]
//     fn basics() {
//         let mut data = 100;
//         let ref1 = &mut data;
//         let ptr = ref1 as *mut i32;
//         let tag = 10;

//         let mut tagged_ptr = TaggedPointer::new(ptr, tag);

//         // Check tag is correct
//         assert_eq!(tag, tagged_ptr.tag(Relaxed));

//         // Check we get the original data
//         assert_eq!(data, unsafe{*tagged_ptr.deref(Relaxed)});
        
//         let mut test = unsafe {tagged_ptr.deref_mut(Relaxed)};
        
//         // Mutating through reference
//         *test = 50;
        
//         // Check mutation works
//         assert_eq!(50, unsafe{*tagged_ptr.deref(Relaxed)});
        
//     }


// }