use rust_queues::benchmark_utils::{self, BenchmarkType::Pairwise};
use rust_queues::lprq::epoch_lprq::lprq::SharedLPRQ;
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

    let queue: SharedLPRQ<i32, 1024> = SharedLPRQ::new();

    pairwise_benchmark::benchmark(threads, logn, even_only, congestion_factor, queue);

    println!("  Finished");
}
