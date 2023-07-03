use std::{
    cell::UnsafeCell,
    ops::Deref,
    ptr::{self, NonNull},
};

pub mod arch;
pub mod containers;
pub mod util;

#[macro_export]
macro_rules! synchronized {
    ($lock:ident, $b:block) => {{
        ($lock).acquire();
        $b;
        ($lock).release();
    }};
}

// TODO: Move this to another file

#[derive(Debug)]
struct SPtrInner<T: ?Sized> {
    data: UnsafeCell<T>,
}
unsafe impl<T: ?Sized + Send> Send for SPtrInner<T> {}
unsafe impl<T: ?Sized + Send> Sync for SPtrInner<T> {}

#[derive(Debug, Clone)]
pub struct SPtr<T: ?Sized> {
    inner: NonNull<SPtrInner<T>>,
}

unsafe impl<T: ?Sized + Send> Send for SPtr<T> {}
unsafe impl<T: ?Sized + Send> Sync for SPtr<T> {}

impl<T: ?Sized> SPtr<T> {
    unsafe fn from_inner(inner: NonNull<SPtrInner<T>>) -> Self {
        Self { inner }
    }

    unsafe fn from_ptr(ptr: *mut SPtrInner<T>) -> Self {
        Self::from_inner(NonNull::new_unchecked(ptr))
    }
    #[inline]
    fn inner(&self) -> &SPtrInner<T> {
        unsafe { self.inner.as_ref() }
    }
}

impl<T> SPtr<T> {
    pub fn new(val: T) -> Self {
        let x = Box::new(SPtrInner {
            data: UnsafeCell::new(val),
        });
        unsafe { Self::from_ptr(Box::into_raw(x)) }
    }

    pub fn store(&self, val: T) {
        unsafe {
            let x = self.inner().data.get();
            std::ptr::write(x, val);
        }
    }
    pub fn swap(&self, val: T) -> T {
        unsafe { ptr::replace(self.inner().data.get(), val) }
    }
}

impl<T: Copy> SPtr<T> {
    pub fn load(&self) -> T {
        unsafe { *self.inner().data.get() }
    }
}

impl<T: ?Sized> Drop for SPtr<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.inner.as_ptr());
        }
    }
}

impl<T: ?Sized> Deref for SPtr<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner().data.get() }
    }
}
