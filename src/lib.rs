///! # Barter-Execution

#[warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

pub mod error;
pub mod event_loop;
pub mod socket;

use crate::error::ClientError;
use barter::execution::FillEvent;
use barter::portfolio::OrderEvent;
use async_trait::async_trait;
use tokio::sync::oneshot;

type ClientResult<T> = Result<T, ClientError>;

// Todo: Could have an ExecutionClient that relies on an ExchangeClient? That way the Trader.run()
//       event loop still calls self.execution.open_order() & keeps generic w/ layer of abstraction.
//       '--> This would also allow the ExecutionClient to keep spawning an event loop if there are
//            failures!
// Todo: Trader would have a notification_rx and would self.receive_notifications() just below
//         receiving remote commands
// Todo: Work through cancel order because it may be clearer since it 'always get filled'
// Todo: Start having a look at futures docs to get a flavour for it

pub enum Command {
    OpenOrder((OrderEvent, oneshot::Sender<ClientResult<FillEvent>>)),
}

#[async_trait]
pub trait ExecutionClient {
    async fn open_order(&self, request: OrderEvent) -> ClientResult<Option<FillEvent>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {



    }
}