use std::env;

use rust_queues::{lcrq::lprq::SharedLPRQ, pairwise_benchmark, shared_queue::SharedQueue};


const LOGN_OPS: u32 = 7;

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <threads> [exponent_base_ten]", args[0]);
        std::process::exit(1);
    }

    let threads: u32 = args[1].parse().expect("Number of threads must be a positive integer");
    if threads == 0 {
        eprintln!("Number of threads cannot be 0.");
        std::process::exit(1);
    }

    let logn: u32 = if args.len() > 2 {
        args[2].parse().expect("Exponent must be positive integer")
    } else {
        LOGN_OPS
    };

    println!("===========================================");
    println!("  Benchmark: {}", args[0]);
    println!("  Threads: {}", threads);
    println!("  Operations: 10^{}", logn);

    let queue: SharedLPRQ<'_, i32, 1028> = SharedLPRQ::new();

    pairwise_benchmark::benchmark(threads, logn, queue);

    println!("  Finished");
}