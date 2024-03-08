use rand::rngs::ThreadRng;
use rand::Rng;
use std::arch::asm;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::thread;

use crate::shared_queue::SharedQueue;

const BASE: u32 = 10;

pub fn benchmark<Q>(nproducer: u32, nconsumer: u32, logn: u32, queue: Q)
where
    Q: SharedQueue<i32> + Clone + Send + 'static,
{
    let stop_flag = Arc::new(AtomicBool::new(false));

    let mut producer_handles = vec![];
    let mut consumer_handles = vec![];

    // Calculate number of operations
    let nops = BASE.pow(logn);
    let tops = (nops / nproducer) as i32;

    // Producers
    for _ in 0..nproducer {
        let mut queue_handle = queue.clone();

        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();

            /* The LPRQ paper does this differently, they:
            - rely on stop_flag for producers also, thus relying on
              loadbalancing to limit on the amount of enqueue operations
            - load balances when first segment PRQ reaches 70%
            - Benchmark runs for 1000ms, then stops */
            for j in 0..tops {
                queue_handle.enqueue(j);
                delay_exec(&mut rng);
            }
        });
        producer_handles.push(handle);
    }

    // Consumers
    for _ in 0..nconsumer {
        let mut queue_handle = queue.clone();
        let stop_flag_handle = stop_flag.clone();

        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();

            loop {
                // TODO: Seperate successful and failed dequeues (as paper)
                match queue_handle.dequeue() {
                    Some(_) => continue,
                    None => {
                        if stop_flag_handle.load(SeqCst) {
                            break;
                        }
                    }
                }
                delay_exec(&mut rng);
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
