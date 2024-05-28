pub mod msg;
use std::net::Ipv4Addr;

use proj_models::codec::Codec;
use smallvec::SmallVec;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _, BufReader, BufWriter},
    net::TcpStream,
};
use tracing::{error, info, trace, warn};

#[derive(Debug, Clone, Copy)]
enum Control {
    FlushWrite,
}

pub struct CompletionHandle {
    receiver: tokio::sync::oneshot::Receiver<bool>,
}

impl CompletionHandle {
    /// Wait and return whether the message has been sent successfully.
    pub async fn wait(self) -> bool {
        self.receiver.await.unwrap()
    }
}

/// A channel that can send and receive messages from a remote endpoint.
/// - `S`: the type of the message sent from the local endpoint to the remote endpoint
/// - `R`: the type of the message sent from the remote endpoint to the local endpoint
#[derive(Debug, Clone)]
pub struct RemoteChannel<S, R>
where
    S: Codec,
    R: Codec,
{
    /// Local -> one of the sockets
    sender: async_channel::Sender<(S, tokio::sync::oneshot::Sender<bool>)>,
    /// Local -> All sockets
    sender_control: tokio::sync::broadcast::Sender<Control>,
    /// One of the sockets -> local
    receiver: async_channel::Receiver<R::Deserialized>,
}

impl<S, R> RemoteChannel<S, R>
where
    S: Codec + Send + 'static,
    R: Codec,
    R::Deserialized: Send + 'static,
{
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
        let (user_side_sender, socket_side_receiver) =
            async_channel::unbounded::<(S, tokio::sync::oneshot::Sender<bool>)>();
        let (user_side_control_sender, socket_side_control_receiver) =
            tokio::sync::broadcast::channel(1);
        for socket in write_sockets.into_iter() {
            let socket_side = socket_side_receiver.clone();
            let mut control_receiver = socket_side_control_receiver.resubscribe();
            tokio::spawn(async move {
                let peer_addr = socket.peer_addr().unwrap();
                let mut writer = BufWriter::new(socket);
                let mut buffer =
                    SmallVec::<[u8; 64]>::from_elem(0, S::SIZE_IN_BYTES.get_size_or_panic());
                loop {
                    let message = tokio::select! {
                        message = socket_side.recv() => message,
                        _ = control_receiver.recv() => {
                            info!("Flushing writer for peer {}", peer_addr);
                            if writer.flush().await.is_err() {
                                warn!("Peer {} is closed before flushing", peer_addr);
                            }
                            continue;
                        }
                    };
                    let (message, handle) = match message {
                        Ok(msg) => msg,
                        Err(_) => {
                            info!("All messages have been sent. Flushing and sending FIN.");
                            if writer.flush().await.is_err() {
                                warn!("Peer {} is closed before flushing", peer_addr);
                            }
                            break;
                        }
                    };
                    message.to_bytes(&mut buffer[..]).unwrap();
                    if writer.write_all(&buffer).await.is_err() {
                        error!("Peer {} is closed before sending message", peer_addr);
                        handle.send(false).unwrap();
                        break;
                    }
                    handle.send(true).unwrap();
                    trace!("Sent message");
                }
            });
        }

        // socket, when receiving a message from the remote endpoint, send it to the user
        let (socket_side_sender, user_side_receiver) =
            async_channel::unbounded::<R::Deserialized>();
        for socket in read_sockets.into_iter() {
            let socket_side = socket_side_sender.clone();
            tokio::spawn(async move {
                let peer_addr = socket.peer_addr().unwrap();
                let mut reader = BufReader::new(socket);
                let mut buffer =
                    SmallVec::<[u8; 64]>::from_elem(0, R::SIZE_IN_BYTES.get_size_or_panic());
                loop {
                    trace!("Waiting for peer");
                    if let Err(e) = reader.read_exact(&mut buffer[..]).await {
                        match e.kind() {
                            std::io::ErrorKind::UnexpectedEof => {
                                info!("Peer {} is closed", peer_addr);
                            }
                            _ => {
                                error!("Error reading from peer {}: {}", peer_addr, e);
                            }
                        }
                        break;
                    }
                    let message = R::from_bytes(&mut buffer.as_ref()).unwrap();
                    trace!("Got a message");
                    socket_side.send(message).await.unwrap();
                }
            });
        }

        Self {
            sender: user_side_sender,
            sender_control: user_side_control_sender,
            receiver: user_side_receiver,
        }
    }

    pub async fn send(&self, message: S) -> CompletionHandle {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.sender
            .send((message, sender))
            .await
            .expect("Failed to send message");
        CompletionHandle { receiver }
    }

    pub async fn flush(&self) {
        if let Ok(n) = self.sender_control.send(Control::FlushWrite) {
            if n == 0 {
                warn!("No receiver for the control message");
            }
        } else {
            error!("Failed to send control message")
        }
    }

    pub async fn recv(&self) -> Result<R::Deserialized, async_channel::RecvError> {
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
