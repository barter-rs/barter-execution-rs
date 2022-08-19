use thiserror::Error;
use barter_integration::error::SocketError;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Socket error due to: {0}")]
    SocketError(#[from] SocketError),

    #[error("Failed to send message due dropped receiver")]
    SendError,

    #[error("Failed to receive message due to dropped sender")]
    ReceiveError,

    // #[error("Failed to establish websocket connection due to failed websocket handshake")]
    // WebSocketConnect(#[source] tungstenite::error::Error),
    //
    // #[error("Failed to write data via websocket connection")]
    // WebSocketWrite(#[source] tungstenite::error::Error),
    //
    // #[error("Failed to read data via websocket connection")]
    // WebSocketRead(#[source] tungstenite::error::Error),
    //
    // #[error("Failed to deserialize/serialize JSON due to: {0}")]
    // JsonSerDeError(#[from] serde_json::Error),
}