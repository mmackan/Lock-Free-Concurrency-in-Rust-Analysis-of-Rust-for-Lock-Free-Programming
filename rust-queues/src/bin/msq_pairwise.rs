use rust_queues::ms_queue::msq_hazp::MSQueue;
use rust_queues::shared_queue::SharedQueue;

use rust_queues::{benchmark_utils, pairwise_benchmark};

/// Default exponent for # operations
const LOGN_OPS: usize = 7;

fn main() {
    let (threads, logn, even_only) = benchmark_utils::parse_args_pairwise();

    let queue = MSQueue::new();

    pairwise_benchmark::benchmark(threads, logn, even_only, queue);

    println!("  Finished");
}
