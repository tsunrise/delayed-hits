pub mod msg;
use std::net::Ipv4Addr;

use msg::Message;
use proj_models::codec::Codec;
use smallvec::SmallVec;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _, BufReader, BufWriter},
    net::TcpStream,
};
use tracing::{error, info, warn};

/// A channel that can send and receive messages from a remote endpoint.
#[derive(Debug, Clone)]
pub struct RemoteChannel {
    /// Local -> one of the sockets
    sender: async_channel::Sender<Message>,
    /// One of the sockets -> local
    receiver: async_channel::Receiver<Message>,
}

impl RemoteChannel {
    pub async fn new(mode: ConnectionMode, num_connections: usize) -> Self {
        match mode {
            ConnectionMode::Server(port) => Self::new_as_server(port, num_connections).await,
            ConnectionMode::Client(ip, port) => {
                Self::new_as_client(ip, port, num_connections).await
            }
        }
    }

    pub async fn new_as_server(port: u16, num_connections: usize) -> Self {
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
            .await
            .unwrap();
        info!("Listening on port {}", port);
        let mut streams = Vec::new();
        for _ in 0..num_connections {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            info!("Accepted connection from {}", remote_addr);
            streams.push(stream);
        }

        Self::from_tcp_streams(streams)
    }

    pub async fn new_as_client(ip: Ipv4Addr, port: u16, num_connections: usize) -> Self {
        let mut streams = Vec::new();
        for _ in 0..num_connections {
            let stream = TcpStream::connect((ip, port)).await.unwrap();
            streams.push(stream);
        }

        Self::from_tcp_streams(streams)
    }

    fn from_tcp_streams(streams: Vec<TcpStream>) -> Self {
        let (read_sockets, write_sockets): (Vec<_>, Vec<_>) =
            streams.into_iter().map(|s| s.into_split()).unzip();

        // socket, when idle, read message from the user and send it to the remote endpoint
        let (user_side_sender, socket_side_receiver) = async_channel::unbounded::<Message>();
        for socket in write_sockets.into_iter() {
            let socket_side = socket_side_receiver.clone();
            tokio::spawn(async move {
                let peer_addr = socket.peer_addr().unwrap();
                let mut writer = BufWriter::new(socket);
                let mut buffer = SmallVec::<[u8; 64]>::new();
                loop {
                    // let message = socket_side.recv().await.unwrap();
                    let message = if let Ok(msg) = socket_side.recv().await {
                        msg
                    } else {
                        info!("All messages have been sent. Flushing and sending FIN.");
                        if writer.flush().await.is_err() {
                            warn!("Peer {} is closed before flushing", peer_addr);
                        }
                        break;
                    };
                    message.to_bytes(&mut buffer).unwrap();
                    if writer.write_all(&buffer).await.is_err() {
                        error!("Peer {} is closed before sending message", peer_addr);
                        break;
                    }
                }
            });
        }

        // socket, when receiving a message from the remote endpoint, send it to the user
        let (socket_side_sender, user_side_receiver) = async_channel::unbounded::<Message>();
        for socket in read_sockets.into_iter() {
            let socket_side = socket_side_sender.clone();
            tokio::spawn(async move {
                let peer_addr = socket.peer_addr().unwrap();
                let mut reader = BufReader::new(socket);
                let mut buffer = SmallVec::<[u8; 64]>::new();
                loop {
                    if let Err(e) = reader.read_exact(&mut buffer).await {
                        match e.kind() {
                            std::io::ErrorKind::UnexpectedEof => {
                                info!("Peer {} is closed", peer_addr);
                            }
                            _ => {
                                error!("Error reading from peer {}: {}", peer_addr, e);
                            }
                        }
                    }
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

    pub async fn send(&self, message: Message) -> Result<(), async_channel::SendError<Message>> {
        self.sender.send(message).await
    }

    pub async fn recv(&self) -> Result<Message, async_channel::RecvError> {
        self.receiver.recv().await
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionMode {
    Server(u16),
    Client(Ipv4Addr, u16),
}

#[derive(Error, Debug)]
pub enum ParseConnectionModeError {
    #[error("Invalid port: {0}")]
    InvalidPort(std::num::ParseIntError),
    #[error("Invalid IP address: {0}")]
    InvalidIpAddr(std::net::AddrParseError),
    #[error("Invalid connection mode: {0}")]
    InvalidConnectionMode(String),
}

/// - `<ipv4_addr>:<port>`: as client, connect to the server at `<ipv4_addr>:<port>`
/// - `<port>`: as server, listen on all interfaces at `<port>`
pub fn parse_connection_mode(mode: &str) -> Result<ConnectionMode, ParseConnectionModeError> {
    let parts: Vec<&str> = mode.split(':').collect();
    match parts.len() {
        1 => {
            let port = parts[0]
                .parse()
                .map_err(ParseConnectionModeError::InvalidPort)?;
            Ok(ConnectionMode::Server(port))
        }
        2 => {
            let ip = parts[0]
                .parse()
                .map_err(ParseConnectionModeError::InvalidIpAddr)?;
            let port = parts[1]
                .parse()
                .map_err(ParseConnectionModeError::InvalidPort)?;
            Ok(ConnectionMode::Client(ip, port))
        }
        _ => Err(ParseConnectionModeError::InvalidConnectionMode(
            mode.to_string(),
        )),
    }
}
