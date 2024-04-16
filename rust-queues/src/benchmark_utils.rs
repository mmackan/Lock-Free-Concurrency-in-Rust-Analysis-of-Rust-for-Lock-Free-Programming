use std::env::{self, args};

/// Default exponent for # operations
const LOGN_OPS: usize = 7;

pub enum BenchmarkType {
    /// (Threads, logn, even_only)
    Pairwise(usize, usize, bool, f32),

    /// (Producers, consumers, logn, even_only)
    Mpmc(usize, usize, usize, bool, f32),
}

pub fn parse_args(benchmark: &str) -> BenchmarkType {
    let args: Vec<String> = env::args().collect();

    match benchmark {
        "pairwise" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: {} <threads> [exponent_base_ten] [even_cores_only] [congestion_factor]",
                    args[0]
                );
                std::process::exit(1);
            }

            let threads: usize = args[1]
                .parse()
                .expect("Number of threads must be a positive");
            if threads == 0 {
                eprintln!("Number of threads cannot be 0.");
                std::process::exit(1);
            }

            let logn: usize = if args.len() > 2 {
                args[2].parse().expect("Exponent must be positive")
            } else {
                LOGN_OPS
            };

            let even_only: bool = if args.len() > 3 {
                args[3].parse().expect("Valid values: true, false")
            } else {
                false
            };
            let congestion_factor: f32 = if args.len() > 4 {
                args[4].parse().expect("A float between 0.0..1.0")
            } else {
                0.0
            };

            println!("===========================================");
            println!("  Benchmark: {}", args[0]);
            println!("  Threads: {}", threads);
            println!("  Operations: 10^{}", logn);
            println!("  Even cores only: {}", even_only);
            println!("  Congestion factor: {}", congestion_factor);

            BenchmarkType::Pairwise(threads, logn, even_only, congestion_factor)
        }

        "mpmc" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage for mpmc: {} <producers> <consumers> [logn] [even_cores_only]",
                    args[0]
                );
                std::process::exit(1);
            }

            let producers: usize = args[1]
                .parse()
                .expect("Number of producers must be a positive");
            if producers == 0 {
                eprintln!("Number of producers cannot be 0.");
                std::process::exit(1);
            }

            let consumers: usize = args[2]
                .parse()
                .expect("Number of consumers must be a positive");
            if consumers == 0 {
                eprintln!("Number of consumers cannot be 0.");
                std::process::exit(1);
            }

            let logn: usize = if args.len() > 3 {
                args[3].parse().expect("Exponent must be positive")
            } else {
                LOGN_OPS
            };

            let even_only: bool = if args.len() > 4 {
                args[4].parse().expect("Valid values: true, false")
            } else {
                false
            };
            let congestion_factor: f32 = if args.len() > 5 {
                args[5].parse().expect("A float between 0.0..1.0")
            } else {
                0.0
            };

            println!("===========================================");
            println!("  Benchmark: {}", args[0]);
            println!("  Producers: {}", producers);
            println!("  Consumers: {}", consumers);
            println!("  Operations: 10^{}", logn);
            println!("  Even cores only: {}", even_only);
            println!("  Congestion factor: {}", congestion_factor);

            BenchmarkType::Mpmc(producers, consumers, logn, even_only, congestion_factor)
        }

        _ => {
            eprintln!("Invalid benchmark type. Must be either 'pairwise' or 'mpmc'.");
            std::process::exit(1);
        }
    }
}
