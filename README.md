# DATX05-Master-Thesis
To benchmark and gather all comparison graphs, simply run:
- `cargo make run` (not working: require C++ LPRQ binary)
NOTE: This command might take some time to finnish, the following sections will provide individual commands that can be executed.

***

## Benchmarking
Benchmark the queues with pairwise or multi-producer multi-consumer (MPMC) workload.  These commands will also build the corresponding project before running the benchmark.

**Run all benchmarks for all queues**
- `cargo make benchmark-all` (not working: require C++ LPRQ binary)

**Both benchmarks for MSQ and LPRQ respectively:**
- `cargo make benchmark-msq`
- `cargo make benchmark-lprq`

**Run individual benchmarks**
- `cargo make benchmark-pairwise-msq`
- `cargo make benchmark-pairwise-lprq` (not working: require C++ LPRQ binary)
- `cargo make benchmark-mpmc-msq`
- `cargo make benchmark-mpmc-lprq` (not working: require C++ LPRQ binary)

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
- `cargo make graph-all` (not working: require C++ LPRQ binary)

**Create graphs for MSQ and LPRQ respectively**
- `cargo make graph-msq`
- `cargo make graph-lprq` (not working: require C++ LPRQ binary)

**Create individual graphs**
- `cargo make graph-pairwise-msq`
- `cargo make graph-pairwise-lprq` (not working: require C++ LPRQ binary)
- `cargo make graph-mpmc-msq`
- `cargo make graph-mpmc-lprq` (not working: require C++ LPRQ binary)