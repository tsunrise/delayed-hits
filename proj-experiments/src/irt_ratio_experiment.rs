use proj_cache_sim::trace_gen::generate_synthetic_traces;
use proj_models::RequestEvents;
use rand::Rng;

use crate::run_experiment;

pub fn irt_ratio_experiment<R: Rng>(
    rng: &mut R,
    expected_rearrival_time: usize,
    num_unique_objects: usize,
    num_requests: usize,
    cache_counts: usize,
    cache_capacity: usize,
    miss_latency: u64,
) {
    let events = generate_synthetic_traces(
        expected_rearrival_time,
        num_unique_objects,
        num_requests,
        rng,
    );

    let median_rearrive_interval = proj_cache_sim::heuristics::median_rearrive_interval(&events);
    let median_irt = proj_cache_sim::heuristics::irt(&events);
    let irt_ratio = median_rearrive_interval as f64 / median_irt;

    println!("median_rearrive_interval: {}", median_rearrive_interval);
    println!("median_irt: {}", median_irt);
    println!("irt_ratio: {}", irt_ratio);

    let result = run_experiment(
        RequestEvents::new(events),
        cache_counts,
        cache_capacity,
        miss_latency,
    );

    println!("result: {}", result);
}
