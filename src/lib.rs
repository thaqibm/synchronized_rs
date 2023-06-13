use std::{cell::UnsafeCell, mem::MaybeUninit, ptr};

pub mod util;
pub mod arch;


#[macro_export]
macro_rules! synchronized {
    ($lock:ident, $b:block) => {
        {
            ($lock).acquire();
            $b;
            ($lock).release();
        }
    }
}


// TODO: Move this to another file

pub struct SyncCellUnsafe<T> {
    value: UnsafeCell<MaybeUninit<T>>
}

unsafe impl<T: Send> Send for SyncCellUnsafe<T> {}
unsafe impl<T: Send> Sync for SyncCellUnsafe<T> {}

impl<T> SyncCellUnsafe<T> {
    pub const fn new(val: T) -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::new(val)),
        }
    }
    pub fn store(&self, val: T) {
        unsafe {
          ptr::write(self.as_ptr(), val)
        } 
    }
    pub fn swap(&self, val: T) -> T {
        unsafe {
            ptr::replace(self.as_ptr(), val)
        }
    }
    
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get().cast::<T>()
    }

}

impl<T: Copy> SyncCellUnsafe<T> {
    pub fn load(&self) -> T {
        unsafe { ptr::read(self.as_ptr()) }
    }
}





