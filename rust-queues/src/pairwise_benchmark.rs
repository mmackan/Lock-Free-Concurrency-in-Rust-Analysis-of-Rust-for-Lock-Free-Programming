use rand::rngs::ThreadRng;
use rand::Rng;
use std::arch::asm;
use std::thread;

use crate::shared_queue::SharedQueue;

use core_affinity;

const BASE: usize = 10;

pub fn benchmark<Q>(nprocs: usize, logn: usize, even_cores_only: bool, queue: Q)
where
    Q: SharedQueue<i32> + Clone + Send + 'static,
{
    // Calculate number of operations
    let nops = BASE.pow(logn as u32);
    let tops = nops / nprocs;

    let binding = core_affinity::get_core_ids().unwrap();
    let mut core_ids = binding.iter();

    let mut handles = vec![];

    for _i in 0..nprocs {
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

            for j in 0..tops {
                queue_handle.enqueue(j.try_into().unwrap());
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
