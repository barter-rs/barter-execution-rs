use std::{
    time::Duration,
    collections::HashMap
};
use crate::{
    ConnectionStatus, ExchangeClient,
    model::ClientOrderId,
};
use barter_integration::model::{Instrument, Side, Symbol};


pub struct Config {
    fees: (),
    latency: Duration,
}

pub struct SimulatedExchange {
    pub config: Config,
    pub connection_status: ConnectionStatus,
    pub instruments: Vec<Instrument>,
    pub open: HashMap<ClientOrderId, Order>,
    pub balances: HashMap<Symbol, Balance>,
}

pub struct Balance {
    pub total: f64,
    pub available: f64,
}

pub struct Order {
    pub direction: Side,
    pub price: f64,
    pub quantity: f64,
}



impl ExchangeClient for SimulatedExchange {
    fn instruments(&self) -> &[Instrument] {
        &self.instruments
    }

    fn connection_status(&self) -> ConnectionStatus {
        self.connection_status
    }

    fn fetch_orders_open(&self) -> () {

    }

    fn fetch_balances(&self) -> () {
        todo!()
    }

    fn open_order(&self) -> () {
        todo!()
    }

    fn open_order_batch(&self) -> () {
        todo!()
    }

    fn cancel_order_by_id(&self) -> () {
        todo!()
    }

    fn cancel_order_by_instrument(&self) -> () {
        todo!()
    }

    fn cancel_order_by_batch(&self) -> () {
        todo!()
    }

    fn cancel_order_all(&self) -> () {
        todo!()
    }
}