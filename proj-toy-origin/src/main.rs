use clap::Parser as _;
use clap_derive::Parser;
use proj_net::{
    msg::{CdnRequestMessage, FixedSizeResponsePayload, OriginResponseMessage},
    ConnectionMode, RemoteChannel,
};
use rand::{Rng as _, SeedableRng};
use rand_xorshift::XorShiftRng;
use tokio::runtime::Runtime;
use tracing::info;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, short = 'c', value_parser = proj_net::parse_connection_mode, help = "use <ip_addr>:<port> for client, <port> for server")]
    pub conn: ConnectionMode,
    #[clap(long, short = 'n', default_value = "8")]
    pub num_connections: usize,
}

async fn async_main() {
    let args = Args::parse();
    info!("{:?}", args);

    // just echo loop
    let chan = RemoteChannel::<OriginResponseMessage, CdnRequestMessage>::new(
        args.conn,
        args.num_connections,
    )
    .await;
    while let Ok(msg) = chan.recv().await {
        let chan = chan.clone();
        tokio::spawn(async move {
            let mut rng = XorShiftRng::from_rng(rand::thread_rng()).unwrap();
            let mut payload = FixedSizeResponsePayload::default();
            rng.fill(&mut payload.content[..]);
            let response = OriginResponseMessage::new(msg.key, payload);
            let handle = chan.send(response).await;
            assert!(handle.wait().await);
            chan.flush().await;
        });
    }
}

fn main() {
    let runtime = Runtime::new().unwrap();
    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    runtime.block_on(async {
        async_main().await;
    })
}
