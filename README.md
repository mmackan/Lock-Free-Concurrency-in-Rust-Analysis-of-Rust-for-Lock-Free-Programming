# DATX05-Master-Thesis
`cd` to the `rust-queues` folder to execute `cargo make` commands.


To benchmark and gather all comparison graphs, simply build and run:
- `cargo make build`
- `cargo make run`
	- **NOTE:** estimated time is 10-12 hours

***

## Building & cleaning
These commands allows you to build the whole project or just each individual queue. Required before benchmarking.

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
The benchmark workloads are Pairwise (PW) or Multi-Producer Multi-Consumer (MPMC). The producer-consumer ratio for MPMC is 1:1 and 2:1.

- MSQ supports only PW.
- LPRQ supports both PW and MPMC.

### All benchmarks
- `cargo make benchmark-msq`
- `cargo make benchmark-lprq`
- `cargo make benchmark-arc`

### Congestion benchmarks
- `cargo make benchmark-msq-congestion`
- `cargo make benchmark-lprq-congestion`

### Energy benchmarks
Benchmark energy consumption using RAPL. Only for LPRQ
- `cargo make benchmark-energy-lprq`
- `cargo make benchmark-energy-lprq-1-1`
- `cargo make benchmark-energy-lprq-2-1`

### Individual benchmarks
Individual commands that benchmarks both Rust and reference implementations.

#### PW workload
- `cargo make benchmark-msq-pw`
- `cargo make benchmark-lprq-pw`

#### MPMC workload
- `cargo make benchmark-lprq-1-1`
- `cargo make benchmark-lprq-2-1`

#### Workloads with congestion
- `cargo make benchmark-msq-congestion`
- `cargo make benchmark-lprq-congestion-pw`
- `cargo make benchmark-lprq-congestion-1-1`
- `cargo make benchmark-lprq-congestion-2-1`

***

## Plotting graphs (final results)
After benchmarks have been successfully executed these commands will generate graphs that show the comparison between the two languages.

### All graphs
- `cargo make graph-all`
- `cargo make graph-msq`
- `cargo make graph-lprq`
- `cargo make graph-congestions`

**Include the Arc-version to LPRQ comparison plots**
- `cargo make graph-arc`

### Individual graphs
Individual graphs comparing Rust against the reference language.

#### PW workload
- `cargo make graph-lprq-pw`
- `cargo make graph-msq-pw`

#### Workloads with congestion
- `cargo make graph-msq-congestion-pw`
- `cargo make graph-lprq-congestion-pw`
- `cargo make graph-lprq-congestion-1-1`
- `cargo make graph-lprq-congestion-2-1`

#### MPMC workload
- `cargo make graph-lprq-1-1`
- `cargo make graph-lprq-2-1`



