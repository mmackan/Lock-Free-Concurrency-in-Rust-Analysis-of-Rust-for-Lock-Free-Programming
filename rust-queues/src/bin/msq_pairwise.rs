use rust_queues::benchmark_utils::{self, BenchmarkType::Pairwise};
use rust_queues::ms_queue::msq_hazp::MSQueue;
use rust_queues::pairwise_benchmark;
use rust_queues::shared_queue::SharedQueue;

fn main() {
    let benchmark = benchmark_utils::parse_args("pairwise");

    let (threads, logn, even_only, congestion_factor) = match benchmark {
        Pairwise(threads, logn, even_only, congestion_factor) => {
            (threads, logn, even_only, congestion_factor)
        }
        _ => panic!("Expected a 'Pairwise' benchmark type"),
    };

    let queue = MSQueue::new();

    pairwise_benchmark::benchmark(threads, logn, even_only, congestion_factor, queue);

    println!("  Finished");
}
