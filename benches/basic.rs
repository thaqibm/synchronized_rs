use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::{Mutex, atomic::AtomicU64};
use synchronized::util::{Counter, MAX_THREADS};


struct MutexCounter {
    counter: Mutex<usize>
}


fn atomic_int_counter(per_thread: usize){
    let counter = AtomicU64::new(0);
    std::thread::scope(|s| {
        (0..MAX_THREADS).for_each(|_tid| {
            s.spawn(|| {
                for _ in 0..per_thread  {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            });
        });
    });
    assert!(counter.load(std::sync::atomic::Ordering::Relaxed) == (per_thread * MAX_THREADS) as u64)
}

fn mutex_counter(per_thread: usize){
    let counter = MutexCounter{counter: Mutex::new(0)};
    std::thread::scope(|s| {
        (0..MAX_THREADS).for_each(|_tid| {
            s.spawn(|| {
                for _ in 0..per_thread  {
                    *counter.counter.lock().unwrap() += 1
                }
            });
        });
    });
    assert!(*counter.counter.lock().unwrap() == per_thread * MAX_THREADS)
}

fn synchronized_counter(per_thread: usize){
    let counter = Counter::new(MAX_THREADS as u64);
    std::thread::scope(|s| {
        (0..MAX_THREADS).for_each(|tid| {
            let cref = &counter;
            s.spawn(move || {
                for _ in 0..per_thread  {
                    cref.inc(tid);
                }
            });
        });
    });
    assert_eq!(counter.get_accurate(), (per_thread * MAX_THREADS) as u64)
}


fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("counters");

    for i in [1000u64, 10000u64, 100000u64, 1000000u64].iter() {

        group.bench_with_input(BenchmarkId::new("atomic_counter", i), i, 
            |b, i| b.iter(|| atomic_int_counter(*i as usize)));

        group.bench_with_input(BenchmarkId::new("mutex_counter", i), i, 
            |b, i| b.iter(|| mutex_counter(*i as usize)));

        group.bench_with_input(BenchmarkId::new("sync_counter", i), i, 
            |b, i| b.iter(|| synchronized_counter(*i as usize)));
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

