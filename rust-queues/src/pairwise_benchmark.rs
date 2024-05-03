use rand::Rng;
use std::arch::asm;
use std::thread;

use crate::shared_queue::SharedQueue;
use crate::core_utils;

use core_affinity;

const BASE: usize = 10;

pub fn benchmark<Q>(
    nprocs: usize,
    logn: usize,
    even_cores_only: bool,
    congestion_factor: f32,
    queue: Q,
) where
    Q: SharedQueue<i32> + Clone + Send + 'static,
{
    // Calculate number of operations
    let nops = BASE.pow(logn as u32);
    let tops = nops / nprocs;

    let binding = core_utils::get_cores(even_cores_only, true);
    let mut core_ids = binding.iter();

    let mut handles = vec![];

    for _i in 0..nprocs {
        let mut queue_handle = queue.clone();
        let core_id = core_ids
            .next()
            .expect("Ran out of cores! Maybe used fewer threads")
            .clone();
        let handle = thread::spawn(move || {
            let _ = core_affinity::set_for_current(core_id);
            let mut rng = rand::thread_rng();

            for j in 0..tops {
                queue_handle.enqueue((&(j as i32)) as *const _);
                if rng.gen_range(0.0..1.0) > congestion_factor {
                    delay_exec();
                }

                queue_handle.dequeue();
                if rng.gen_range(0.0..1.0) > congestion_factor {
                    delay_exec();
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
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
