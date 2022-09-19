use barter_integration::error::SocketError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Socket error due to: {0}")]
    Socket(#[from] SocketError),
}
