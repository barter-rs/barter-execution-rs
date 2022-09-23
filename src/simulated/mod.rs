use crate::{
    AccountEvent, Open, Order, RequestOpen, RequestCancel, ExecutionError, SymbolBalance, OrderId,
    model::{ClientOrderId, balance::Balance},
};
use barter_integration::model::{Instrument, Side, Symbol};
use barter_data::model::PublicTrade;
use std::{
    collections::HashMap,
    time::Duration,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

pub mod matches;
pub mod engine;

#[derive(Debug)]
pub enum SimulatedEvent {
    MarketTrade((Instrument, PublicTrade)),
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(oneshot::Sender<Result<Vec<SymbolBalance>, ExecutionError>>),
    OpenOrders((Vec<Order<RequestOpen>>, oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>)),
    CancelOrders((Vec<Order<RequestCancel>>, oneshot::Sender<Vec<Result<OrderId, ExecutionError>>>)),
    CancelOrdersAll(oneshot::Sender<Result<(), ExecutionError>>)
}

#[derive(Debug)]
pub struct SimulatedExchange {
    pub fees_percent: f64,
    pub latency: Duration,
    pub event_account_tx: mpsc::UnboundedSender<AccountEvent>,
    pub event_simulated_rx: mpsc::UnboundedReceiver<SimulatedEvent>,
    pub balances: HashMap<Symbol, Balance>,
    pub markets: HashMap<Instrument, ClientOrders>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ClientOrders {
    pub bids: Vec<Order<Open>>,
    pub asks: Vec<Order<Open>>,
}

impl ClientOrders {
    fn num_orders(&self) -> usize {
        self.bids.len() + self.asks.len()
    }
}

// Todo: Add notifications for any relevant operations, eg/ balance updates upon opens?

impl SimulatedExchange {
    pub async fn run(mut self) {
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                SimulatedEvent::MarketTrade((instrument, trade)) => {

                }
                SimulatedEvent::FetchOrdersOpen(response_tx) => {
                    self.fetch_orders_open(response_tx).await
                }
                SimulatedEvent::FetchBalances(response_tx) => {
                    self.fetch_balances(response_tx).await
                }
                SimulatedEvent::OpenOrders((open_requests, response_tx)) => {
                    self.open_orders(open_requests, response_tx).await
                }
                SimulatedEvent::CancelOrders((cancel_requests, response_tx)) => {
                    self.cancel_orders(cancel_requests, response_tx).await
                }
                SimulatedEvent::CancelOrdersAll(response_tx) => {
                    self.cancel_orders_all(response_tx).await
                }
            }
        }
    }

    pub async fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>) {
        tokio::time::sleep(self.latency).await;

        let orders_open = self.markets
            .values()
            .map(|market| [&market.bids, &market.asks])
            .flatten()
            .flatten()
            .cloned()
            .map(Ok)
            .collect::<Result<Vec<Order<Open>>, ExecutionError>>();

        response_tx
            .send(orders_open)
            .expect("SimulatedExchange failed to send oneshot fetch_orders_open response")
    }

    pub async fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<SymbolBalance>, ExecutionError>>) {
        tokio::time::sleep(self.latency).await;

        let balances = self.balances
            .clone()
            .into_iter()
            .map(|(symbol, balance) | Ok(SymbolBalance::new(symbol, balance)))
            .collect::<Result<Vec<SymbolBalance>, ExecutionError>>();

        response_tx
            .send(balances)
            .expect("SimulatedExchange failed to send oneshot fetch_balances response")
    }

    pub async fn open_orders(&mut self, open_requests: Vec<Order<RequestOpen>>, response_tx: oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>) {
        tokio::time::sleep(self.latency).await;

        let opened_orders = open_requests
            .into_iter()
            .map(|request| {
                // Try to find the SimulatedMarket for the RequestOpen Instrument
                let orders = self
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
                match open.state.side {
                    Side::Buy => {
                        orders.bids.push(open.clone());
                        orders.bids.sort();
                    },
                    Side::Sell => {
                        orders.asks.push(open.clone());
                        orders.bids.sort();
                    }
                }

                Ok(open)
            })
            .collect::<Vec<Result<Order<Open>, ExecutionError>>>();

        response_tx
            .send(opened_orders)
            .expect("SimulatedExchange failed to send oneshot open_orders response")

    }

    pub async fn cancel_orders(&mut self, cancel_requests: Vec<Order<RequestCancel>>, response_tx: oneshot::Sender<Vec<Result<OrderId, ExecutionError>>>) {
        tokio::time::sleep(self.latency).await;

        let cancelled_orders = cancel_requests
            .into_iter()
            .map(|request| {
                // Try to find the SimulatedMarket for the RequestCancel Instrument
                let orders = self
                    .markets
                    .get_mut(&request.instrument)
                    .ok_or_else(|| ExecutionError::Simulated(format!(
                        "received Order<RequestCancel> for unknown Instrument: {}", request.instrument
                    )))?;

                // Try to find an Order<Open> to remove for the RequestCancel ClientOrderId
                let bid_indexes = Self::find_order_indexes(&orders.bids, &request.cid);
                let ask_indexes = Self::find_order_indexes(&orders.asks, &request.cid);

                match (bid_indexes.len(), ask_indexes.len()) {
                    (0, 0) => {
                        Err(ExecutionError::Simulated(format!(
                            "received Order<RequestCancel> for unknown ClientOrderId: {:?}", request.cid,
                        )))
                    },
                    (1, 0) => {
                        Ok(orders.bids.remove(bid_indexes[0]).state.id) // Todo: Should we return CID?
                    }
                    (0, 1) => {
                        Ok(orders.asks.remove(ask_indexes[0]).state.id) // Todo: Should we return CID?
                    }
                    _ => {
                        panic!("RequestCancel ClientOrderId matches more than one Order<Open>")
                    }
                }
            })
            .collect::<Vec<Result<OrderId, ExecutionError>>>();

        response_tx
            .send(cancelled_orders)
            .expect("SimulatedExchange failed to send oneshot cancel_orders response")
    }

    pub fn find_order_indexes(orders: &Vec<Order<Open>>, cid: &ClientOrderId) -> Vec<usize> {
        orders
            .iter()
            .enumerate()
            .fold(vec![], |mut open_indexes, (current_index, order)| {
                if order.cid.cmp(cid).is_eq() {
                    open_indexes.push(current_index);
                }
                open_indexes
            })
    }

    pub async fn cancel_orders_all(&mut self, response_tx: oneshot::Sender<Result<(), ExecutionError>>) {
        tokio::time::sleep(self.latency).await;

        self.markets
            .values_mut()
            .map(|market| {
                market.bids.drain(..);
                market.asks.drain(..);
            });

        response_tx
            .send(Ok(()))
            .expect("SimulatedExchange failed to send oneshot cancel_orders_all response")
    }
}
