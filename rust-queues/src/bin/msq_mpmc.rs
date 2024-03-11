use rust_queues::ms_queue::msq_hazp::MSQueue;
use rust_queues::shared_queue::SharedQueue;
use rust_queues::mpmc_benchmark;
use rust_queues::benchmark_utils::{self, BenchmarkType::Mpmc};

fn main() {
    let benchmark = benchmark_utils::parse_args("mpmc");
    
    let (producers, consumers, logn, even_only) = match benchmark {
        Mpmc(producers, consumers, logn, even_only) => {
            (producers, consumers, logn, even_only)
        }
        _ => panic!("Expected a 'Mpmc' benchmark type"),
    };

    let queue = MSQueue::new();

    mpmc_benchmark::benchmark(producers, consumers, logn, even_only, queue);

    println!("  Finished");
}
