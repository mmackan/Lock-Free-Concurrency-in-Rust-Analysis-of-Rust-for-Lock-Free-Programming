use rust_queues::benchmark_utils::{self, BenchmarkType::Mpmc};
use rust_queues::lprq::lprq::SharedLPRQ;
use rust_queues::mpmc_benchmark;
use rust_queues::shared_queue::SharedQueue;

fn main() {
    let benchmark = benchmark_utils::parse_args("mpmc");

    let (producers, consumers, logn, even_only) = match benchmark {
        Mpmc(producers, consumers, logn, even_only) => (producers, consumers, logn, even_only),
        _ => panic!("Expected a 'Mpmc' benchmark type"),
    };

    let queue: SharedLPRQ<'_, i32, 1028> = SharedLPRQ::new();
    
    mpmc_benchmark::benchmark(producers, consumers, logn, even_only, queue);

    println!("  Finished");
}
