#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

use std::fmt::{Display, Formatter};
///! # Barter-Execution
///! Todo:
use crate::{
    error::ExecutionError,
    model::{
        balance::SymbolBalance,
        order::{Open, Order, OrderId, RequestCancel, RequestOpen},
        AccountEvent,
    },
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub mod builder;
/// Todo:
pub mod error;
/// Contains `ExchangeClient` implementations for specific exchanges.
pub mod exchange;
pub mod model;
pub mod simulated;

// Todo:
//  - Add Health/ClientStatus to Client, AccountEvent, etc.
//  - Perhaps we want a `cancel_orders_instrument()` with matching ExecutionRequest
//    '--> Or cancel_orders_all(instrument: Option<Instrument>)
//  - Vec<Result> or Result<Vec> for batch open & cancels?

#[async_trait]
pub trait ExecutionClient {
    const CLIENT: ClientId;
    type Config;
    async fn init(config: Self::Config, event_tx: mpsc::UnboundedSender<AccountEvent>) -> Self;
    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError>;
    async fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError>;
    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Result<Vec<Order<Open>>, ExecutionError>;
    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Result<Vec<OrderId>, ExecutionError>;
    async fn cancel_orders_all(&self) -> Result<(), ExecutionError>;
}

/// Used to uniquely identify an [`ExecutionClient`] implementation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "client", rename_all = "snake_case")]
pub enum ClientId {
    Simulated,
    Ftx,
}

impl Display for ClientId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ClientId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientId::Simulated => "simulated",
            ClientId::Ftx => "ftx"
        }
    }
}

pub mod test_util {
    use crate::{
        Open, Order, OrderId,
        model::ClientOrderId
    };
    use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
    use barter_data::ExchangeId;
    use uuid::Uuid;

    pub fn order_open(side: Side, price: f64, quantity: f64, filled: f64) -> Order<Open> {
        Order {
            exchange: Exchange::from("exchange"),
            instrument: Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual)),
            cid: ClientOrderId(Uuid::new_v4()),
            state: Open {
                id: OrderId::from("id"),
                side,
                price,
                quantity,
                filled_quantity: filled
            }
        }
    }
}