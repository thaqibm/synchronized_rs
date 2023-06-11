

pub mod counter {
    use std::{sync::atomic::AtomicU64, convert::TryInto};
    use std::sync::atomic::Ordering::SeqCst;
    use std::cmp;


    // TODO: pass num-threads at compile time
    #[cfg(feature = "max-threads")]
    todo!();

    pub const MAX_THREADS: usize = 32;


    const ALIGN: usize = 64;
    const PADDING_BYTES: usize = 64;
    type Byte = bool;

    type PadBytes<const N: usize> = [bool;  N];

    #[repr(C, align(64))]
    struct AtomicU64Padded(AtomicU64);

    impl AtomicU64Padded {
        pub fn new(val: u64) -> Self{
            Self(AtomicU64::new(val))
        }
    }


    #[repr(C, align(64))]
    pub struct Counter{
        _padding0: PadBytes<PADDING_BYTES>,
        sub_counters: [AtomicU64Padded; MAX_THREADS],
        global_count: AtomicU64,
        _padding1: PadBytes<PADDING_BYTES>,
        num_threads: u64
    }
    impl Counter {
        const C: u64 = 10;
        const LIMIT: u64 = 1000;

        pub fn new(num_threads: u64) -> Self{
            Self { 
                _padding0: [false; PADDING_BYTES],
                sub_counters: (0..MAX_THREADS)
                    .map(|_| AtomicU64Padded::new(0))
                    .collect::<Vec<_>>()
                    .try_into().ok().unwrap(),
                global_count: AtomicU64::new(0), 
                _padding1: [false; PADDING_BYTES],
                num_threads
            }
        }
        pub fn get(&self) -> u64 {
            self.global_count.load(SeqCst)
        }

        pub fn inc(&self, tid: usize) {
            let res = self.sub_counters[tid].0.fetch_add(1, SeqCst);
            let val = res + 1;
            if val >= cmp::max(Counter::LIMIT, Counter::C * self.num_threads) {
                // flush to global_count
                self.global_count.fetch_add(val, SeqCst);
                self.sub_counters[tid].0.store(0, SeqCst);
            }
        }

        pub fn set(&self, val: u64) {
            self.global_count.store(val, SeqCst);
            for counter in &self.sub_counters{
                counter.0.store(0, SeqCst);
            }
        }

        pub fn get_accurate(&self) -> u64{
            let subcnt = self.sub_counters.iter().
                fold(0, |acc, counter|{
                    let cnt = counter.0.load(SeqCst);
                    acc + cnt
                });
            subcnt + self.global_count.load(SeqCst)
        }
    }

    //SAFETY: Safe to share counter bewteen threads
    unsafe impl Sync for Counter {}
    unsafe impl Send for Counter {}

    #[test]
    fn type_sizes(){
        use std::mem::size_of;
        let padbytes_size = size_of::<PadBytes<64>>();   
        assert_eq!(padbytes_size, 64);

        let paddedu64_size = size_of::<AtomicU64Padded>();
        let u64_size = size_of::<AtomicU64>();
        assert_eq!(u64_size, 8);
        assert_eq!(paddedu64_size, 64);
    }

    #[test]
    fn type_padding(){
        let paddedarr = [AtomicU64Padded::new(0), AtomicU64Padded::new(1)];
        let a1 = &paddedarr[0].0 as *const AtomicU64 as usize;
        let a2 = &paddedarr[1].0 as *const AtomicU64 as usize;

        assert!(a2 - a1 >= 64);
        assert_eq!(a1%64, 0);
        assert_eq!(a2%64, 0);
    }

    #[test]
    fn test_init(){
        let counter = Counter::new(10);
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_inc(){
        let counter = Counter::new(10);

        (0..MAX_THREADS).for_each(|i|{
            counter.inc(i);
        });

        // each sub counter has count 1
        assert_eq!(counter.get(), 0);

        assert_eq!(counter.get_accurate(), MAX_THREADS as u64);

        let init = 20;
        counter.set(init);

        (0..MAX_THREADS).for_each(|i|{
            counter.inc(i);
        });

        assert_eq!(counter.get_accurate(), init + MAX_THREADS as u64);
    }

    #[test]
    fn test_limit(){
        use super::counter::Counter;
        let counter = Counter::new(MAX_THREADS as u64);
        let limit = std::cmp::max(Counter::LIMIT, Counter::C * MAX_THREADS as u64);

        (0..MAX_THREADS).for_each(|i|{
            (0..limit - 1).for_each(|_| {
                counter.inc(i);
            });
        });

        assert_eq!(counter.get(), 0);

        // this should flush one sub counter
        assert_eq!(limit - 1, counter.sub_counters[0].0.load(SeqCst));
        counter.inc(0);
        assert_eq!(0, counter.sub_counters[0].0.load(SeqCst));
        assert_eq!(counter.get(), limit);
        assert_eq!(counter.get_accurate(), (MAX_THREADS as u64)*(limit - 1) + 1);
    }

    #[test]
    fn test_multithread(){
        use super::counter::*;
        let counter = Counter::new(MAX_THREADS as u64);
        let limit = std::cmp::max(Counter::LIMIT, Counter::C * MAX_THREADS as u64);
        (0..MAX_THREADS).for_each(|tid| {
            std::thread::scope(|s| {
                s.spawn(|| {
                    (0..limit - 1).for_each(|_| {
                        counter.inc(tid);
                    });
                });
            });
        });

        assert_eq!(counter.get(), 0);
        (0..MAX_THREADS).for_each(|tid| {
            std::thread::scope(|s| {
                s.spawn(|| {
                    counter.inc(tid);
                });
            });
        });
        assert_eq!(counter.get(), (MAX_THREADS as u64) * limit);
    }

    #[test]
    fn test_rayon(){
        use rayon::prelude::*;
        let counter = Counter::new(MAX_THREADS as u64);
        let limit = std::cmp::max(Counter::LIMIT, Counter::C * MAX_THREADS as u64);
        let tids = (0..MAX_THREADS).collect::<Vec<_>>();
        tids.par_iter()
            .for_each(|&tid| {
                (0..limit - 1).for_each(|_| {
                    counter.inc(tid);
                });
            });
        assert_eq!(counter.get(), 0);
        tids.par_iter().for_each(|&tid| {
            counter.inc(tid);
        });
        assert_eq!(counter.get(), (MAX_THREADS as u64) * limit);

        tids.par_iter()
            .for_each(|&tid| {
                for _ in 0..(3*limit) {
                    counter.inc(tid);
                }
            });

        let exp = (MAX_THREADS as u64) * limit + (MAX_THREADS as u64)*limit*3;
        assert_eq!(counter.get_accurate(), exp);
    }

}


pub mod try_lock{
    use std::{sync::atomic::AtomicU64};
    use std::sync::atomic::Ordering::SeqCst;
    use std::{time, cmp};

    pub struct TryLock{
        state: AtomicU64, // last bit tells if lock is held. rest of the bits are for counting 
    }

    impl TryLock {
        pub fn new() -> Self {
            TryLock { state: AtomicU64::new(0) }
        }

        pub fn try_acquire(&self) -> bool {
            // TODO: Change the SeqCst
            let read = self.state.load(SeqCst);
            if (read&1) != 0 {
                false
            }
            else {
                self.state.compare_exchange_weak(read, read|1, SeqCst, SeqCst).is_ok()
            }
        }

        pub fn acquire(&self) {
            while !self.try_acquire() {
                std::hint::spin_loop();
            }
        }

        pub fn release(&self){
            assert!(self.is_held());
            self.state.fetch_add(1, SeqCst);
        }

        pub fn is_held(&self) -> bool{
            (self.state.load(SeqCst) & 1) != 0
        }

        pub fn acquire_count(&self) -> u64 {
            self.state.load(SeqCst) >> 1
        }
    }

    #[test]
    fn init_test(){
        let lock = TryLock::new();
        assert!(!lock.is_held());
        lock.acquire();
        assert!(lock.is_held());
    }

    #[test]
    fn test_multithread(){
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
        assert_eq!(lock.acquire_count(), 32);
    }

    #[test]
    fn test_rayon(){
        use rayon::prelude::*;
        let lock = TryLock::new();
        [0; 10].par_iter()
        .for_each(|_|{
                lock.acquire();
                std::thread::sleep(time::Duration::from_millis(100));
                lock.release();
        });
        assert!(!lock.is_held());
        assert_eq!(lock.acquire_count(), 10);
    }

}

/*
pub mod intrinsics {
    extern "C" {
        #[link_name = "llvm.x86.xbegin"]
        pub fn xbegin() -> i32;

        #[link_name = "llvm.x86.xend"]
        pub fn xend() -> ();

        #[link_name = "llvm.x86.xabort"]
        pub fn xabort(a: i8) -> ();

        #[link_name = "llvm.x86.xtest"]
        pub fn xtest() -> i32;
    }
}
*/
