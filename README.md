# Lock-Free Concurrency in Rust
## Analysis of Rust for Lock-Free Programming
This is all the code and raw results for our master's thesis Lock-Free Concurrency in Rust: Analysis of Rust for Lock-Free Programming.
A link to the thesis will be provided here upon final publication.
Third-party code is included in accordance with their respective licenses which can be found in their respective folders.


`cd` to the `rust-queues` folder to execute `cargo make` commands.


To benchmark and gather all comparison graphs, simply build and run:
- `cargo make build`
- `cargo make run`
	- **NOTE:** estimated time is 15 hours

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

- MSQ supports only PW and compares Rust vs. C using hazard pointer as memory reclamation.
- LPRQ supports both PW and MPMC and compares Rust vs. C++. Additional energy and memory benchmarks are included for the LPRQ.
- The C++ reference utilizes only hazard pointers while Rust includes three different memory reclamation techniques. In more detail:
	- LPRQ's PW and MPMC 1:1 benchmarks include C++ and all three Rust versions:
		- Hazardpointer (Hazp)
		- Epoch-based (Epoch)
		- Reference counting (Aarc)
	- LPRQ MPMC 2:1 include C++ but only the two Rust versions:
		- Hazp
		- Aarc

### All benchmarks
- `cargo make benchmark-msq`
- `cargo make benchmark-lprq`
- `cargo make benchmark-rust`
	- Benchmark all the Rust versions (not leaking version)

### Individual Rust versions
- `cargo make benchmark-hazp`
- `cargo make benchmark-epoch`
- `cargo make benchmark-arc`
- `cargo make benchmark-leaking`
	- Extra Rust version of LPRQ with leaking memory for those interested.

### Energy and Memory benchmarks
Benchmark energy consumption using perf that utilizes RAPL, and memusage for memory usage as well as perf for memory access patterns.
- `cargo make benchmark-energy-lprq`
- `cargo make benchmark-memory-lprq`





***

## Plotting graphs and creating tables (final results)
After benchmarks have been successfully executed these commands will generate graphs that show the comparison between the two languages.

### All graphs and tables
- `cargo make create-tables`
- `cargo make graph-all`
	- `cargo make graph-msq`
	- `cargo make graph-lprq`



If you are interested in seeing all commands and finding commands for specific benchmarks, see the `Makefile.toml` file. Each command is simply run with `cargo make`

