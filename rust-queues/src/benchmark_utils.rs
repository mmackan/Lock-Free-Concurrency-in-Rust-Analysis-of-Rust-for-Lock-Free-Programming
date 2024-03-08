use std::env;

const LOGN_OPS: usize = 7;

pub fn parse_args_pairwise() -> (usize, usize, bool) {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <threads> [exponent_base_ten] [even_cores_only]",
            args[0]
        );
        std::process::exit(1);
    }

    let threads: usize = args[1]
        .parse()
        .expect("Number of threads must be a positive integer");
    if threads == 0 {
        eprintln!("Number of threads cannot be 0.");
        std::process::exit(1);
    }

    let logn: usize = if args.len() > 2 {
        args[2].parse().expect("Exponent must be positive integer")
    } else {
        LOGN_OPS
    };

    let even_only: bool = if args.len() > 3 {
        args[3].parse().expect("Valid values: true, false")
    } else {
        false
    };

    println!("===========================================");
    println!("  Benchmark: {}", args[0]);
    println!("  Threads: {}", threads);
    println!("  Operations: 10^{}", logn);
    return (threads, logn, even_only);
}
