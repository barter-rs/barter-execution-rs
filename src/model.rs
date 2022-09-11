use uuid::Uuid;


// Todo:
//  - Do I want a batch ExchangeRequest
//   '--> eg/ CancelAll on all exchanges, or return HashMap<Exchange, ConnectionStatus>, etc.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientOrderId(pub Uuid);

#[derive(Clone, Copy, Debug)]
pub enum ConnectionStatus {
    Connected,
    CancelOnly,
    Disconnected,
}

// Todo:
//   - Better name for this? This is the equivilant to ExchangeId...
//    '--> renamed to ClientId for now to avoid confusion in development
#[derive(Clone, Debug)]
pub enum ClientId {
    Simulated(super::simulated::Config),
}

