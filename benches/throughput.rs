use criterion::*;
use synchronized::util::{Counter, MAX_THREADS};
use std::sync::{Mutex, atomic::AtomicU64};




struct MutexCounter {
    counter: Mutex<usize>
}


fn atomic_int_counter(per_thread: usize){
    let counter = AtomicU64::new(0);
    (0..MAX_THREADS).for_each(|_tid| {
        std::thread::scope(|s| {
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
    (0..MAX_THREADS).for_each(|_tid| {
        std::thread::scope(|s| {
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
    (0..MAX_THREADS).for_each(|tid| {
        std::thread::scope(|s| {
            s.spawn(|| {
                for _ in 0..per_thread {
                    counter.inc(tid);
                }
            });
        });
    });
    assert_eq!(counter.get_accurate(), (per_thread * MAX_THREADS) as u64)
}


fn bench(c: &mut Criterion) {

    let mut group = c.benchmark_group("counter-throughput");
    group.sample_size(10000);
    group.sampling_mode(criterion::SamplingMode::Flat);

    for i in  [1000u64, 10000u64, 100000u64, 1000000u64, 
        2000000u64, 3000000u64, 4000000u64, 5000000u64,
        6000000u64, 7000000u64, 8000000u64, 9000000u64, 10000000u64].iter() {

            group.throughput(Throughput::Elements(i.clone()));

            group.bench_with_input(BenchmarkId::new("atomic_counter", i), i, 
                |b, i| b.iter(|| atomic_int_counter(*i as usize)));

            group.bench_with_input(BenchmarkId::new("mutex_counter", i), i, 
                |b, i| b.iter(|| mutex_counter(*i as usize)));

            group.bench_with_input(BenchmarkId::new("sync_counter", i), i, 
                |b, i| b.iter(|| synchronized_counter(*i as usize)));

        }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);

