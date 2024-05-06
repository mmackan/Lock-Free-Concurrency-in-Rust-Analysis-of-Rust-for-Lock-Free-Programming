use core_affinity::{get_core_ids, CoreId};

pub fn get_cores(even_only: bool) -> Vec<CoreId> {
    let mut cores = get_core_ids().unwrap();

    if even_only {
        cores.retain(|&x| x.id % 2 == 0);
    }

    cores
}
