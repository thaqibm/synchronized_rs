
#[cfg(test)]
mod tests {
    use std::alloc::System;
    use mockalloc::Mockalloc;

    use synchronized::*;
    use synchronized::util::*;

    #[global_allocator]
    static ALLOCATOR: Mockalloc<System> = Mockalloc(System);

    #[test]
    fn basic(){
        fn test_thread(n: usize){
            let val = SPtr::new(0);
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


    #[test]
    fn alloc_test(){
        mockalloc::assert_allocs(|| {
            let ptr = [SPtr::new(10), SPtr::new(10), SPtr::new(10)];
            ptr[0].store(20);
            assert_eq!(ptr[0].load(), 20);
        });
    }

}

