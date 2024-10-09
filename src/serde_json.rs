use bevy::app::App;
use bevy_eventwork::{
    managers::{
        network_request::{
            AppNetworkRequestMessage, AppNetworkResponseMessage, RequestInternal, RequestMessage,
            ResponseInternal,
        },
        NetworkProvider,
    },
    AppNetworkMessage, NetworkDataTypes, NetworkMessage, NetworkPacket, NetworkSerializedData,
};
use serde::{de::DeserializeOwned, Serialize};

pub trait EventworkSerdeJsonAppExt {
    /// Registers a new network message using JSON serialization
    fn register_json_message<T: NetworkMessage, NP: NetworkProvider>(&mut self) -> &mut Self;

    /// Registers a new request message to be received over the network using JSON serialization
    fn register_receive_request_json_message<T: RequestMessage, NP: NetworkProvider>(
        &mut self,
    ) -> &mut Self;

    /// Registers a new request message to be sent over the network using JSON serialization
    fn register_send_request_json_message<T: RequestMessage, NP: NetworkProvider>(
        &mut self,
    ) -> &mut Self;
}

impl EventworkSerdeJsonAppExt for App {
    fn register_json_message<T: NetworkMessage, NP: NetworkProvider>(&mut self) -> &mut Self {
        self.register_message_with::<T, NP>(
            NetworkDataTypes::Text,
            json_de::<T>,
            json_ser::<T>,
            json_network_packet_de,
            json_network_packet_ser,
        )
    }

    fn register_receive_request_json_message<T: RequestMessage, NP: NetworkProvider>(
        &mut self,
    ) -> &mut Self {
        self.register_receive_request_message_with::<T, NP>(
            NetworkDataTypes::Text,
            json_de::<RequestInternal<T>>,
            json_ser::<RequestInternal<T>>,
            json_network_packet_de,
            json_network_packet_ser,
            json_de::<ResponseInternal<T::ResponseMessage>>,
            json_ser::<ResponseInternal<T::ResponseMessage>>,
        )
    }

    fn register_send_request_json_message<T: RequestMessage, NP: NetworkProvider>(
        &mut self,
    ) -> &mut Self {
        self.register_send_request_message_with::<T, NP>(
            NetworkDataTypes::Text,
            json_de::<RequestInternal<T>>,
            json_ser::<RequestInternal<T>>,
            json_network_packet_de,
            json_network_packet_ser,
            json_de::<ResponseInternal<T::ResponseMessage>>,
            json_ser::<ResponseInternal<T::ResponseMessage>>,
        )
    }
}

/// Default bincode based deserialization fn. Only supports binary data types.
pub fn json_de<T: DeserializeOwned>(data: &NetworkSerializedData) -> Result<T, String> {
    let NetworkSerializedData::Text(string) = data else {
        return Err("Expected String data found Binary data".to_string());
    };
    serde_json::from_str(string).map_err(|err| err.to_string())
}

/// Default bincode based serialization fn. Only supports binary data types.
pub fn json_ser<T: Serialize>(data: &T) -> Result<NetworkSerializedData, String> {
    serde_json::to_string(data)
        .map_err(|err| err.to_string())
        .map(|data| NetworkSerializedData::Text(data))
}

/// Default bincode based [`NetworkPacket`] deserialization fn. Only supports binary data types.
pub fn json_network_packet_de(data: NetworkSerializedData) -> Result<NetworkPacket, String> {
    let NetworkSerializedData::Text(string) = data else {
        return Err("Expected String data found Binary data".to_string());
    };
    serde_json::from_str(&string).map_err(|err| err.to_string())
}

/// Default bincode based [`NetworkPacket`] serialization fn. Only supports binary data types.
pub fn json_network_packet_ser(data: NetworkPacket) -> Result<NetworkSerializedData, String> {
    serde_json::to_string(&data)
        .map_err(|err| err.to_string())
        .map(|data| NetworkSerializedData::Text(data))
}
