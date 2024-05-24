use proj_models::RequestEvent;

/// Make the timestamp essentially discrete, as used by the original paper.
/// - `constant_arrival`: whether to make packets arrive at a constant rate (the average inter-arrival time) or not.
pub fn downsample_events<T>(
    events: Vec<RequestEvent<T>>,
    constant_arrival: bool,
    irt: Option<u64>,
) -> Vec<RequestEvent<T>> {
    // calculate the average inter-arrival time
    let intervals_avg = if let Some(irt) = irt {
        irt
    } else {
        (events
            .windows(2)
            .map(|pair| {
                assert!(pair[0].timestamp <= pair[1].timestamp);
                (pair[1].timestamp - pair[0].timestamp) as u128
            })
            .sum::<u128>() as f64
            / (events.len() - 1) as f64)
            .round() as u64
    };
    println!("average inter-arrival time: {}", intervals_avg);
    if constant_arrival {
        return events
            .into_iter()
            .enumerate()
            .map(|(i, event)| RequestEvent {
                key: event.key,
                timestamp: i as u64 * intervals_avg,
            })
            .collect();
    }
    // downsample the events: the timestamp is now the multiple of the average inter-arrival time closet to the original timestamp
    events
        .into_iter()
        .map(|event| {
            let timestamp =
                (event.timestamp as f64 / intervals_avg as f64).round() as u64 * intervals_avg;
            RequestEvent {
                key: event.key,
                timestamp,
            }
        })
        .collect()
}
