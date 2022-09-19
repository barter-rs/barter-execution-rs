
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Order<State> {
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub cid: ClientOrderId,
    pub state: State,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Deserialize, Serialize)]
pub struct RequestOpen {
    pub kind: OrderKind,
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum OrderKind {
    Market,
    Limit,
    PostOnly,
    ImmediateOrCancel
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct InFlight;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct RequestCancel;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Open {
    pub id: OrderId,
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Cancelled;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderId(pub String);

impl OrderId {
    pub fn new<S>(id: S) -> Self
        where
            S: Into<String>
    {
        Self(id.into())
    }
}

impl From<&Order<RequestOpen>> for Order<InFlight> {
    fn from(request: &Order<RequestOpen>) -> Self {
        Self {
            exchange: request.exchange.clone(),
            instrument: request.instrument.clone(),
            cid: request.cid,
            state: InFlight
        }
    }
}