use crate::{AccountEvent, ExecutionClient, ExecutionError, model::{
    ClientOrderId,
    order::{Order, Open},
    balance::Balance,
}, OrderId, RequestCancel, RequestOpen, SymbolBalance};
use barter_integration::model::{Instrument, Symbol};
use std::{
    collections::HashMap,
    time::Duration,
};
use std::sync::Arc;
use async_trait::async_trait;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Config {
    pub instruments: Vec<Instrument>,
    pub balances: HashMap<Symbol, Balance>,
    pub fees_percent: f64,
    pub latency: Duration,
}

#[derive(Debug)]
struct SimulatedExecution {
    pub event_tx: mpsc::UnboundedSender<AccountEvent>,
    pub fees_percent: f64,
    pub latency: Duration,
    pub balances: HashMap<Symbol, Balance>,
    pub markets: HashMap<Instrument, SimulatedMarket>,
}

#[derive(Debug, Default)]
struct SimulatedMarket {
    client_orders: HashMap<ClientOrderId, Order<Open>>,
}

#[async_trait]
impl ExecutionClient for SimulatedExecution {
    type Config = Config;

    async fn init(config: Self::Config, event_tx: mpsc::UnboundedSender<AccountEvent>) -> Self {
        Self {
            event_tx,
            fees_percent: config.fees_percent,
            latency: config.latency,
            balances: config.balances,
            markets: config
                .instruments
                .into_iter()
                .map(|instrument| (instrument, SimulatedMarket::default()))
                .collect::<HashMap<Instrument, SimulatedMarket>>()
        }
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError> {
        tokio::time::sleep(self.latency).await;

        self.markets
            .values()
            .map(|market| market.client_orders.values())
            .flatten()
            .cloned()
            .map(Ok)
            .collect::<Result<Vec<Order<Open>>, ExecutionError>>()
    }

    async fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError> {
        tokio::time::sleep(self.latency).await;

        self.balances
            .clone()
            .into_iter()
            .map(|(symbol, balance) | Ok(SymbolBalance::new(symbol, balance)))
            .collect::<Result<Vec<SymbolBalance>, ExecutionError>>()
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Result<Vec<Order<Open>>, ExecutionError> {
        tokio::time::sleep(self.latency).await;

        // If Market,

        todo!()
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Result<Vec<OrderId>, ExecutionError> {
        tokio::time::sleep(self.latency).await;


        Ok(())
    }

    async fn cancel_orders_all(&self) -> Result<(), ExecutionError> {
        tokio::time::sleep(self.latency).await;

        // Todo: Exchange needs to be on separate thread due to due &self requirement of client
        self.markets
            .values()
            .map(|mut market| market.client_orders.drain())
            .flatten()
            .for_each(|(id, order)| {

            })

        Ok(())
    }
}






















