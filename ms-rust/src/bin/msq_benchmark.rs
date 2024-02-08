use core::time;
use std::sync::Arc;
use std::env;
use std::thread;
use rand::Rng;

use ms_rust::msq_hazp::Queue;

/// Default exponent for # operations
const LOGN_OPS: u32 = 7;
const BASE: u32 = 10;

// Delay will be between 50~150ns
const DELAY_LOW: u64 = 50;
const DELAY_UPPER: u64 = 150;

// cargo run --release --bin benchmark1 <threads> [exponent_base_ten]

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

    println!("RUNNING BENCHMARK");
    println!(" Threads: {}", threads);
    println!(" Operations: 10^{}", logn);

    benchmark(threads, logn);

    println!("BENCHMARK COMPLETED");
}

fn benchmark(nprocs: u32, logn: u32) {

    let queue = Arc::new(Queue::new());
    let mut handles = vec![];
    let mut rng = rand::thread_rng();

    // Calculate number of operations
    let nops = BASE.pow(logn);
    let tops = (nops / nprocs) as i32;

    for _ in 0..nprocs {

        // Randomized "work" time
        let rt = rng.gen_range(DELAY_LOW..DELAY_UPPER);
        let dur = time::Duration::from_nanos(rt);

        let queue = Arc::clone(&queue);
        let handle = thread::spawn(move || {

            println!(" Thread {:?} running..", thread::current().id());
            for j in 0..tops {
                queue.enqueue(j);
                thread::sleep(dur);
                queue.dequeue();
            }
            println!(" Thread {:?} done", thread::current().id());

        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Should be empty  
    assert_eq!(queue.dequeue(), None);
}