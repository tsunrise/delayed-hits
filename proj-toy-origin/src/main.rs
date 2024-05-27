use clap::Parser as _;
use clap_derive::Parser;
use proj_net::RemoteChannel;
use tokio::runtime::Runtime;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, short = 'p', default_value = "12244")]
    pub port: u16,
    #[clap(long, short = 'n', default_value = "64")]
    pub num_connections: usize,
}

async fn async_main() {
    let args = Args::parse();
    println!("{:?}", args);

    tracing_subscriber::fmt::init();

    // just echo loop
    let chan = RemoteChannel::new_as_origin_server(args.port, args.num_connections).await;
    loop {
        let msg = if let Ok(msg) = chan.recv().await {
            msg
        } else {
            break;
        };
        if chan.send(msg).await.is_err() {
            break;
        }
    }
}

fn main() {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        async_main().await;
    })
}
