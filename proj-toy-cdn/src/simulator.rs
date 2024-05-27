use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use ahash::AHashMap;
use proj_cache_sim::{cache::Cache, simulator::RequestResult};
use proj_models::{RequestId, TimeUnit};
use proj_net::RemoteChannel;
use tracing::error;

struct LocalState<C> {
    cache: C,
    requests_in_progress: AHashMap<u64, Vec<TimeUnit>>,
}

/// We have two tasks running simultaneously:
/// - The request sending task:
///     - Replay the provided `requests` in order and record the current timestamp, and put it to `requests_in_progress`.
///     - If the request is already in the cache, directly forward the request to the completion handling task.
///         - otherwise, send the request to the origin
/// - The completion handling task (main task)
///     - On receiving a completion of a request id, record the current timestamp. Go to `requests_in_progress`, find all corresponding requests, and put to `request_results`.
///     - Update the cache.
pub async fn run_simulation<C, I>(
    cache: C,
    requests: I,
    origin: &RemoteChannel,
) -> Vec<RequestResult>
where
    C: Cache<RequestId, ()> + Send + 'static,
    I: IntoIterator<Item = RequestId>,
{
    // Requests that are currently in fetching state.
    let state = Arc::new(Mutex::new(LocalState {
        cache,
        requests_in_progress: AHashMap::new(),
    }));

    // just store the requests in memory for better simulation
    let requests = requests.into_iter().collect::<Vec<_>>();
    let requests_count = requests.len();

    let start_of_time = Instant::now();

    let (completion_sender, mut completion_receiver) =
        tokio::sync::mpsc::unbounded_channel::<RequestId>();

    let request_sending_handle = {
        let state = state.clone();
        let completion_sender = completion_sender.clone();
        let origin = origin.clone();
        tokio::spawn(async move {
            for request in requests {
                let timestamp = start_of_time.elapsed().as_nanos() as TimeUnit;

                let (first_request, cache_hit) = {
                    let mut state = state.lock().unwrap();
                    // add the request to the in-progress list
                    let requests_in_progress =
                        state.requests_in_progress.entry(request).or_default();
                    requests_in_progress.push(timestamp);
                    let first_request = requests_in_progress.len() == 1;
                    let cache_hit = state.cache.get(&request, timestamp).is_some();
                    (first_request, cache_hit)
                };
                if cache_hit {
                    // the request is immediately fulfilled.
                    completion_sender
                        .send(request)
                        .expect("completion receiver is completed but got a message");
                } else {
                    if first_request {
                        // the request is not in the cache, and is not in transit, so we need to send it

                        origin
                            .send(request.into())
                            .await
                            .expect("trying to send a message but origin connection is closed");
                    }
                }
            }
        })
    };

    // a proxy task to direct the received message to the completion handling task
    let proxy_handle = {
        let origin = origin.clone();
        let completion_sender = completion_sender.clone();
        tokio::spawn(async move {
            loop {
                let request = if let Ok(r) = origin.recv().await {
                    r
                } else {
                    break;
                };
                completion_sender
                    .send(request.into())
                    .expect("got a message but the completion receiver is already completed");
            }
        })
    };

    // the completion handling task
    let mut request_results = Vec::with_capacity(requests_count);
    while let Some(request) = completion_receiver.recv().await {
        let timestamp = start_of_time.elapsed().as_nanos() as TimeUnit;
        let mut state = state.lock().unwrap();
        let pending_requests = state
            .requests_in_progress
            .remove(&request)
            .expect("received an unexpected request response");
        let results = pending_requests
            .into_iter()
            .map(|req_timestamp| RequestResult {
                key: request,
                request_timestamp: req_timestamp,
                completion_timestamp: timestamp,
            });
        request_results.extend(results);

        state.cache.write(request, (), timestamp);
        if request_results.len() == requests_count {
            break;
        }
    }
    let _ = (completion_sender, completion_receiver); // drop the completion receiver
    if let Err(e) = request_sending_handle.await {
        error!("request sending task failed: {:?}", e);
    }
    if let Err(e) = proxy_handle.await {
        error!("proxy task failed: {:?}", e);
    }
    request_results
}
