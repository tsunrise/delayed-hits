mod simulator;
use std::net::Ipv4Addr;

use clap::Parser as _;
use clap_derive::Parser;

#[derive(Parser, Debug)]
struct Args {
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
    #[clap(long, short = 'a')]
    origin_addr: Ipv4Addr,
    #[clap(long, short = 'p', default_value = "12244")]
    origin_port: u16,
}

async fn async_main() {
    let args = Args::parse();
    println!("{:?}", args);

    tracing_subscriber::fmt::init();
}

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        async_main().await;
    })
}
