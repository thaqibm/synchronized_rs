

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

        pub fn inc(&mut self, tid: usize) {
            let val = self.sub_counters[tid].0.fetch_add(1, SeqCst);
            if val >= cmp::max(Counter::LIMIT, Counter::C * self.num_threads) {
                // flush to global_count
                self.global_count.fetch_add(val, SeqCst);
                self.sub_counters[tid].0.store(0, SeqCst);
            }
        }

        pub fn set(&mut self, val: u64) {
            self.global_count.store(val, SeqCst);
            for counter in &self.sub_counters{
                counter.0.store(0, SeqCst);
            }
        }

        pub fn getAccurate(&self) -> u64{
            let subcnt = self.sub_counters.iter().
                fold(0, |acc, counter|{
                    let cnt = counter.0.load(SeqCst);
                    acc + cnt
                });
            subcnt + self.global_count.load(SeqCst)
        }
    }


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
    fn test_init(){
        let counter = Counter::new(10);
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_inc(){
        let mut counter = Counter::new(10);

        (0..MAX_THREADS).for_each(|i|{
            counter.inc(i);
        });

        // each sub counter has count 1
        assert_eq!(counter.get(), 0);

        assert_eq!(counter.getAccurate(), MAX_THREADS as u64);

        let init = 20;
        counter.set(init);

        (0..MAX_THREADS).for_each(|i|{
            counter.inc(i);
        });

        assert_eq!(counter.getAccurate(), init + MAX_THREADS as u64);

    }

    
}
