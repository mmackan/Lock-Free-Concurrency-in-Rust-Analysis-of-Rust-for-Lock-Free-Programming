use std::arch::asm;
use std::sync::Arc;
use std::env;
use std::thread;
use haphazard::HazardPointer;
use rand::rngs::ThreadRng;
use rand::Rng;

use rust_queues::ms_queue::msq_hazp::Queue;

/// Default exponent for # operations
const LOGN_OPS: u32 = 7;
const BASE: u32 = 10;

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

    benchmark(threads, logn);

    println!("  Finished");
}

fn benchmark(nprocs: u32, logn: u32) {

    let queue = Arc::new(Queue::new());
    let mut handles = vec![];
    
    // Calculate number of operations
    let nops = BASE.pow(logn);
    let tops = (nops / nprocs) as i32;
    
    for _ in 0..nprocs {       
        let queue = Arc::clone(&queue);

        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut hazp = HazardPointer::new();
            let mut hazp2 = HazardPointer::new();

            for j in 0..tops {
                queue.enqueue(j, &mut hazp);
                delay_exec(&mut rng);

                queue.dequeue(&mut hazp, &mut hazp2);
                delay_exec(&mut rng);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn delay_exec(state: &mut ThreadRng) {
    let n = state.gen_range(0..100);
    let delay_end = 50 + n % 100;

    for _ in 50..delay_end {
        #[cfg(not(miri))]
        unsafe {
            asm!("nop");
        }
    }
}