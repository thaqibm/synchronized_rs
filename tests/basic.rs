use synchronized::*;
use synchronized::util::*;

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
    test_thread(100);
    test_thread(100);
    test_thread(100);
    test_thread(100);
    test_thread(100);
}
