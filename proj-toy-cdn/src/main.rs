#[allow(unused)]
mod experiment;

use std::{str::FromStr, time::Instant};

use clap::{Parser as _, Subcommand};
use clap_derive::Parser;
use experiment::{run_cdn_experiment, Clock};
use proj_cache_sim::{
    cache::{construct_k_way_cache, lru::LRU, lru_mad::LRUMinAD, Cache},
    get_time_string,
    io::load_events_file,
    simulator::compute_statistics,
};
use proj_models::{RequestId, TimeUnit};
use proj_net::{
    msg::{CdnRequestMessage, OriginResponseMessage},
    ConnectionMode, RemoteChannel,
};
use tracing::info;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CacheType {
    LRU,
    LRUMinAD,
}

impl FromStr for CacheType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lru" => Ok(Self::LRU),
            "lru-mad" => Ok(Self::LRUMinAD),
            _ => Err(format!("Unknown cache type: {}", s)),
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    Bench {
        #[clap(
            long,
            short = 'r',
            default_value = "100",
            help = "number of measure rounds"
        )]
        num_measure_rounds: usize,
    },
    Experiment {
        #[clap(required = true, help = "Path to the processed events file")]
        event_path: String,
        #[clap(
            long,
            short = 'k',
            help = "number of caches (k for k-way set-associative cache)"
        )]
        cache_count: usize,
        #[clap(long, short = 'c', help = "cache capacity in each cache")]
        cache_capacity: usize,
        #[clap(
            long,
            short = 'w',
            help = "number of warmup requests to warm the cache before sending actual requests. Those requests are not sent to the internet and are not counted."
        )]
        warmup: usize,
        #[clap(
            long,
            short = 'm',
            help = "number of actual requests to process after the warmup"
        )]
        num_requests: usize,
        #[clap(long, short = 't', default_value = "lru", help = "cache type")]
        cache_type: CacheType,
        #[clap(long, short = 'l', help = "estimated miss latency for warmup, with unit (e.g. 300ns, 2ms)", value_parser = proj_cache_sim::parse_time_unit)]
        miss_latency: u64,
        #[clap(
            long,
            short = 'i',
            help = "inter-request time, with unit (e.g. 300ns, 2ms)",
            value_parser = proj_cache_sim::parse_time_unit,
            default_value = "1us"
        )]
        irt: TimeUnit,
    },
}

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Commands,
    #[clap(long, short = 'c', value_parser = proj_net::parse_connection_mode, help = "use <ip_addr>:<port> for client, <port> for server")]
    conn: ConnectionMode,
    #[clap(long, short = 'n', default_value = "8", help = "number of connections")]
    num_connections: usize,

    #[clap(
        long,
        short = 'b',
        default_value = "4",
        help = "number of messages buffered in the channel"
    )]
    num_msg_buffered: usize,
}

async fn measurement(
    chan: RemoteChannel<CdnRequestMessage, OriginResponseMessage>,
    num_measure_rounds: usize,
) {
    info!("Measuring average latency...");
    let start_of_time = Instant::now();
    // subscribe to responses
    let ends_handle = {
        let chan = chan.clone();
        tokio::spawn(async move {
            let mut ends = Vec::new();
            while let Ok(response) = chan.recv().await {
                ends.push((std::time::Instant::now(), response.key));
            }
            ends
        })
    };

    let mut starts = Vec::new();
    let mut handles = Vec::new();
    for idx in 0..num_measure_rounds {
        let clock = Clock::tick();
        starts.push(std::time::Instant::now());
        handles.push(chan.send(CdnRequestMessage::new(idx as u64)).await);
        clock.wait_until_next_available(1000).await;
    }
    {
        let chan = chan.clone();
        tokio::spawn(async move {
            for handle in handles {
                assert!(handle.wait().await);
                chan.flush().await;
            }
            // chan.flush().await;
            chan.close_send().await;
        });
    }

    let mut ends = ends_handle.await.unwrap();
    ends.sort_by_key(|(_, key)| *key);

    let starts_ns = starts
        .iter()
        .map(|start| (*start - start_of_time).as_nanos() as u64)
        .collect::<Vec<_>>();
    let ends_ns = ends
        .iter()
        .map(|(end, _)| (*end - start_of_time).as_nanos() as u64)
        .collect::<Vec<_>>();

    let average_i_request_t = starts_ns.windows(2).map(|w| w[1] - w[0]).sum::<u64>() as f64
        / (num_measure_rounds as u64 - 1) as f64;
    let average_i_response_t = ends_ns.windows(2).map(|w| w[1] - w[0]).sum::<u64>() as f64
        / (num_measure_rounds as u64 - 1) as f64;
    let average_delay = ends_ns
        .iter()
        .zip(starts_ns.iter())
        .map(|(e, s)| e - s)
        .sum::<u64>() as f64
        / num_measure_rounds as f64;

    info!("Average inter-request time: {} ns", average_i_request_t);
    info!("Average inter-response time: {} ns", average_i_response_t);
    info!(
        "Average delay: {} ns = {} ms",
        average_delay,
        average_delay / 1_000_000.0
    );

    npy::to_file("request_timestamps.npy", starts_ns).unwrap();
    npy::to_file("response_timestamps.npy", ends_ns).unwrap();
}

async fn experiment_on_cache<C>(
    cache: C,
    chan: RemoteChannel<CdnRequestMessage, OriginResponseMessage>,
    event_path: String,
    warmup: usize,
    num_requests: usize,
    estimated_miss_latency: u64,
    irt_ns: u64,
) where
    C: Cache<RequestId, ()> + Send + 'static,
{
    let events = load_events_file(&event_path)
        .take(warmup + num_requests)
        .map(|r| r.key);
    let (results, origin_send_timestamps, origin_response_timestamps) =
        run_cdn_experiment(cache, events, &chan, warmup, estimated_miss_latency, irt_ns).await;

    let stats = compute_statistics(&results);
    info!(
        "Average latency: {}",
        get_time_string(stats.average_latency as u128)
    );
    info!("Saving Results...");
    let request_starts = results
        .iter()
        .map(|r| r.request_timestamp)
        .collect::<Vec<_>>();
    let request_ends = results
        .iter()
        .map(|r| r.completion_timestamp)
        .collect::<Vec<_>>();
    npy::to_file(format!("request_starts_{}.npy", C::NAME), request_starts).unwrap();
    npy::to_file(format!("request_ends_{}.npy", C::NAME), request_ends).unwrap();
    npy::to_file(
        format!("origin_send_timestamps_{}.npy", C::NAME),
        origin_send_timestamps,
    )
    .unwrap();
    npy::to_file(
        format!("origin_response_timestamps_{}.npy", C::NAME),
        origin_response_timestamps,
    )
    .unwrap();
}

async fn experiment(
    chan: RemoteChannel<CdnRequestMessage, OriginResponseMessage>,
    cache_type: CacheType,
    event_path: String,
    cache_count: usize,
    cache_capacity: usize,
    warmup: usize,
    num_requests: usize,
    estimated_miss_latency_ns: TimeUnit,
    irt_ns: TimeUnit,
) {
    match cache_type {
        CacheType::LRU => {
            let cache = construct_k_way_cache(cache_count, |_| LRU::new(cache_capacity));
            experiment_on_cache(
                cache,
                chan,
                event_path,
                warmup,
                num_requests,
                estimated_miss_latency_ns,
                irt_ns,
            )
            .await
        }
        CacheType::LRUMinAD => {
            let cache = construct_k_way_cache(cache_count, |_| {
                LRUMinAD::new(cache_capacity, estimated_miss_latency_ns)
            });
            experiment_on_cache(
                cache,
                chan,
                event_path,
                warmup,
                num_requests,
                estimated_miss_latency_ns,
                irt_ns,
            )
            .await
        }
    };
}

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let args = Args::parse();
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
        info!("{:?}", args);

        let chan = RemoteChannel::<CdnRequestMessage, OriginResponseMessage>::new(
            args.conn,
            args.num_connections,
            args.num_msg_buffered,
        )
        .await;
        match args.command {
            Commands::Bench { num_measure_rounds } => {
                measurement(chan, num_measure_rounds).await;
            }
            Commands::Experiment {
                event_path,
                cache_count,
                cache_capacity,
                cache_type,
                warmup,
                num_requests,
                miss_latency,
                irt,
            } => {
                experiment(
                    chan,
                    cache_type,
                    event_path,
                    cache_count,
                    cache_capacity,
                    warmup,
                    num_requests,
                    miss_latency,
                    irt,
                )
                .await
            }
        }
    })
}
