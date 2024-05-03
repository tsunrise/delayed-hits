use std::{collections::VecDeque, iter::Peekable};

use ahash::AHashMap;

use crate::{
    cache::{Cache, ObjectId},
    types::Timestamp,
    verbose,
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct RequestResult<K> {
    pub key: K,
    pub request_timestamp: Timestamp,
    pub completion_timestamp: Timestamp,
}

#[derive(Debug)]
enum Event<K> {
    Request(K, Timestamp),
    Completion(K, Timestamp),
    End,
}

fn next_event<K, I>(
    requests: &mut Peekable<I>,
    future_completion: &mut VecDeque<(K, Timestamp)>,
    last_request_timestamp: &mut Timestamp,
) -> Event<K>
where
    K: ObjectId,
    I: Iterator<Item = (K, Timestamp)>,
{
    // get the earlist among the next request and the next completion
    // if the timestamp of the next request is the same as the next completion, we should process the request first.
    // also, for request, use max(last_request_timestamp, request_timestamp) as the timestamp of the request, and update last_request_timestamp.
    // for completion, just pop it from the queue.
    // if there is no request or completion, return None.

    let next_request = requests.peek().map(|(key, timestamp)| {
        if timestamp < last_request_timestamp {
            (key, *last_request_timestamp)
        } else {
            (key, *timestamp)
        }
    });

    let next_completion = future_completion.front();

    let choose_request = match (next_request, next_completion) {
        (Some((_, req_timestamp)), Some((_, com_timestamp))) if req_timestamp <= *com_timestamp => {
            true
        }
        (Some(_), None) => true,
        _ => false,
    };

    if choose_request {
        let (key, timestamp) = requests.next().unwrap();
        *last_request_timestamp = timestamp;
        Event::Request(key, timestamp)
    } else {
        future_completion
            .pop_front()
            .map(|(key, timestamp)| Event::Completion(key, timestamp))
            .unwrap_or(Event::End)
    }
}

/// Run a delay-aware cache simulation, given a `caches.len()`-Way set associative cache and a sequence of requests. Return a vector of `RequestResult`.
/// - `miss_penalty` is the time in nanoseconds it takes to fetch a missed request from the backing store.
pub fn run_simulation<K, C, I>(
    caches: &mut [C],
    requests: I,
    miss_penalty: usize,
) -> Vec<RequestResult<K>>
where
    K: ObjectId,
    C: Cache<K>,
    I: IntoIterator<Item = (K, Timestamp)>,
{
    // Requests that are currently in fetching state.
    let mut requests_in_progress: AHashMap<K, Vec<Timestamp>> = AHashMap::new();
    // A monotonic queue of completion timestamps of requests.
    let mut future_completions: VecDeque<(K, Timestamp)> = VecDeque::new();
    // A vector of request results.
    let mut results = Vec::new();

    // In case the request are occasionally out of order, we use timestamp = max(last_request_timestamp, request_timestamp) as the timestamp of the request.
    let mut last_request_timestamp = 0;
    let mut requests = requests.into_iter().peekable();

    loop {
        let event = next_event(
            &mut requests,
            &mut future_completions,
            &mut last_request_timestamp,
        );
        verbose!("{:?}", event);
        match event {
            Event::End => {
                break;
            }
            Event::Request(key, timestamp) => {
                let cache = key.get_cache(caches);
                cache.report_access(key.clone(), timestamp);
                if cache.contains(&key) {
                    // the request is immediately fulfilled.
                    cache.write(key.clone(), timestamp);
                    results.push(RequestResult {
                        key,
                        request_timestamp: timestamp,
                        completion_timestamp: timestamp,
                    });
                } else {
                    // check if the request is already in progress.
                    if !requests_in_progress.contains_key(&key) {
                        requests_in_progress.insert(key.clone(), Vec::new());
                        future_completions
                            .push_back((key.clone(), timestamp + miss_penalty as u64));
                    }
                    requests_in_progress.get_mut(&key).unwrap().push(timestamp);
                }
            }
            Event::Completion(key, timestamp) => {
                let cache = key.get_cache(caches);
                debug_assert!(!cache.contains(&key), "{key:?} should not in the cache until the completion of the request, but it is.");
                let pending_requests = requests_in_progress
                    .remove(&key)
                    .expect("pending requests for {key:?} should exist.");
                debug_assert!(
                    pending_requests.len() > 0,
                    "pending requests for {key:?} should not be empty."
                );

                cache.write(key.clone(), timestamp);
                pending_requests.into_iter().for_each(|req_timestamp| {
                    results.push(RequestResult {
                        key: key.clone(),
                        request_timestamp: req_timestamp,
                        completion_timestamp: timestamp,
                    });
                });
            }
        }
    }

    results
}
