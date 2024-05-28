#[allow(unused)]
mod simulator;

use clap::Parser as _;
use clap_derive::Parser;
use proj_net::{
    msg::{CdnRequestMessage, OriginResponseMessage},
    ConnectionMode, RemoteChannel,
};
use tracing::info;

const MEASURE_ROUNDS: usize = 500000;

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
}

async fn async_main() {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    info!("{:?}", args);

    let chan = RemoteChannel::<CdnRequestMessage, OriginResponseMessage>::new(
        args.conn,
        args.num_connections,
    )
    .await;

    info!("Measuring average latency...");
    let mut starts = Vec::new();
    let mut handles = Vec::new();
    for idx in 0..MEASURE_ROUNDS {
        starts.push(tokio::time::Instant::now());
        handles.push(chan.send(CdnRequestMessage::new(idx as u64)).await);
    }
    for handle in handles {
        assert!(handle.wait().await);
    }
    chan.flush().await;
    let mut total_latency = tokio::time::Duration::from_secs(0);
    // TODO: things got serialized
    for _ in 0..MEASURE_ROUNDS {
        let response = chan.recv().await.unwrap();
        total_latency += tokio::time::Instant::now() - starts[response.key as usize];
    }
    let avg_latency = total_latency.as_micros() as f64 / MEASURE_ROUNDS as f64;
    info!("Average latency: {:.2}us", avg_latency);
}

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        async_main().await;
    })
}
