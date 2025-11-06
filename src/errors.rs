//! Error types for E3DC-MQTT bridge
//!
//! Uses thiserror for ergonomic error definitions.
//! These errors can be converted to anyhow::Error in the main application.

/// E3DC connection and communication errors
#[derive(Debug, thiserror::Error)]
pub enum E3dcError {
    #[error("Failed to connect to E3DC at {host}: {reason}")]
    ConnectionFailed { host: String, reason: String },

    #[error("Failed to query E3DC data: {0}")]
    QueryFailed(String),

    #[error("Failed to parse E3DC response: {0}")]
    ParseError(String),

    #[error("Missing tag: {0}")]
    MissingTag(u32),

    #[error("Missing data in tag: {0}")]
    MissingData(u32),

    #[error("Invalid Datatype expected: {0}")]
    Type(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// MQTT connection and publishing errors
#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("Failed to publish message to topic '{topic}': {reason}")]
    PublishFailed { topic: String, reason: String },

    #[error("Failed to serialize data: {error:?}")]
    SerializationError { error: serde_json::Error },

    #[error("MQTT client error: {0}")]
    ClientError(String),
}
