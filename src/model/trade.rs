
/// Normalised Barter private [`Trade`] model.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Trade {
    pub id: TradeId,
    pub order_id: OrderId,
    pub instrument: Instrument,
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    pub fees: f64,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct TradeId(pub String);

impl TradeId {
    pub fn new<S>(id: S) -> Self
    where
        S: Into<String>
    {
        Self(id.into())
    }
}