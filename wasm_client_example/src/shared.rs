use bevy::prelude::*;
use bevy_eventwork::NetworkMessage;
use bevy_eventwork_mod_websockets::WebSocketProvider;
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

#[allow(unused)]
pub fn client_register_network_messages(app: &mut App) {
    use bevy_eventwork::AppNetworkMessage;

    // The client registers messages that arrives from the server, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.listen_for_message::<NewChatMessage, WebSocketProvider>();
}

#[allow(unused)]
pub fn server_register_network_messages(app: &mut App) {
    use bevy_eventwork::AppNetworkMessage;

    // The server registers messages that arrives from a client, so that
    // it is prepared to handle them. Otherwise, an error occurs.
    app.listen_for_message::<UserChatMessage, WebSocketProvider>();
}
