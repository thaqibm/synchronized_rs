use criterion::*;
use std::sync::Mutex;
use synchronized::util::{Counter, MAX_THREADS};



struct MutexCounter {
    counter: Mutex<usize>
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
    let bytes : [u8; 3] = [1,2,3];

    let mut group = c.benchmark_group("counter-throughput");
    for i in [10000000u64].iter() {
            group.throughput(Throughput::Elements(bytes.len() as u64));
            group.bench_with_input(BenchmarkId::new("mutex_counter", i), i, 
                |b, i| b.iter(|| mutex_counter(*i as usize)));

            group.bench_with_input(BenchmarkId::new("sync counter", i), i, 
                |b, i| b.iter(|| synchronized_counter(*i as usize)));
        }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);

