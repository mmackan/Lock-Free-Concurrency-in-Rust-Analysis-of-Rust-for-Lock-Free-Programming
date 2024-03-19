#![feature(strict_provenance, thread_id_value)]
#![warn(fuzzy_provenance_casts)]

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub mod ms_queue;

pub mod benchmark_utils;
pub mod lprq;
pub mod mpmc_benchmark;
pub mod pairwise_benchmark;
pub mod shared_queue;
