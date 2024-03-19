# DATX05-Master-Thesis
To benchmark and gather all comparison graphs, simply run:
- `cargo make run`

NOTE: This command might take some time to finnish, the following sections will provide individual commands that can be executed.

***

## Benchmarking
Benchmark the queues with pairwise or multi-producer multi-consumer (MPMC) workload. Each benchmark will run two parameter scans each based on the categories:
1. Fixed number of operations and different # threads.
2. Fixed number of threads and different # operations.

**All benchmarks**
- `cargo make benchmark-all`

**Both benchmarks for MSQ and LPRQ respectively:**
- `cargo make benchmark-msq`
- `cargo make benchmark-lprq`

**Individual benchmarks for both categories**
- `cargo make benchmark-pairwise-msq`
- `cargo make benchmark-pairwise-lprq`
- `cargo make benchmark-mpmc-msq`
- `cargo make benchmark-mpmc-lprq`

**Benchmark each individual category**

Adding a `-t` or `-o` will test individual categories
- e.g., `cargo make benchmark-pairwise-msq-t`


***

## Building & cleaning
These commands allows you to build the whole project or just each individual queue.

**Build whole project**
- `cargo make build`

**Individual builds**
- `cargo make build-rust-queues`
- `cargo make build-c-msq`
- `cargo make build-c-lprq`

**Clean whole project**
- `cargo make clean`

***

## Plotting the graphs (final results)
These commands will generate graphs that show the comparison between the two languages for respective benchmark workload.

**Create graphs for all benchmarks**
- `cargo make graph-all`

**Create graphs for MSQ and LPRQ respectively**
- `cargo make graph-msq`
- `cargo make graph-lprq`

**Create individual graphs**
- `cargo make graph-pairwise-msq`
- `cargo make graph-pairwise-lprq`
- `cargo make graph-mpmc-msq`
- `cargo make graph-mpmc-lprq`

**Create graphs for each individual category**

Adding a `-t` or `-o` will graph individual categories
- e.g., `cargo make graph-pairwise-msq-t`