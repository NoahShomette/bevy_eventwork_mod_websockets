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

pub mod serde_json;

#[cfg(not(target_arch = "wasm32"))]
mod native_websocket {
    use std::{net::SocketAddr, pin::Pin};

    use async_channel::{Receiver, Sender};
    use async_std::net::{TcpListener, TcpStream};
    use async_trait::async_trait;
    use async_tungstenite::{
        tungstenite::{protocol::WebSocketConfig, Message},
        WebSocketStream,
    };
    use bevy::prelude::{error, info, trace, Deref, DerefMut, Resource};
    use bevy_eventwork::{
        error::NetworkError, managers::NetworkProvider, NetworkPacket, NetworkSerializedData,
    };
    use futures::{
        stream::{SplitSink, SplitStream},
        SinkExt, StreamExt,
    };
    use futures_lite::{Future, FutureExt, Stream};

    /// A provider for WebSockets
    #[derive(Default, Debug, Clone)]
    pub struct NativeWesocketProvider;

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    impl NetworkProvider for NativeWesocketProvider {
        type NetworkSettings = NetworkSettings;

        type Socket = WebSocketStream<TcpStream>;

        type ReadHalf = SplitStream<WebSocketStream<TcpStream>>;

        type WriteHalf = SplitSink<WebSocketStream<TcpStream>, Message>;

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
            return Ok(stream);
        }

        async fn recv_loop(
            mut read_half: Self::ReadHalf,
            messages: Sender<NetworkPacket>,
            _settings: Self::NetworkSettings,
            network_packet_de: fn(data: NetworkSerializedData) -> Result<NetworkPacket, String>,
        ) {
            loop {
                let message = match read_half.next().await {
                    Some(message) => match message {
                        Ok(message) => message,
                        Err(err) => match err {
                            async_tungstenite::tungstenite::Error::ConnectionClosed
                            | async_tungstenite::tungstenite::Error::AlreadyClosed => {
                                error!("Connection Closed");
                                break;
                            }
                            _ => {
                                error!("Nonfatal error detected: {}", err);
                                continue;
                            }
                        },
                    },
                    None => {
                        continue;
                    }
                };

                let packet = match message {
                    Message::Text(text) => {
                        if cfg!(feature = "json") {
                            match network_packet_de(NetworkSerializedData::String(text)) {
                                Ok(packet) => packet,
                                Err(err) => {
                                    error!("Failed to decode network packet from: {}", err);
                                    break;
                                }
                            }
                        } else {
                            error!("String message recieved and not supported. Enable JSON feature to accept string messages");
                            break;
                        }
                    }
                    Message::Binary(binary) => {
                        match network_packet_de(NetworkSerializedData::Binary(binary)) {
                            Ok(packet) => packet,
                            Err(err) => {
                                error!("Failed to decode network packet from: {}", err);
                                break;
                            }
                        }
                    }
                    Message::Ping(_) => {
                        error!("Ping Message Received");
                        break;
                    }
                    Message::Pong(_) => {
                        error!("Pong Message Received");
                        break;
                    }
                    Message::Close(_) => {
                        error!("Connection Closed");
                        break;
                    }
                    Message::Frame(_) => todo!(),
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
            network_packet_ser: fn(data: NetworkPacket) -> Result<NetworkSerializedData, String>,
        ) {
            while let Ok(message) = messages.recv().await {
                let encoded = match network_packet_ser(message) {
                    Ok(encoded) => encoded,
                    Err(err) => {
                        error!("Could not encode packet: {}", err);
                        continue;
                    }
                };

                trace!("Sending the content of the message!");
                match encoded {
                    NetworkSerializedData::String(text) => match write_half
                        .send(async_tungstenite::tungstenite::Message::Text(text))
                        .await
                    {
                        Ok(_) => (),
                        Err(err) => {
                            error!("Could not send packet: {}", err);
                            break;
                        }
                    },
                    NetworkSerializedData::Binary(vec) => match write_half
                        .send(async_tungstenite::tungstenite::Message::Binary(vec))
                        .await
                    {
                        Ok(_) => (),
                        Err(err) => {
                            error!("Could not send packet: {}", err);
                            break;
                        }
                    },
                }

                trace!("Succesfully written all!");
            }
        }

        fn split(combined: Self::Socket) -> (Self::ReadHalf, Self::WriteHalf) {
            let (write, read) = combined.split();
            (read, write)
        }
    }

    #[derive(Clone, Debug, Resource, Default, Deref, DerefMut)]
    #[allow(missing_copy_implementations)]
    /// Settings to configure the network, both client and server
    pub struct NetworkSettings(WebSocketConfig);

    /// A special stream for recieving ws connections
    pub struct OwnedIncoming {
        inner: TcpListener,
        stream: Option<Pin<Box<dyn Future<Output = Option<WebSocketStream<TcpStream>>>>>>,
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
        type Item = WebSocketStream<TcpStream>;

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

                    let stream: WebSocketStream<TcpStream> = match stream {
                        Some(stream) => {
                            if let Some(stream) = async_tungstenite::accept_async(stream).await.ok()
                            {
                                stream
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
    use async_trait::async_trait;
    use bevy::prelude::{error, info, trace, Deref, DerefMut, Resource};
    use bevy_eventwork::{
        error::NetworkError, managers::NetworkProvider, NetworkPacket, NetworkSerializedData,
    };
    use futures::{
        stream::{SplitSink, SplitStream},
        SinkExt, StreamExt,
    };
    use futures_lite::Stream;
    use send_wrapper::SendWrapper;
    use tokio_tungstenite_wasm::{Message, WebSocketStream};

    /// A provider for WebSockets
    #[derive(Default, Debug, Clone)]
    pub struct WasmWebSocketProvider;

    #[async_trait(?Send)]
    impl NetworkProvider for WasmWebSocketProvider {
        type NetworkSettings = NetworkSettings;

        type Socket = SendWrapper<WebSocketStream>;

        type ReadHalf = SendWrapper<SplitStream<WebSocketStream>>;

        type WriteHalf = SendWrapper<SplitSink<WebSocketStream, Message>>;

        type ConnectInfo = url::Url;

        type AcceptInfo = SocketAddr;

        type AcceptStream = OwnedIncoming;

        async fn accept_loop(
            _accept_info: Self::AcceptInfo,
            _: Self::NetworkSettings,
        ) -> Result<Self::AcceptStream, NetworkError> {
            panic!("Can't create servers on WASM");
        }

        async fn connect_task(
            connect_info: Self::ConnectInfo,
            _network_settings: Self::NetworkSettings,
        ) -> Result<Self::Socket, NetworkError> {
            info!("Beginning connection");
            let stream = tokio_tungstenite_wasm::connect(connect_info)
                .await
                .map_err(|error| match error {
                    tokio_tungstenite_wasm::Error::ConnectionClosed => {
                        NetworkError::Error(format!("Connection Closed"))
                    }
                    tokio_tungstenite_wasm::Error::AlreadyClosed => {
                        NetworkError::Error(format!("Connection Already Closed"))
                    }
                    tokio_tungstenite_wasm::Error::Io(err) => {
                        NetworkError::Error(format!("IO Error: {}", err))
                    }
                    tokio_tungstenite_wasm::Error::Tls(err) => {
                        NetworkError::Error(format!("TLS Error: {}", err))
                    }
                    tokio_tungstenite_wasm::Error::Capacity(err) => {
                        NetworkError::Error(format!("Capacity Error: {}", err))
                    }
                    tokio_tungstenite_wasm::Error::Protocol(err) => {
                        NetworkError::Error(format!("Protocol Error: {}", err))
                    }
                    tokio_tungstenite_wasm::Error::WriteBufferFull(err) => {
                        NetworkError::Error(format!("Write Buffer Full: {}", err))
                    }
                    tokio_tungstenite_wasm::Error::Utf8 => {
                        NetworkError::Error(format!("UTF8 Encoding Error"))
                    }
                    tokio_tungstenite_wasm::Error::AttackAttempt => {
                        NetworkError::Error(format!("Attack Attempt Detected"))
                    }
                    tokio_tungstenite_wasm::Error::Url(err) => {
                        NetworkError::Error(format!("Url Error: {}", err))
                    }
                    tokio_tungstenite_wasm::Error::Http(err) => {
                        NetworkError::Error(format!("HTTP Error: {:?}", err))
                    }
                    tokio_tungstenite_wasm::Error::HttpFormat(err) => {
                        NetworkError::Error(format!("HTTP Format Error: {}", err))
                    }
                    tokio_tungstenite_wasm::Error::BlobFormatUnsupported => {
                        NetworkError::Error(format!("Blob Format Unsupported"))
                    }
                    tokio_tungstenite_wasm::Error::UnknownFormat => {
                        NetworkError::Error(format!("Invalid Format"))
                    }
                })?;
            info!("Connected!");
            return Ok(SendWrapper::new(stream));
        }

        async fn recv_loop(
            mut read_half: Self::ReadHalf,
            messages: Sender<NetworkPacket>,
            _settings: Self::NetworkSettings,
            network_packet_de: fn(data: NetworkSerializedData) -> Result<NetworkPacket, String>,
        ) {
            loop {
                let message = match read_half.next().await {
                    Some(message) => match message {
                        Ok(message) => message,
                        Err(err) => match err {
                            tokio_tungstenite_wasm::Error::ConnectionClosed
                            | tokio_tungstenite_wasm::Error::AlreadyClosed => {
                                error!("Connection Closed");
                                break;
                            }
                            _ => {
                                error!("Nonfatal error detected: {}", err);
                                continue;
                            }
                        },
                    },
                    None => {
                        continue;
                    }
                };

                let packet = match message {
                    Message::Text(text) => {
                        if cfg!(feature = "json") {
                            match network_packet_de(NetworkSerializedData::String(text)) {
                                Ok(packet) => packet,
                                Err(err) => {
                                    error!("Failed to decode network packet from: {}", err);
                                    break;
                                }
                            }
                        } else {
                            error!("String message recieved and not supported. Enable JSON feature to accept string messages");
                            break;
                        }
                    }
                    Message::Binary(binary) => {
                        match network_packet_de(NetworkSerializedData::Binary(binary)) {
                            Ok(packet) => packet,
                            Err(err) => {
                                error!("Failed to decode network packet from: {}", err);
                                break;
                            }
                        }
                    }

                    Message::Close(_) => {
                        error!("Connection Closed");
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
            network_packet_ser: fn(data: NetworkPacket) -> Result<NetworkSerializedData, String>,
        ) {
            while let Ok(message) = messages.recv().await {
                let encoded = match network_packet_ser(message) {
                    Ok(encoded) => encoded,
                    Err(err) => {
                        error!("Could not encode packet: {}", err);
                        continue;
                    }
                };

                trace!("Sending the content of the message!");
                match encoded {
                    NetworkSerializedData::String(text) => match write_half
                        .send(async_tungstenite::tungstenite::Message::Text(text))
                        .await
                    {
                        Ok(_) => (),
                        Err(err) => {
                            error!("Could not send packet: {}", err);
                            break;
                        }
                    },
                    NetworkSerializedData::Binary(vec) => match write_half
                        .send(async_tungstenite::tungstenite::Message::Binary(vec))
                        .await
                    {
                        Ok(_) => (),
                        Err(err) => {
                            error!("Could not send packet: {}", err);
                            break;
                        }
                    },
                }

                trace!("Succesfully written all!");
            }
        }

        fn split(combined: Self::Socket) -> (Self::ReadHalf, Self::WriteHalf) {
            let (write, read) = combined.take().split();
            (SendWrapper::new(read), SendWrapper::new(write))
        }
    }

    #[derive(Clone, Debug, Resource, Deref, DerefMut)]
    #[allow(missing_copy_implementations)]
    /// Settings to configure the network
    ///
    /// Note that on WASM this is currently ignored and defaults are used
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
        type Item = SendWrapper<WebSocketStream>;

        fn poll_next(
            self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            panic!("WASM does not support servers");
        }
    }
}
