use bevy::app::App;
use bevy_eventwork::{
    managers::NetworkProvider, AppNetworkMessage, NetworkMessage, NetworkPacket,
    NetworkSerializedData,
};
use serde::{de::DeserializeOwned, Serialize};

pub trait EventworkSerdeJsonAppExt {
    /// Registers a new network message using JSON serialization
    fn register_json_message<T: NetworkMessage, NP: NetworkProvider>(&mut self) -> &mut Self;
}

impl EventworkSerdeJsonAppExt for App {
    fn register_json_message<T: NetworkMessage, NP: NetworkProvider>(&mut self) -> &mut Self {
        self.register_message_with::<T, NP>(json_de::<T>, json_ser::<T>)
    }
}

/// Default bincode based deserialization fn. Only supports binary data types.
pub fn json_de<T: DeserializeOwned>(data: &NetworkSerializedData) -> Result<T, String> {
    let NetworkSerializedData::String(string) = data else {
        return Err("Expected String data found Binary data".to_string());
    };
    serde_json::from_str(string).map_err(|err| err.to_string())
}

/// Default bincode based serialization fn. Only supports binary data types.
pub fn json_ser<T: Serialize>(data: &T) -> Result<NetworkSerializedData, String> {
    serde_json::to_string(data)
        .map_err(|err| err.to_string())
        .map(|data| NetworkSerializedData::String(data))
}

/// Default bincode based [`NetworkPacket`] deserialization fn. Only supports binary data types.
pub fn json_network_packet_de(data: NetworkSerializedData) -> Result<NetworkPacket, String> {
    let NetworkSerializedData::String(string) = data else {
        return Err("Expected String data found Binary data".to_string());
    };
    serde_json::from_str(&string).map_err(|err| err.to_string())
}

/// Default bincode based [`NetworkPacket`] serialization fn. Only supports binary data types.
pub fn json_network_packet_ser(data: NetworkPacket) -> Result<NetworkSerializedData, String> {
    serde_json::to_string(&data)
        .map_err(|err| err.to_string())
        .map(|data| NetworkSerializedData::String(data))
}
