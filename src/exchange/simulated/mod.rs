use crate::{
    AccountEvent, ExecutionClient, ExecutionError, OrderId, RequestCancel, RequestOpen, SymbolBalance, simulated,
    ClientId,
    model::{
        order::{Order, Open}
    },
};
use std::{
    sync::Arc
};
use async_trait::async_trait;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::warn;

#[derive(Debug)]
struct SimulatedExecution {
    pub event_tx: mpsc::UnboundedSender<AccountEvent>,
    pub exchange: Arc<RwLock<simulated::Exchange>>,
}

#[async_trait]
impl ExecutionClient for SimulatedExecution {
    const CLIENT: ClientId = ClientId::Simulated;
    type Config = Arc<RwLock<simulated::Exchange>>;

    async fn init(exchange: Self::Config, event_tx: mpsc::UnboundedSender<AccountEvent>) -> Self {
        Self {
            event_tx,
            exchange,
        }
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError> {
        self.exchange
            .read()
            .fetch_orders_open()
    }

    async fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError> {
        self.exchange
            .read()
            .fetch_balances()
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Result<Vec<Order<Open>>, ExecutionError> {
        let open_results = self.exchange
            .write()
            .open_orders(open_requests);

        open_results
            .into_iter()
            .map(|open_result| {
                if let Err(error) = &open_result {
                    warn!(client = %Self::CLIENT, ?error, "failed to open order")
                }
                open_result
            })
            .collect::<Result<Vec<_>, ExecutionError>>()
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Result<Vec<OrderId>, ExecutionError> {
        let cancel_results = self.exchange
            .write()
            .cancel_orders(cancel_requests);

        cancel_results
            .into_iter()
            .map(|cancel_result| {
                if let Err(error) = &cancel_result {
                    warn!(client = %Self::CLIENT, ?error, "failed to cancel order")
                }
                cancel_result
            })
            .collect::<Result<Vec<_>, ExecutionError>>()
    }

    async fn cancel_orders_all(&self) -> Result<(), ExecutionError> {
        self.exchange
            .write()
            .cancel_orders_all()
    }
}






















