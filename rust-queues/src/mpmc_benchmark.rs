use std::arch::asm;
use std::thread;
use rand::rngs::ThreadRng;
use rand::Rng;
use crossbeam_channel::unbounded;

use crate::shared_queue::SharedQueue;

const BASE: u32 = 10;

pub fn benchmark<Q>(nproducer: u32, nconsumer: u32, logn: u32, queue: Q)
    where Q: SharedQueue<i32> + Clone + Send + 'static {

    let mut producer_handles = vec![];
    let mut consumer_handles = vec![];

    // Calculate number of operations
    let nops = BASE.pow(logn);
    let tops = (nops / nproducer) as i32;
    
    // Unbounded channel
    let (tx, rx) = unbounded();

    // Consumers
    for _ in 0..nconsumer {
        let mut queue_handle = queue.clone();
        let rx_handle = rx.clone();

        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();

            loop {
                match rx_handle.recv() {
                    Ok(_) => {
                        queue_handle.dequeue().unwrap();
                        delay_exec(&mut rng);
                    },
                    // Channel is closed
                    Err(_) => break,
                }
            }
        });
        consumer_handles.push(handle);
    }

    // Producers
    for _ in 0..nproducer {       
        let mut queue_handle = queue.clone();
        let tx_handle = tx.clone();

        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();

            for j in 0..tops {
                queue_handle.enqueue(j);
                
                // Notify consumers
                tx_handle.send(1).unwrap();

                delay_exec(&mut rng);
            }
        });
        producer_handles.push(handle);
    }

    for p in producer_handles {
        p.join().unwrap();
    }

    // Producers are finished, close connection
    drop(tx);

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
