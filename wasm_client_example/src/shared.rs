use bevy::prelude::*;
use bevy_eventwork::{
    managers::network_request::{
        AppNetworkRequestMessage, AppNetworkResponseMessage, RequestMessage,
    },
    NetworkMessage,
};
use bevy_eventwork_mod_websockets::{serde_json::EventworkSerdeJsonAppExt, WebSocketProvider};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserChatMessage {
    pub message: String,
}

impl NetworkMessage for UserChatMessage {
    const NAME: &'static str = "example:UserChatMessage";
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewChatMessage {
    pub name: String,
    pub message: String,
}

impl NetworkMessage for NewChatMessage {
    const NAME: &'static str = "example:NewChatMessage";
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestStatus;

impl RequestMessage for RequestStatus {
    /// The type of message that the server will send back to the client.
    /// It must implement [`NetworkMessage`]
    type ResponseMessage = StatusResponse;

    /// A unique identifying name for the request message.
    const REQUEST_NAME: &'static str = "client_request_status";
}

/// The response that the server will eventually return to the client
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusResponse {
    pub response: bool,
}

impl NetworkMessage for StatusResponse {
    const NAME: &'static str = "client_request_status_response";
}

#[allow(unused)]
pub fn register_network_messages(app: &mut App) {
    use bevy_eventwork::AppNetworkMessage;

    // The client registers messages that arrives from the server, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.register_json_message::<NewChatMessage, WebSocketProvider>();
    app.register_json_message::<UserChatMessage, WebSocketProvider>();
}

/// Register request messages that we want the client to send.
#[allow(unused)]
pub fn client_register_request_messages(app: &mut App) {
    use bevy_eventwork::AppNetworkMessage;

    app.register_send_request_message::<RequestStatus, WebSocketProvider>();
}

/// Register request messages that we want the server to receive.
#[allow(unused)]
pub fn server_register_request_messages(app: &mut App) {
    use bevy_eventwork::AppNetworkMessage;
    app.register_receive_request_message::<RequestStatus, WebSocketProvider>();
}
