use barter_integration::socket::error::SocketError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Socket error due to: {0}")]
    SocketError(#[from] SocketError),
}