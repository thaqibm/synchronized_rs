## Synchronized 

Blazingly Fast 🚀 Low level multithreading library in rust


**Key Features**

### Distributed Multithreaded counter
```{rust}
use rayon::prelude::*;
use synchronized::util::*;

let counter = Counter::new(MAX_THREADS as u64); // Counter provides interior mutability
let tids = (0..MAX_THREADS).collect::<Vec<_>>();
tids.par_iter()
    .for_each(|&tid| {
            (0..1000).for_each(|_| { // 1000 increments on each thread
                    counter.inc(tid);
                    );
            });

let approx_count = counter.get(); // get() returns approx count
assert_eq!(counter.get_accurate(), 1000 * MAX_THREADS); // get_accurate returns the accurate count
```

### Versioned TryLock

```{rust}
use synchronized::util::*;
let lock = TryLock::new();
std::thread::scope(|s|{
        for _ in 0..32 {
        s.spawn(|| {
                lock.acquire();
                std::thread::sleep(time::Duration::from_millis(100));
                lock.release();
                });
        }
        });

assert!(!lock.is_held());
assert_eq!(lock.acquire_count(), 32); // Keeps track of times lock was acquired
```
The lock uses bits of an atomic integer to keep track of the count and doesn't use extra
bytes for storing the count.


#### Unstable Features
These features are unstable and there might be breaking changes in the future.

##### Intel Hardware Transactional memory intrinsics
The crate provides intrinsics for ``x86_64`` RTM intrinsics.
```{rust}
let mut res = false;
unsafe {
    if _xbegin() == _XBEGIN_STARTED { // begin transaction
        res = _xtest(); // test if within transaction
        _xend();
        assert!(res);
    }
    else {
        assert!(!res);
    }
}

// Implementation of _xbegin
#[inline(always)]
pub unsafe fn _xbegin() -> u32 {
    let status: u32;
    asm!(
            "mov eax, 0xFFFFFFFF",
            "xbegin 2f",
            "2:",
            "mov {0:e}, eax",
            out(reg) status,
        );
    status
}

// Check at runtime if rmt is available
pub fn htm_supported_runtime() -> bool {
    std::arch::is_x86_feature_detected!("rtm")
}
```


