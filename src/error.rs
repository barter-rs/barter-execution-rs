use thiserror::Error;
use barter_integration::error::SocketError;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Socket error due to: {0}")]
    Socket(#[from] SocketError),
}