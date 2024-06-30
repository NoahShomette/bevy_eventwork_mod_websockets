use bevy_eventwork::{error::NetworkError, NetworkSerializer};

#[derive(Default)]
pub struct JsonSerializer;

impl NetworkSerializer for JsonSerializer {
    fn serialize<T: ?Sized>(value: &T) -> Result<Vec<u8>, bevy_eventwork::error::NetworkError>
    where
        T: serde::Serialize,
    {
        match serde_json::to_vec(value) {
            Ok(data) => Ok(data),
            Err(error) => {
                println!("Failed to serialize message: {}", error);
                Err(NetworkError::Serialization)
            }
        }
    }

    fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T, bevy_eventwork::error::NetworkError>
    where
        T: serde::de::Deserialize<'a>,
    {
        match serde_json::from_slice(bytes) {
            Ok(data) => Ok(data),
            Err(error) => {
                println!("Failed to deserialize message: {}", error);
                Err(NetworkError::Serialization)
            }
        }
    }
}
