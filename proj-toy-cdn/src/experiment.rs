use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use ahash::AHashMap;
use proj_cache_sim::{
    cache::Cache,
    simulator::{run_simulation, RequestResult},
};
use proj_models::{RequestEvent, RequestId, TimeUnit};
use proj_net::{
    msg::{CdnRequestMessage, OriginResponseMessage},
    RemoteChannel,
};
use tracing::{error, info};

struct LocalState<C> {
    cache: C,
    requests_in_progress: AHashMap<u64, Vec<TimeUnit>>,
}

pub struct Clock {
    start_time: Instant,
}

impl Clock {
    pub fn tick() -> Self {
        let start_time = Instant::now();
        Self { start_time }
    }

    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    pub async fn wait_until_next_available(self, irt_ns: u64) {
        // tokio::sleep is not accurate enough for capturing microsecond-level time.
        while (self.start_time.elapsed().as_nanos() as u64) < irt_ns {
            tokio::task::yield_now().await;
        }
    }
}

/// - `cache`: An empty cache to use.
///
/// We have two tasks running simultaneously:
/// - The request sending task:
///     - Replay the provided `requests` in order and record the current timestamp, and put it to `requests_in_progress`.
///     - If the request is already in the cache, directly forward the request to the completion handling task.
///         - otherwise, send the request to the origin
/// - The completion handling task (main task)
///     - On receiving a completion of a request id, record the current timestamp. Go to `requests_in_progress`, find all corresponding requests, and put to `request_results`.
///     - Update the cache.
/// - The proxy task:
///    - Direct the received message to the completion handling task.
pub async fn run_cdn_experiment<C, I>(
    mut cache: C,
    requests: I,
    origin: &RemoteChannel<CdnRequestMessage, OriginResponseMessage>,
    warmup: usize,
    miss_latency_in_warmup: TimeUnit,
    irt_ns: u64,
) -> (Vec<RequestResult>, Vec<TimeUnit>, Vec<TimeUnit>)
where
    C: Cache<RequestId, ()> + Send + 'static,
    I: IntoIterator<Item = RequestId>,
{
    // warmup requests are not actually sent to the origin and is only used to warm up the cache.
    let mut requests = requests.into_iter();
    info!("Running {} warmup requests", warmup);
    let last_event = tokio::task::block_in_place(|| {
        let warmup_requests =
            requests
                .by_ref()
                .take(warmup)
                .enumerate()
                .map(|(i, r)| RequestEvent {
                    key: r,
                    timestamp: i as u64 * irt_ns,
                });
        run_simulation(&mut cache, warmup_requests, miss_latency_in_warmup).last_event_timestamp
    });
    info!("Warmup requests completed, starting the main simulation");

    // Requests that are currently in fetching state.
    let state = Arc::new(Mutex::new(LocalState {
        cache,
        requests_in_progress: AHashMap::new(),
    }));
    // just store the requests in memory for better simulation
    let requests = requests.into_iter().collect::<Vec<_>>();
    let requests_count = requests.len();

    // for simplicity, we assume the trace resumes after all warmup requests are fulfilled.
    let start_of_time =
        Instant::now() - Duration::from_nanos(last_event as u64) - Duration::from_nanos(irt_ns);

    let (completion_sender, mut completion_receiver) =
        tokio::sync::mpsc::unbounded_channel::<RequestId>();

    // the completion handling task
    let completion_handle = {
        let state = state.clone();

        tokio::spawn(async move {
            let mut request_results = Vec::with_capacity(requests_count);
            let mut last_progress_timestamp = Instant::now();
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

                // report progress
                if last_progress_timestamp.elapsed() > Duration::from_secs(3) {
                    info!(
                        "{}/{} requests fulfilled",
                        request_results.len(),
                        requests_count
                    );
                    last_progress_timestamp = Instant::now();
                }

                if request_results.len() == requests_count {
                    break;
                }
            }
            request_results
        })
    };

    // the request sending task
    let request_sending_handle = {
        let state = state.clone();
        let completion_sender = completion_sender.clone();
        let origin = origin.clone();

        tokio::spawn(async move {
            let mut origin_request_timestamps = Vec::new();
            for request in requests {
                let clock = Clock::tick();
                // let timestamp = start_of_time.elapsed().as_nanos() as TimeUnit;
                let timestamp = (clock.start_time() - start_of_time).as_nanos() as TimeUnit;

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
                        origin_request_timestamps.push(timestamp);
                        origin.send(request.into()).await;
                    }
                }
                clock.wait_until_next_available(irt_ns).await;
            }
            origin_request_timestamps
        })
    };

    // a proxy task to direct the received message to the completion handling task
    let proxy_handle = {
        let origin = origin.clone();
        let completion_sender = completion_sender.clone();
        tokio::spawn(async move {
            let mut origin_response_timestamps = Vec::new();
            loop {
                let response = if let Ok(r) = origin.recv().await {
                    r
                } else {
                    break;
                };
                let timestamp = (Instant::now() - start_of_time).as_nanos() as TimeUnit;
                origin_response_timestamps.push(timestamp);
                completion_sender
                    .send(response.key)
                    .expect("got a message but the completion receiver is already completed");
            }
            origin_response_timestamps
        })
    };

    let _ = completion_sender; // drop the completion receiver
    let origin_request_timestamps = request_sending_handle
        .await
        .expect("request sending task failed");
    let origin_response_timestamps = proxy_handle.await.expect("proxy task failed");
    let request_results = completion_handle
        .await
        .expect("completion handling task failed");
    info!("All requests fulfilled. Experiment completed.");
    (
        request_results,
        origin_request_timestamps,
        origin_response_timestamps,
    )
}
