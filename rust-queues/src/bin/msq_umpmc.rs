use std::env;

use rust_queues::ms_queue::msq_hazp::MSQueue;
use rust_queues::shared_queue::SharedQueue;

use rust_queues::mpmc_benchmark;

/// Default exponent for # operations
const LOGN_OPS: u32 = 7;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <producers> <consumers> [exponent_base_ten]", args[0]);
        std::process::exit(1);
    }

    let producers: u32 = args[1].parse().expect("Number of producers must be a positive integer");
    if producers == 0 {
        eprintln!("Number of producers cannot be 0.");
        std::process::exit(1);
    }

    let consumers: u32 = args[2].parse().expect("Number of consumers must be a positive integer");
    if consumers == 0 {
        eprintln!("Number of consumers cannot be 0.");
        std::process::exit(1);
    }

    let logn: u32 = if args.len() > 3 {
        args[3].parse().expect("Exponent must be positive integer")
    } else {
        LOGN_OPS
    };

    println!("===========================================");
    println!("  Benchmark: {}", args[0]);
    println!("  Producers: {}", producers);
    println!("  Consumers: {}", consumers);
    println!("  Operations: 10^{}", logn);

    let queue = MSQueue::new();

    mpmc_benchmark::benchmark(producers, consumers, logn, queue);

    println!("  Finished");
}
