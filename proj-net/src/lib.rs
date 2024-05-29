pub mod msg;
use std::net::Ipv4Addr;

use proj_models::codec::Codec;
use smallvec::SmallVec;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _, BufReader, BufWriter},
    net::TcpStream,
    task::JoinHandle,
};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug)]
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
    /// The task handles for the sender and receiver tasks (are not cloned)
    task_handles: Option<Box<Vec<JoinHandle<()>>>>,
}

impl<S: Codec, R: Codec> Clone for RemoteChannel<S, R> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            sender_control: self.sender_control.clone(),
            receiver: self.receiver.clone(),
            task_handles: None,
        }
    }
}

impl<S, R> RemoteChannel<S, R>
where
    S: Codec + Send + 'static,
    R: Codec,
    R::Deserialized: Send + 'static,
{
    pub async fn new(
        mode: ConnectionMode,
        num_connections: usize,
        num_msg_buffered: usize,
    ) -> Self {
        match mode {
            ConnectionMode::Server(port) => {
                Self::new_as_server(port, num_connections, num_msg_buffered).await
            }
            ConnectionMode::Client(ip, port) => {
                Self::new_as_client(ip, port, num_connections, num_msg_buffered).await
            }
        }
    }

    pub async fn new_as_server(port: u16, num_connections: usize, num_msg_buffered: usize) -> Self {
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

        Self::from_tcp_streams(streams, num_msg_buffered)
    }

    pub async fn new_as_client(
        ip: Ipv4Addr,
        port: u16,
        num_connections: usize,
        num_msg_buffered: usize,
    ) -> Self {
        let mut streams = Vec::new();
        for _ in 0..num_connections {
            let stream = TcpStream::connect((ip, port)).await.unwrap();
            streams.push(stream);
        }

        Self::from_tcp_streams(streams, num_msg_buffered)
    }

    fn from_tcp_streams(streams: Vec<TcpStream>, num_msg_buffered: usize) -> Self {
        streams.iter().for_each(|s| s.set_nodelay(true).unwrap());
        let (read_sockets, write_sockets): (Vec<_>, Vec<_>) =
            streams.into_iter().map(|s| s.into_split()).unzip();

        // socket, when idle, read message from the user and send it to the remote endpoint
        let (user_side_sender, socket_side_receiver) =
            async_channel::unbounded::<(S, tokio::sync::oneshot::Sender<bool>)>();
        let (user_side_control_sender, socket_side_control_receiver) =
            tokio::sync::broadcast::channel(1);

        let mut task_handles = Vec::new();
        for socket in write_sockets.into_iter() {
            let socket_side = socket_side_receiver.clone();
            let mut control_receiver = socket_side_control_receiver.resubscribe();
            let task_handle = tokio::spawn(async move {
                let peer_addr = socket.peer_addr().unwrap();
                let mut writer = BufWriter::with_capacity(
                    S::SIZE_IN_BYTES.get_size_or_panic() * num_msg_buffered,
                    socket,
                );
                // BufWriter::new(socket);
                let mut buffer =
                    SmallVec::<[u8; 64]>::from_elem(0, S::SIZE_IN_BYTES.get_size_or_panic());
                loop {
                    let message = tokio::select! {
                        message = socket_side.recv() => message,
                        _ = control_receiver.recv() => {
                            // trace!("Flushing writer for peer {}", peer_addr);
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
                    // trace!("Sent message");
                }
            });
            task_handles.push(task_handle);
        }

        // socket, when receiving a message from the remote endpoint, send it to the user
        let (socket_side_sender, user_side_receiver) =
            async_channel::unbounded::<R::Deserialized>();
        for socket in read_sockets.into_iter() {
            let socket_side = socket_side_sender.clone();
            let task_handle = tokio::spawn(async move {
                let peer_addr = socket.peer_addr().unwrap();
                let mut reader = BufReader::with_capacity(
                    R::SIZE_IN_BYTES.get_size_or_panic() * num_msg_buffered,
                    socket,
                );
                // BufReader::new(socket);
                let mut buffer =
                    SmallVec::<[u8; 64]>::from_elem(0, R::SIZE_IN_BYTES.get_size_or_panic());
                loop {
                    // trace!("Waiting for peer");
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
                    // trace!("Got a message");
                    socket_side.send(message).await.unwrap();
                }
            });
            task_handles.push(task_handle);
        }

        Self {
            sender: user_side_sender,
            sender_control: user_side_control_sender,
            receiver: user_side_receiver,
            task_handles: Some(Box::new(task_handles)),
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

    pub async fn close_send(&self) {
        self.sender.close();
    }

    pub async fn close_recv(&self) {
        self.receiver.close();
    }

    pub async fn recv(&self) -> Result<R::Deserialized, async_channel::RecvError> {
        self.receiver.recv().await
    }

    pub fn take_task_handles(&mut self) -> Option<Vec<tokio::task::JoinHandle<()>>> {
        self.task_handles.take().map(|b| *b)
    }

    pub async fn grace_shutdown(mut self) {
        self.close_send().await;
        self.close_recv().await;
        if let Some(task_handles) = self.take_task_handles() {
            for handle in task_handles {
                handle.await.unwrap();
            }
        } else {
            error!("Attempt to grace shutdown a remote channel without task handles")
        }
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
