use std::{
    collections::HashMap,
    time::Duration
};
use crate::{
    ConnectionStatus,
    model::ClientOrderId,
};
use barter_integration::model::{Instrument, Side, Symbol};

#[derive(Clone, Debug)]
pub struct Config {
    fees: (),
    latency: Duration,
}

#[derive(Clone, Debug)]
pub struct SimulatedExchange {
    pub config: Config,
    pub connection_status: ConnectionStatus,
    pub instruments: Vec<Instrument>,
    pub open: HashMap<ClientOrderId, Order>,
    pub balances: HashMap<Symbol, Balance>,
}

#[derive(Clone, Debug)]
pub struct Balance {
    pub total: f64,
    pub available: f64,
}

#[derive(Clone, Debug)]
pub struct Order {
    pub direction: Side,
    pub price: f64,
    pub quantity: f64,
}