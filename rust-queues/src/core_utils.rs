use core_affinity::{get_core_ids, CoreId};

pub fn get_cores(even_only: bool, mixed_hyperthreads: bool) -> Vec<CoreId> {
    let mut cores = get_core_ids().unwrap();

    println!("{:?}", cores);

    if even_only {
        cores.retain(|&x| x.id % 2 == 0);
    }

    // This logic assumes that the upper half of the cpu's are hyperthreads
    // This is true on most inter processors without efficiency cores

    if mixed_hyperthreads {
        // If there are not an even amount of cores left this logic will fail so panic
        assert!(cores.len() % 2 == 0);
        let half_len = cores.len()/2;
        for i in (1..(half_len+1)).step_by(2) {
            cores.swap(i, i + half_len);
        }
    }

    return cores; 
}
