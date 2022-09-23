use self::{
    balance::SymbolBalance,
    order::{Cancelled, Open, Order},
    trade::Trade,
};
use barter_integration::model::Exchange;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod balance;
pub mod order;
pub mod trade;

/// Normalised Barter [`AccountEvent`] containing metadata about the included
/// [`AccountEventKind`] variant. Produced by [`ExecutionClients`](crate::ExecutionClient).
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct AccountEvent {
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub kind: AccountEventKind,
}

/// Defines the type of Barter [`AccountEvent`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum AccountEventKind {
    // HTTP Return
    Balances(Vec<SymbolBalance>),
    OrdersOpen(Vec<Order<Open>>),
    OrdersNew(Vec<Order<Open>>),
    OrdersCancelled(Vec<Order<Cancelled>>),

    // WebSocket Return
    Balance(SymbolBalance),
    Trade(Trade),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientOrderId(pub Uuid);

#[derive(Clone, Copy, Debug)]
pub enum ClientStatus {
    Connected,
    CancelOnly,
    Disconnected,
}