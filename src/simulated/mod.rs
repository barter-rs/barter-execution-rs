use crate::{error::ExecutionError, model::{
    ClientOrderId,
    order::{Order, Open},
    balance::Balance
}, OrderId, RequestCancel, RequestOpen, SymbolBalance};
use barter_integration::model::{Instrument, Symbol};
use std::{
    collections::HashMap,
    time::Duration,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Config {
    pub instruments: Vec<Instrument>,
    pub balances: HashMap<Symbol, Balance>,
    pub fees_percent: f64,
    pub latency: Duration,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Exchange {
    pub fees_percent: f64,
    pub latency: Duration,
    pub balances: HashMap<Symbol, Balance>,
    pub markets: HashMap<Instrument, SimulatedMarket>
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct SimulatedMarket {
    pub client_orders: HashMap<ClientOrderId, Order<Open>>,
    pub book: (),
}

impl Exchange {
    pub fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError> {
        std::thread::sleep(self.latency);

        self.markets
            .values()
            .map(|market| market.client_orders.values())
            .flatten()
            .cloned()
            .map(Ok)
            .collect::<Result<Vec<Order<Open>>, ExecutionError>>()
    }

    pub fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError> {
        std::thread::sleep(self.latency);

        self.balances
            .clone()
            .into_iter()
            .map(|(symbol, balance) | Ok(SymbolBalance::new(symbol, balance)))
            .collect::<Result<Vec<SymbolBalance>, ExecutionError>>()
    }

    pub fn open_orders(&mut self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExecutionError>> {
        std::thread::sleep(self.latency);

        open_requests
            .into_iter()
            .map(|request| {
                // Try to find the SimulatedMarket for the RequestOpen Instrument
                let market = self
                    .markets
                    .get_mut(&request.instrument)
                    .ok_or_else(|| ExecutionError::Simulated(format!(
                        "received Order<RequestOpen> for unknown Instrument: {}", request.instrument
                    )))?;

                // Map Order<RequestOpen> to Order<Open>
                let open = Order {
                    exchange: request.exchange,
                    instrument: request.instrument,
                    cid: request.cid,
                    state: Open {
                        id: OrderId::from(Uuid::new_v4().to_string()),
                        side: request.state.side,
                        price: request.state.price,
                        quantity: request.state.quantity,
                        filled_quantity: 0.0
                    }
                };

                // Insert a SimulatedMarket Order<Open>
                market
                    .client_orders
                    .insert(open.cid, open.clone())
                    .ok_or_else(|| ExecutionError::Simulated(format!(
                        "received Order<RequestOpen> with duplicate ClientOrderId: {:?}", open.cid
                    )))?;

                Ok(open)
            })
            .collect::<Vec<Result<Order<Open>, ExecutionError>>>()

    }

    pub fn cancel_orders(&mut self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<OrderId, ExecutionError>> {
        std::thread::sleep(self.latency);

        cancel_requests
            .into_iter()
            .map(|request| {
                // Try to find the SimulatedMarket for the RequestCancel Instrument
                let market = self
                    .markets
                    .get_mut(&request.instrument)
                    .ok_or_else(|| ExecutionError::Simulated(format!(
                        "received Order<RequestCancel> for unknown Instrument: {}", request.instrument
                    )))?;

                // Try to find an Order<Open> to remove for the RequestCancel ClientOrderId
                market
                    .client_orders
                    .remove(&request.cid)
                    .map(|removed| removed.state.id)
                    .ok_or_else(|| ExecutionError::Simulated(format!(
                        "received Order<RequestCancel> for unknown ClientOrderId: {:?}", request.cid,
                    )))
            })
            .collect::<Vec<Result<OrderId, ExecutionError>>>()
    }

    pub fn cancel_orders_all(&mut self) -> Result<(), ExecutionError> {
        std::thread::sleep(self.latency);

        let _ = self.markets
            .values_mut()
            .map(|market| market.client_orders.drain());

        Ok(())
    }
}

