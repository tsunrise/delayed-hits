//! Generate synthetic traces

use std::{cmp::Reverse, collections::BinaryHeap};

use proj_models::RequestEvent;
use rand::Rng;
use rand_distr::{Distribution as _, Exp};

/// expected rearrival time / irt $\approx$ num_unique_objects
pub fn generate_synthetic_traces<R: Rng>(
    expected_rearrival_time: usize,
    num_unique_objects: usize,
    num_requests: usize,
    rng: &mut R,
) -> Vec<RequestEvent<usize>> {
    let exp = Exp::new(1.0 / expected_rearrival_time as f64).unwrap();

    let mut events_next_timestamps = (0..num_unique_objects)
        .map(|idx| {
            (
                Reverse(rng.gen_range(0..expected_rearrival_time) as u64),
                idx,
            )
        })
        .collect::<BinaryHeap<_>>();

    let mut events = Vec::with_capacity(num_requests);
    for _ in 0..num_requests {
        let (Reverse(timestamp), object_id) = events_next_timestamps.pop().unwrap();
        events.push(RequestEvent {
            key: object_id,
            timestamp: timestamp as u64,
        });
        events_next_timestamps.push((
            Reverse((timestamp as f64 + exp.sample(rng)) as u64),
            object_id,
        ));
    }
    events
}

#[cfg(test)]
mod tests {

    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;

    use crate::heuristics::{irt, maximum_active_objects, median_rearrive_interval};

    use super::*;

    #[test]
    fn test_generate_traces() {
        let mut rng = XorShiftRng::seed_from_u64(244);
        for ratio in [5, 9, 17, 28] {
            let events = generate_synthetic_traces(10000, ratio, 5000, &mut rng);
            let median_rearrive_interval = median_rearrive_interval(&events);
            let irt = irt(&events);
            let actual_ratio = median_rearrive_interval as f64 / irt;
            let maximum_active_objects = maximum_active_objects(&events);

            println!("median_rearrive_interval: {}", median_rearrive_interval);
            println!("median_irt: {}", irt);
            println!("ratio: {}", median_rearrive_interval as f64 / irt);
            println!("maximum_active_objects: {}", maximum_active_objects);

            assert!(f64::abs(actual_ratio - ratio as f64) < 2.0);
        }
    }
}
