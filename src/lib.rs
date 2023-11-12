/// A provider for WebSockets
#[cfg(not(target_arch = "wasm32"))]
pub type WebSocketProvider = native_websocket::NativeWesocketProvider;

/// A provider for WebSockets
#[cfg(target_arch = "wasm32")]
pub type WebSocketProvider = wasm_websocket::WasmWebSocketProvider;

#[cfg(not(target_arch = "wasm32"))]
pub use native_websocket::NetworkSettings;

#[cfg(target_arch = "wasm32")]
pub use wasm_websocket::NetworkSettings;

#[cfg(not(target_arch = "wasm32"))]
mod native_websocket {
    use std::{net::SocketAddr, pin::Pin};

    use async_channel::{Receiver, Sender};
    use async_std::net::{TcpListener, TcpStream};
    use async_trait::async_trait;
    use async_tungstenite::tungstenite::protocol::WebSocketConfig;
    use bevy::prelude::{debug, error, info, trace, Deref, DerefMut, Resource};
    use bevy_eventwork::{error::NetworkError, managers::NetworkProvider, NetworkPacket};
    use futures::AsyncReadExt;
    use futures_lite::{AsyncWriteExt, Future, FutureExt, Stream};
    use ws_stream_tungstenite::WsStream;

    /// A provider for WebSockets
    #[derive(Default, Debug)]
    pub struct NativeWesocketProvider;

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    impl NetworkProvider for NativeWesocketProvider {
        type NetworkSettings = NetworkSettings;

        type Socket = WsStream<TcpStream>;

        type ReadHalf = futures::io::ReadHalf<WsStream<TcpStream>>;

        type WriteHalf = futures::io::WriteHalf<WsStream<TcpStream>>;

        type ConnectInfo = url::Url;

        type AcceptInfo = SocketAddr;

        type AcceptStream = OwnedIncoming;

        async fn accept_loop(
            accept_info: Self::AcceptInfo,
            _: Self::NetworkSettings,
        ) -> Result<Self::AcceptStream, NetworkError> {
            let listener = TcpListener::bind(accept_info)
                .await
                .map_err(NetworkError::Listen)?;
            Ok(OwnedIncoming::new(listener))
        }

        async fn connect_task(
            connect_info: Self::ConnectInfo,
            network_settings: Self::NetworkSettings,
        ) -> Result<Self::Socket, NetworkError> {
            info!("Beginning connection");
            let (stream, _response) = async_tungstenite::async_std::connect_async_with_config(
                connect_info,
                Some(*network_settings),
            )
            .await
            .map_err(|error| match error {
                async_tungstenite::tungstenite::Error::ConnectionClosed => {
                    NetworkError::Error(String::from("Connection closed"))
                }
                async_tungstenite::tungstenite::Error::AlreadyClosed => {
                    NetworkError::Error(String::from("Connection was already closed"))
                }
                async_tungstenite::tungstenite::Error::Io(io_error) => {
                    NetworkError::Error(format!("Io Error: {}", io_error))
                }
                async_tungstenite::tungstenite::Error::Tls(tls_error) => {
                    NetworkError::Error(format!("Tls Error: {}", tls_error))
                }
                async_tungstenite::tungstenite::Error::Capacity(cap) => {
                    NetworkError::Error(format!("Capacity Error: {}", cap))
                }
                async_tungstenite::tungstenite::Error::Protocol(proto) => {
                    NetworkError::Error(format!("Protocol Error: {}", proto))
                }
                async_tungstenite::tungstenite::Error::WriteBufferFull(buf) => {
                    NetworkError::Error(format!("Write Buffer Full Error: {}", buf))
                }
                async_tungstenite::tungstenite::Error::Utf8 => {
                    NetworkError::Error(format!("Utf8 Error"))
                }
                async_tungstenite::tungstenite::Error::AttackAttempt => {
                    NetworkError::Error(format!("Attack Attempt"))
                }
                async_tungstenite::tungstenite::Error::Url(url) => {
                    NetworkError::Error(format!("Url Error: {}", url))
                }
                async_tungstenite::tungstenite::Error::Http(http) => {
                    NetworkError::Error(format!("HTTP Error: {:?}", http))
                }
                async_tungstenite::tungstenite::Error::HttpFormat(http_format) => {
                    NetworkError::Error(format!("HTTP Format Error: {}", http_format))
                }
            })?;
            info!("Connected!");
            return Ok(WsStream::new(stream));
        }

        async fn recv_loop(
            mut read_half: Self::ReadHalf,
            messages: Sender<NetworkPacket>,
            settings: Self::NetworkSettings,
        ) {
            let mut buffer = vec![0; settings.max_message_size.unwrap_or(64 << 20)];
            loop {
                info!("Reading message length");
                let length = match read_half.read(&mut buffer[..8]).await {
                    Ok(0) => {
                        // EOF, meaning the TCP stream has closed.
                        info!("Client disconnected");
                        // TODO: probably want to do more than just quit the receive task.
                        //       to let eventwork know that the peer disconnected.
                        break;
                    }
                    Ok(8) => {
                        let bytes = &buffer[..8];
                        u64::from_le_bytes(
                            bytes
                                .try_into()
                                .expect("Couldn't read bytes from connection!"),
                        ) as usize
                    }
                    Ok(n) => {
                        error!(
                            "Could not read enough bytes for header. Expected 8, got {}",
                            n
                        );
                        break;
                    }
                    Err(err) => {
                        error!("Encountered error while fetching length: {}", err);
                        break;
                    }
                };
                info!("Message length: {}", length);

                if length > settings.max_message_size.unwrap_or(64 << 20) {
                    error!(
                        "Received too large packet: {} > {}",
                        length,
                        settings.max_message_size.unwrap_or(64 << 20)
                    );
                    break;
                }

                info!("Reading message into buffer");
                match read_half.read_exact(&mut buffer[..length]).await {
                    Ok(()) => (),
                    Err(err) => {
                        error!(
                            "Encountered error while fetching stream of length {}: {}",
                            length, err
                        );
                        break;
                    }
                }
                info!("Message read");

                let packet: NetworkPacket = match bincode::deserialize(&buffer[..length]) {
                    Ok(packet) => packet,
                    Err(err) => {
                        error!("Failed to decode network packet from: {}", err);
                        break;
                    }
                };

                if messages.send(packet).await.is_err() {
                    error!("Failed to send decoded message to eventwork");
                    break;
                }
                info!("Message deserialized and sent to eventwork");
            }
        }

        async fn send_loop(
            mut write_half: Self::WriteHalf,
            messages: Receiver<NetworkPacket>,
            _settings: Self::NetworkSettings,
        ) {
            while let Ok(message) = messages.recv().await {
                let encoded = match bincode::serialize(&message) {
                    Ok(encoded) => encoded,
                    Err(err) => {
                        error!("Could not encode packet {:?}: {}", message, err);
                        continue;
                    }
                };

                let len = encoded.len() as u64;
                debug!("Sending a new message of size: {}", len);

                match write_half.write(&len.to_le_bytes()).await {
                    Ok(_) => (),
                    Err(err) => {
                        error!("Could not send packet length: {:?}: {}", len, err);
                        break;
                    }
                }

                trace!("Sending the content of the message!");

                match write_half.write_all(&encoded).await {
                    Ok(_) => (),
                    Err(err) => {
                        error!("Could not send packet: {:?}: {}", message, err);
                        break;
                    }
                }

                trace!("Succesfully written all!");
            }
        }

        fn split(combined: Self::Socket) -> (Self::ReadHalf, Self::WriteHalf) {
            combined.split()
        }
    }

    #[derive(Clone, Debug, Resource, Default, Deref, DerefMut)]
    #[allow(missing_copy_implementations)]
    /// Settings to configure the network, both client and server
    pub struct NetworkSettings(WebSocketConfig);

    /// A special stream for recieving ws connections
    pub struct OwnedIncoming {
        inner: TcpListener,
        stream: Option<Pin<Box<dyn Future<Output = Option<WsStream<TcpStream>>>>>>,
    }

    impl OwnedIncoming {
        fn new(listener: TcpListener) -> Self {
            Self {
                inner: listener,
                stream: None,
            }
        }
    }

    impl Stream for OwnedIncoming {
        type Item = WsStream<TcpStream>;

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            let incoming = self.get_mut();
            if incoming.stream.is_none() {
                let listener: *const TcpListener = &incoming.inner;
                incoming.stream = Some(Box::pin(async move {
                    let stream = unsafe {
                        listener
                            .as_ref()
                            .expect("Segfault when trying to read listener in OwnedStream")
                    }
                    .accept()
                    .await
                    .map(|(s, _)| s)
                    .ok();

                    let stream: WsStream<TcpStream> = match stream {
                        Some(stream) => {
                            if let Some(stream) = async_tungstenite::accept_async(stream).await.ok()
                            {
                                WsStream::new(stream)
                            } else {
                                return None;
                            }
                        }

                        None => return None,
                    };
                    Some(stream)
                }));
            }
            if let Some(stream) = &mut incoming.stream {
                if let std::task::Poll::Ready(res) = stream.poll(cx) {
                    incoming.stream = None;
                    return std::task::Poll::Ready(res);
                }
            }
            std::task::Poll::Pending
        }
    }

    unsafe impl Send for OwnedIncoming {}
}

#[cfg(target_arch = "wasm32")]
mod wasm_websocket {
    use core::panic;
    use std::{net::SocketAddr, pin::Pin};

    use async_channel::{Receiver, Sender};
    use async_io_stream::IoStream;
    use async_trait::async_trait;
    use bevy::prelude::{debug, error, info, trace, Deref, DerefMut, Resource};
    use bevy_eventwork::{error::NetworkError, managers::NetworkProvider, NetworkPacket};
    use futures::AsyncReadExt;
    use futures_lite::{AsyncWriteExt, Future, FutureExt, Stream};
    use ws_stream_wasm::{WsMeta, WsStream, WsStreamIo};

    /// A provider for WebSockets
    #[derive(Default, Debug)]
    pub struct WasmWebSocketProvider;

    #[async_trait(?Send)]
    impl NetworkProvider for WasmWebSocketProvider {
        type NetworkSettings = NetworkSettings;

        type Socket = (WsMeta, WsStream);

        type ReadHalf = futures::io::ReadHalf<IoStream<WsStreamIo, Vec<u8>>>;

        type WriteHalf = futures::io::WriteHalf<IoStream<WsStreamIo, Vec<u8>>>;

        type ConnectInfo = url::Url;

        type AcceptInfo = SocketAddr;

        type AcceptStream = OwnedIncoming;

        async fn accept_loop(
            accept_info: Self::AcceptInfo,
            _: Self::NetworkSettings,
        ) -> Result<Self::AcceptStream, NetworkError> {
            panic!("Can't create servers on WASM");
        }

        async fn connect_task(
            connect_info: Self::ConnectInfo,
            network_settings: Self::NetworkSettings,
        ) -> Result<Self::Socket, NetworkError> {
            info!("Beginning connection");
            let stream =
                WsMeta::connect(connect_info, None)
                    .await
                    .map_err(|error| match error {
                        ws_stream_wasm::WsErr::InvalidWsState { supplied } => {
                            NetworkError::Error(format!("Invalid Websocket State: {}", supplied))
                        }
                        ws_stream_wasm::WsErr::ConnectionNotOpen => {
                            NetworkError::Error(format!("Connection Not Open"))
                        }
                        ws_stream_wasm::WsErr::InvalidUrl { supplied } => {
                            NetworkError::Error(format!("Invalid URL: {}", supplied))
                        }
                        ws_stream_wasm::WsErr::InvalidCloseCode { supplied } => {
                            NetworkError::Error(format!("Invalid Close Code: {}", supplied))
                        }
                        ws_stream_wasm::WsErr::ReasonStringToLong => {
                            NetworkError::Error(format!("Reason String To Long"))
                        }
                        ws_stream_wasm::WsErr::ConnectionFailed { event } => {
                            NetworkError::Error(format!("Connection Failed: {:?}", event))
                        }
                        ws_stream_wasm::WsErr::InvalidEncoding => {
                            NetworkError::Error(format!("IOnvalid Encoding"))
                        }
                        ws_stream_wasm::WsErr::CantDecodeBlob => {
                            NetworkError::Error(format!("Cant Decode Blob"))
                        }
                        ws_stream_wasm::WsErr::UnknownDataType => {
                            NetworkError::Error(format!("Unkown Data Type"))
                        }
                        _ => NetworkError::Error(format!("Error in Ws_Stream_Wasm")),
                    })?;
            info!("Connected!");
            return Ok(stream);
        }

        async fn recv_loop(
            mut read_half: Self::ReadHalf,
            messages: Sender<NetworkPacket>,
            settings: Self::NetworkSettings,
        ) {
            let mut buffer = vec![0; settings.max_message_size];
            loop {
                info!("Reading message length");
                let length = match read_half.read(&mut buffer[..8]).await {
                    Ok(0) => {
                        // EOF, meaning the TCP stream has closed.
                        info!("Client disconnected");
                        // TODO: probably want to do more than just quit the receive task.
                        //       to let eventwork know that the peer disconnected.
                        break;
                    }
                    Ok(8) => {
                        let bytes = &buffer[..8];
                        u64::from_le_bytes(
                            bytes
                                .try_into()
                                .expect("Couldn't read bytes from connection!"),
                        ) as usize
                    }
                    Ok(n) => {
                        error!(
                            "Could not read enough bytes for header. Expected 8, got {}",
                            n
                        );
                        break;
                    }
                    Err(err) => {
                        error!("Encountered error while fetching length: {}", err);
                        break;
                    }
                };
                info!("Message length: {}", length);

                if length > settings.max_message_size {
                    error!(
                        "Received too large packet: {} > {}",
                        length, settings.max_message_size
                    );
                    break;
                }

                info!("Reading message into buffer");
                match read_half.read_exact(&mut buffer[..length]).await {
                    Ok(()) => (),
                    Err(err) => {
                        error!(
                            "Encountered error while fetching stream of length {}: {}",
                            length, err
                        );
                        break;
                    }
                }
                info!("Message read");

                let packet: NetworkPacket = match bincode::deserialize(&buffer[..length]) {
                    Ok(packet) => packet,
                    Err(err) => {
                        error!("Failed to decode network packet from: {}", err);
                        break;
                    }
                };

                if messages.send(packet).await.is_err() {
                    error!("Failed to send decoded message to eventwork");
                    break;
                }
                info!("Message deserialized and sent to eventwork");
            }
        }

        async fn send_loop(
            mut write_half: Self::WriteHalf,
            messages: Receiver<NetworkPacket>,
            _settings: Self::NetworkSettings,
        ) {
            while let Ok(message) = messages.recv().await {
                let encoded = match bincode::serialize(&message) {
                    Ok(encoded) => encoded,
                    Err(err) => {
                        error!("Could not encode packet {:?}: {}", message, err);
                        continue;
                    }
                };

                let len = encoded.len() as u64;
                debug!("Sending a new message of size: {}", len);

                match write_half.write(&len.to_le_bytes()).await {
                    Ok(_) => (),
                    Err(err) => {
                        error!("Could not send packet length: {:?}: {}", len, err);
                        break;
                    }
                }

                trace!("Sending the content of the message!");

                match write_half.write_all(&encoded).await {
                    Ok(_) => (),
                    Err(err) => {
                        error!("Could not send packet: {:?}: {}", message, err);
                        break;
                    }
                }

                trace!("Succesfully written all!");
            }
        }

        fn split(combined: Self::Socket) -> (Self::ReadHalf, Self::WriteHalf) {
            combined.1.into_io().split()
        }
    }

    #[derive(Clone, Debug, Resource, Deref, DerefMut)]
    #[allow(missing_copy_implementations)]
    /// Settings to configure the network, both client and server
    pub struct NetworkSettings {
        max_message_size: usize,
    }

    impl Default for NetworkSettings {
        fn default() -> Self {
            Self {
                max_message_size: 64 << 20,
            }
        }
    }

    /// A dummy struct as WASM is unable to accept connections and act as a server
    pub struct OwnedIncoming;

    impl Stream for OwnedIncoming {
        type Item = (WsMeta, WsStream);

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            panic!("WASM does not support servers");
        }
    }
}
