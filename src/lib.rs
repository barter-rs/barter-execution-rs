#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

///! # Barter-Execution

pub mod error;

use crate::error::ClientError;
use barter::execution::FillEvent;
use barter::portfolio::OrderEvent;
use async_trait::async_trait;
use tokio::sync::oneshot;

type ClientResult<T> = Result<T, ClientError>;

#[derive(Copy, Clone, Debug)]
pub struct OrderCancelEvent;

#[derive(Debug)]
pub enum Command {
    OpenOrder((OrderEvent, oneshot::Sender<ClientResult<FillEvent>>)),
}

#[async_trait]
pub trait ExecutionClient {
    // Todo: Why Result<Option<T>>>? For barter trait compatibility?
    async fn open_order(&self, request: OrderEvent) -> ClientResult<Option<FillEvent>>;
    async fn cancel_order(&self, request: OrderCancelEvent) -> ClientResult<Option<String>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {



    }
}