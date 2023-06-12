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
use util::try_lock::TryLock;

struct SyncCellUnsafe<T> {
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



#[test]
fn basic(){
    fn test_thread(n: usize){
        let val = SyncCellUnsafe::new(0);
        let lock = TryLock::new();
        std::thread::scope(|scope| {
            for _ in 0..n {
                let valref = &val;
                let lockref = &lock;
                scope.spawn(move || {
                    synchronized!(lockref, {
                        valref.swap(valref.load() + 1);
                    });
                });
            }
        });
        assert_eq!(val.load(), n);
    }
    test_thread(1000);
    test_thread(1000);
    test_thread(1000);
    test_thread(1000);
    test_thread(1000);
}

