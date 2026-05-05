use thiserror::Error;
use tokio_tungstenite::tungstenite;

/// Errors returned by this library.
#[derive(Debug, Error)]
pub enum Error {
    /// A WebSocket transport error.
    #[error("websocket error: {0}")]
    WebSocket(Box<tungstenite::Error>),
    /// A JSON serialization or deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A message was not valid.
    #[error("invalid message: {0}")]
    InvalidMessage(#[from] Box<dyn std::error::Error>),
}

impl From<tungstenite::Error> for Error {
    fn from(e: tungstenite::Error) -> Self {
        Error::WebSocket(Box::new(e))
    }
}
