use std::arch::asm;
use std::thread;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::shared_queue::SharedQueue;

const BASE: u32 = 10;

pub fn benchmark<Q>(nprocs: u32, logn: u32, queue: Q) 
    where Q: SharedQueue<i32> + Clone + Send + 'static{

    let mut handles = vec![];
    
    // Calculate number of operations
    let nops = BASE.pow(logn);
    let tops = (nops / nprocs) as i32;
    
    for _ in 0..nprocs {       
        let mut queue_handle = queue.clone();

        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();

            for j in 0..tops {
                queue_handle.enqueue(j);
                delay_exec(&mut rng);

                queue_handle.dequeue();
                delay_exec(&mut rng);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
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