use rust_queues::{benchmark_utils, lprq::lprq::SharedLPRQ, pairwise_benchmark, shared_queue::SharedQueue};

fn main() {

    let (threads, logn, even_only) = benchmark_utils::parse_args_pairwise();

    let queue: SharedLPRQ<'_, i32, 1028> = SharedLPRQ::new();

    pairwise_benchmark::benchmark(threads, logn, even_only,  queue);

    println!("  Finished");
}