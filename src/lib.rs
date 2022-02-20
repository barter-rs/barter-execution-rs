///! # Barter-Execution

#[warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

pub mod error;
pub mod event_loop;

use crate::error::ClientError;
use barter::execution::error::ExecutionError;
use barter::execution::FillEvent;
use barter::portfolio::{Balance, OrderEvent};
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

type ClientResult<T> = Result<T, ClientError>;

// Todo: Trader would have a notification_rx and would self.receive_notifications() just below
//         receiving remote commands

pub enum Command {
    OpenOrder((OrderEvent, oneshot::Sender<ClientResult<FillEvent>>)),
}

#[async_trait]
pub trait ExecutionClient {
    async fn open_order(&self, request: OrderEvent) -> ClientResult<FillEvent>;
    // Todo: Work through cancel order because it may be clearer since it 'always get filled'
    // Todo: Start having a look at futures docs to get a flavour for it
}

struct LiveExecution {
    command_tx: mpsc::Sender<Command>,
}

#[async_trait]
impl ExecutionClient for LiveExecution {
    async fn open_order(&self, request: OrderEvent) -> ClientResult<FillEvent> {
        // Create oneshot channel to receive response
        let (response_tx, response_rx) = oneshot::channel();

        // Send a Command::OpenOrder to the event loop
        self.command_tx
            .send(Command::OpenOrder((request, response_tx)))
            .await
            .map_err(|_| ClientError::SendError)?;

        // Receive response
        response_rx
            .await
            .map_err(|_| ClientError::ReceiveError)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {



    }
}