use rand::rngs::ThreadRng;
use rand::Rng;
use std::arch::asm;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::{thread, time};

use crate::shared_queue::SharedQueue;

use core_affinity;

const BASE: usize = 10;

pub fn benchmark<Q>(
    nproducer: usize,
    nconsumer: usize,
    logn: usize,
    even_cores_only: bool,
    congestion_factor: f32,
    queue: Q,
) where
    Q: SharedQueue<i32> + Clone + Send + 'static,
{
    let stop_flag = Arc::new(AtomicBool::new(false));

    let mut producer_handles = vec![];
    let mut consumer_handles = vec![];

    // Calculate number of operations
    let nops = BASE.pow(logn as u32);
    let tops = nops / nproducer;

    let binding = core_affinity::get_core_ids().unwrap();
    let mut core_ids = binding.iter();

    // Producers
    for _ in 0..nproducer {
        let mut queue_handle = queue.clone();
        let core_id = core_ids
            .next()
            .expect("Ran out of cores! Maybe used fewer threads")
            .clone();
        if even_cores_only {
            // Skip a core so we only use even ones, for use on the server
            let _ = core_ids.next();
        }

        let handle = thread::spawn(move || {
            let _ = core_affinity::set_for_current(core_id);
            let mut rng = rand::thread_rng();

            /* The LPRQ paper does this differently, they:
            - rely on stop_flag for producers also, thus relying on
              loadbalancing to limit on the amount of enqueue operations
            - load balances when first segment PRQ reaches 70%
            - Benchmark runs for 1000ms, then stops */
            for j in 0..tops {
                queue_handle.enqueue(j.try_into().unwrap());
                if rng.gen_range(0.0..1.0) > congestion_factor {
                    delay_exec();
                }
            }
        });
        producer_handles.push(handle);
    }

    // Consumers
    for _ in 0..nconsumer {
        let mut queue_handle = queue.clone();
        let stop_flag_handle = stop_flag.clone();
        let core_id = core_ids
            .next()
            .expect("Ran out of cores! Maybe used fewer threads")
            .clone();
        if even_cores_only {
            // Skip a core so we only use even ones, for use on the server
            let _ = core_ids.next();
        }

        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let _ = core_affinity::set_for_current(core_id);
            let mut backoff = 1;

            loop {
                match queue_handle.dequeue() {
                    Some(_) => {}
                    None => {
                        if stop_flag_handle.load(SeqCst) {
                            break;
                        }
                        backoff = backoff * 2;
                        for _ in 0..backoff {
                            delay_exec();
                        }

                    }
                }
                if rng.gen_range(0.0..1.0) > congestion_factor {
                    delay_exec();
                }
            }
        });
        consumer_handles.push(handle);
    }

    for p in producer_handles {
        p.join().unwrap();
    }

    // Notify consumers no more elements will be enqueued
    stop_flag.store(true, SeqCst);

    for c in consumer_handles {
        c.join().unwrap();
    }
}

fn delay_exec() {
    for _ in 0..100 {
        #[cfg(not(miri))]
        unsafe {
            asm!("nop");
        }
    }
}
