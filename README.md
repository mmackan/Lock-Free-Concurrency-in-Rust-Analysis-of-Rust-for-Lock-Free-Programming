# DATX05-Master-Thesis
To benchmark and gather all comparison graphs, simply run:
- `cargo make run`

NOTE: This command might take some time to finnish, the following sections will provide individual commands.

***

## Building & cleaning
These commands allows you to build the whole project or just each individual queue. **Required before benchmarking**

**Build whole project**
- `cargo make build`

**Individual builds**
- `cargo make build-rust-queues`
- `cargo make build-c-msq`
- `cargo make build-c-lprq`

**Clean whole project**
- `cargo make clean`

***

## Benchmarking
The benchmark workloads are Pairwise (PW) or Multi-Producer Multi-Consumer (MPMC). The producer-consumer ratio for MPMC is 1:1, 1:2, and 2:1.

PW include two categories:
1. Scan over number of threads with fixed number of operations.
2. Scan over number of operations with fixed number of threads.

Available workloads for respective queue:
- LPRQ supports both PW and MPMC.
- MSQ supports only PW.

**All benchmarks**
- `cargo make benchmark-all`

**Individual queues**
- `cargo make benchmark-msq`
- `cargo make benchmark-lprq`

**Individual workloads for LPRQ**
- `cargo make benchmark-pairwise-lprq`
- `cargo make benchmark-mpmc-lprq`

**Individual categories for PW workload**

Adding a `-t` will scan over threads, `-o` over operations

- `cargo make benchmark-pairwise-msq-t`
- `cargo make benchmark-pairwise-msq-o`
- `cargo make benchmark-pairwise-lprq-t`
- `cargo make benchmark-pairwise-lprq-o`

**Individual ratios for LPRQ's MPMC workload**
- `cargo make benchmark-mpmc-lprq-1-1`
- `cargo make benchmark-mpmc-lprq-1-2`
- `cargo make benchmark-mpmc-lprq-2-1`

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