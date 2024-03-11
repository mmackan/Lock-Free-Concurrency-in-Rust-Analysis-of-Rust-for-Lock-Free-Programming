use rust_queues::lprq::lprq::SharedLPRQ;
use rust_queues::shared_queue::SharedQueue;
use rust_queues::pairwise_benchmark;
use rust_queues::benchmark_utils::{self, BenchmarkType::Pairwise};

fn main() {
    let benchmark = benchmark_utils::parse_args("pairwise");

    let (threads, logn, even_only) = match benchmark {
        Pairwise(threads, logn, even_only) => {
            (threads, logn, even_only)
        }
        _ => panic!("Expected a 'Pairwise' benchmark type"),
    };

    let queue: SharedLPRQ<'_, i32, 1028> = SharedLPRQ::new();

    pairwise_benchmark::benchmark(threads, logn, even_only, queue);

    println!("  Finished");
}
