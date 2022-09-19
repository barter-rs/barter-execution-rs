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

    ExecutionError(ExecutionError),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientOrderId(pub Uuid);

#[derive(Clone, Copy, Debug)]
pub enum ClientStatus {
    Connected,
    CancelOnly,
    Disconnected,
}