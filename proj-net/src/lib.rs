pub mod msg;
use std::net::Ipv4Addr;

use msg::Message;
use proj_models::codec::Codec;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _, BufReader, BufWriter},
    net::TcpStream,
};
use tracing::info;

/// A channel that can send and receive messages from a remote endpoint.
#[derive(Debug, Clone)]
pub struct RemoteChannel {
    /// Local -> one of the sockets
    sender: async_channel::Sender<Message>,
    /// One of the sockets -> local
    receiver: async_channel::Receiver<Message>,
}

impl RemoteChannel {
    pub async fn new_as_server<I>(ports: I) -> Self
    where
        I: IntoIterator<Item = u16>,
    {
        let handles = ports
            .into_iter()
            .map(|port| {
                tokio::spawn(async move {
                    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
                        .await
                        .unwrap();
                    let (stream, remote_addr) = listener.accept().await.unwrap();
                    info!("Accepted connection from {}", remote_addr);
                    stream
                })
            })
            .collect::<Vec<_>>();
        let mut streams = Vec::new();
        for handle in handles {
            streams.push(handle.await.unwrap());
        }

        Self::from_tcp_streams(streams)
    }

    pub async fn new_as_client<I>(ip: Ipv4Addr, ports: I) -> Self
    where
        I: IntoIterator<Item = u16>,
    {
        let handles = ports
            .into_iter()
            .map(|port| {
                tokio::spawn(async move {
                    let stream = TcpStream::connect((ip, port)).await.unwrap();
                    info!("Connected to {}", stream.peer_addr().unwrap());
                    stream
                })
            })
            .collect::<Vec<_>>();
        let mut streams = Vec::new();
        for handle in handles {
            streams.push(handle.await.unwrap());
        }

        Self::from_tcp_streams(streams)
    }

    pub fn from_tcp_streams(streams: Vec<TcpStream>) -> Self {
        for stream in streams.iter() {
            stream.set_nodelay(true).unwrap();
        }
        let (read_sockets, write_sockets): (Vec<_>, Vec<_>) =
            streams.into_iter().map(|s| s.into_split()).unzip();

        // socket, when idle, read message from the user and send it to the remote endpoint
        let (user_side_sender, socket_side_receiver) = async_channel::unbounded::<Message>();
        for socket in write_sockets.into_iter() {
            let socket_side = socket_side_receiver.clone();
            tokio::spawn(async move {
                // We want each message take one RTT to be sent and received
                let mut writer = BufWriter::with_capacity(Message::size_in_bytes(), socket);
                let mut buffer = [0; Message::SIZE_IN_BYTES];
                loop {
                    let message = socket_side.recv().await.unwrap();
                    message.to_bytes(&mut buffer.as_mut()).unwrap();
                    writer.write_all(&buffer).await.unwrap();
                }
            });
        }

        // socket, when receiving a message from the remote endpoint, send it to the user
        let (socket_side_sender, user_side_receiver) = async_channel::unbounded::<Message>();
        for socket in read_sockets.into_iter() {
            let socket_side = socket_side_sender.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::with_capacity(Message::size_in_bytes(), socket);
                let mut buffer = [0; Message::SIZE_IN_BYTES];
                loop {
                    reader.read_exact(&mut buffer).await.unwrap();
                    let message = Message::from_bytes(&mut buffer.as_ref()).unwrap();
                    socket_side.send(message).await.unwrap();
                }
            });
        }

        Self {
            sender: user_side_sender,
            receiver: user_side_receiver,
        }
    }

    pub async fn send(&self, message: Message) {
        self.sender.send(message).await.unwrap();
    }

    pub async fn recv(&self) -> Message {
        self.receiver.recv().await.unwrap()
    }
}
