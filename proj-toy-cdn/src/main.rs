#[allow(unused)]
mod simulator;

use std::time::Instant;

use clap::Parser as _;
use clap_derive::Parser;
use proj_net::{
    msg::{CdnRequestMessage, OriginResponseMessage},
    ConnectionMode, RemoteChannel,
};
use simulator::Clock;
use tracing::info;

#[derive(Parser, Debug)]
struct Args {
    // #[clap(required = true, help = "Path to the processed events file")]
    // event_path: String,
    // #[clap(
    //     long,
    //     short = 'k',
    //     help = "number of caches (k for k-way set-associative cache)"
    // )]
    // cache_count: usize,
    // #[clap(long, short = 'c', help = "cache capacity in each cache")]
    // cache_capacity: usize,
    // #[clap(
    //     long,
    //     short = 'w',
    //     help = "number of warmup requests to warm the cache before sending actual requests. Those requests are not sent to the internet and are not counted."
    // )]
    // warmup: usize,
    // #[clap(
    //     long,
    //     short = 'm',
    //     help = "number of actual requests to process after the warmup"
    // )]
    // num_requests: usize,
    #[clap(long, short = 'c', value_parser = proj_net::parse_connection_mode, help = "use <ip_addr>:<port> for client, <port> for server")]
    conn: ConnectionMode,
    #[clap(long, short = 'n', default_value = "8", help = "number of connections")]
    num_connections: usize,
    #[clap(
        long,
        short = 'r',
        default_value = "100",
        help = "number of measure rounds"
    )]
    num_measure_rounds: usize,
    #[clap(
        long,
        short = 'b',
        default_value = "4",
        help = "number of messages buffered in the channel"
    )]
    num_msg_buffered: usize,
}

async fn measurement() {
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
    for idx in 0..args.num_measure_rounds {
        let clock = Clock::tick();
        starts.push(std::time::Instant::now());
        handles.push(chan.send(CdnRequestMessage::new(idx as u64)).await);
        clock.wait_until_next_available(1000).await; // TODO: try increase this
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
        / (args.num_measure_rounds as u64 - 1) as f64;
    let average_i_response_t = ends_ns.windows(2).map(|w| w[1] - w[0]).sum::<u64>() as f64
        / (args.num_measure_rounds as u64 - 1) as f64;
    let average_delay = ends_ns
        .iter()
        .zip(starts_ns.iter())
        .map(|(e, s)| e - s)
        .sum::<u64>() as f64
        / args.num_measure_rounds as f64;

    info!(
        "Starts: {:?}",
        starts_ns.iter().take(10).collect::<Vec<_>>()
    );
    info!("Ends: {:?}", ends_ns.iter().take(10).collect::<Vec<_>>());
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

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        measurement().await;
    })
}
