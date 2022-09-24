use crate::{
    AccountEvent, Open, Order, RequestOpen, RequestCancel, ExecutionError, SymbolBalance, OrderId,
    model::{ClientOrderId, AccountEventKind, trade::{Trade, TradeId}, balance::Balance},
};
use barter_integration::model::{Instrument, Side, Symbol};
use barter_data::model::PublicTrade;
use std::{
    collections::HashMap,
    time::Duration,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;
use crate::model::trade::SymbolFees;

pub mod matches;
pub mod engine;

/// Todo:
#[derive(Debug)]
pub enum SimulatedEvent {
    MarketTrade((Instrument, PublicTrade)),
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(oneshot::Sender<Result<Vec<SymbolBalance>, ExecutionError>>),
    OpenOrders((Vec<Order<RequestOpen>>, oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>)),
    CancelOrders((Vec<Order<RequestCancel>>, oneshot::Sender<Vec<Result<OrderId, ExecutionError>>>)),
    CancelOrdersAll(oneshot::Sender<Result<(), ExecutionError>>)
}

/// Todo:
#[derive(Debug)]
pub struct SimulatedExchange {
    pub fees_percent: f64,
    pub latency: Duration,
    pub event_account_tx: mpsc::UnboundedSender<AccountEvent>,
    pub event_simulated_rx: mpsc::UnboundedReceiver<SimulatedEvent>,
    pub balances: HashMap<Symbol, Balance>,
    pub markets: HashMap<Instrument, ClientOrders>,
    pub trade_number: u64,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ClientAccount {
    balances: HashMap<Symbol, Balance>,
    orders: HashMap<Instrument, ClientOrders>,
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
// Todo: Make it work with Candles, too. Could use eg/ trait { price() -> f64, volume() -> f64 }
// Todo: Check for overlapping bids / asks, one trade could provide liquidity for both sides atm
//     '--> One Vec for all bids?
// Todo: Check bids and asks are sorted correct, ie/ asks are reverse sorted
// Todo: is_full_fill may live somewhere else along with enum eg/ Orders
// Todo: Ensure in barter-rs core we are applying AccountEvent::Balances correctly (ie/ only apply what's in array)
// Todo: Ensure balances are updated to include fees, I believe the order size should be altered based on fees?

impl SimulatedExchange {
    pub async fn run(mut self) {
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                SimulatedEvent::MarketTrade((instrument, trade)) => {
                    self.match_orders(instrument, trade)
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

    pub fn match_orders(&mut self, instrument: Instrument, trade: PublicTrade) {
        // Get ClientOrders associated with input PublicTrade
        let mut client_orders = self.client_orders(&instrument);

        if Self::has_matching_bid(client_orders, &trade) {
            self.match_bids(&mut client_orders, &trade);
        }

        if Self::has_matching_ask(client_orders, &trade) {
            // Do matches
        }
    }

    fn has_matching_bid(client_orders: &ClientOrders, trade: &PublicTrade) -> bool {
        match client_orders.bids.last() {
            Some(best_bid) if best_bid.state.price >= trade.price => true,
            _ => false,
        }
    }

    fn match_bids(&mut self, client_orders: &mut ClientOrders, trade: &PublicTrade) {
        // Keep track of how much trade liquidity is remaining to match with
        let mut remaining_liquidity = trade.quantity;

        let best_bid = while let Some(mut best_bid) = client_orders.bids.pop() {

            // If there is no longer a match, or the trade liquidity is exhausted
            if best_bid.state.price < trade.price || remaining_liquidity <= 0.0 {
                break Some(best_bid);
            }

            // Liquidity is either enough for a full-fill or partial fill
            match Self::fill_kind(&best_bid, remaining_liquidity) {
                // Exact full Order<Open> fill with zero remaining trade liquidity
                OrderFill::Full if remaining_liquidity.is_zero() => {
                    break None
                }

                // Full Order<Open> fill with remaining trade liquidity,
                OrderFill::Full => {
                    // Remove trade quantity from remaining liquidity
                    let trade_quantity = best_bid.state.remaining_quantity();
                    remaining_liquidity -= trade_quantity;

                    // Update balances & send AccountEvents to Client
                    self.update_client_account_from_match(best_bid, trade_quantity, );

                    if remaining_liquidity.is_zero() {
                        break None
                    } else {
                        continue
                    }
                }

                // Partial Order<Open> fill with zero remaining trade liquidity
                OrderFill::Partial => {
                    break Some(best_bid)
                }
            }
        };

        // Push the remaining best bid back into the end of the open bids
        if let Some(best_bid) = best_bid {
            client_orders.bids.push(best_bid)
        }
    }

    fn fill_kind(order: &Order<Open>, liquidity: f64) -> OrderFill {
        match order.state.remaining_quantity() <= liquidity {
            true => OrderFill::Full,
            false => OrderFill::Partial
        }
    }

    pub fn update_client_account_from_match(&mut self, order: Order<Open>, trade_quantity: f64) {
        // Calculate the quote denominated trade fees
        let fees = SymbolFees {
            symbol: order.instrument.quote.clone(),
            fees: self.fees_percent * order.state.price * trade_quantity
        };

        // Generate AccountEvent::Balances by updating the base & quote SymbolBalances
        let balance_event = self.update_balance_from_match(&order, trade_quantity, &fees);

        // Generate AccountEvent::Trade for the Order<Open> match


        let trade_event = AccountEvent {
            received_time: Utc::now(),
            exchange: order.exchange,
            kind: AccountEventKind::Trade(Trade {
                id: self.trade_id(),
                order_id: order.state.id,
                instrument: order.instrument,
                side: order.state.side,
                price: order.state.price,
                quantity: trade_quantity,
                fees: trade_fees_quote_denominated
            })
        };

        // Send AccountEvents to client
        self.event_account_tx
            .send(trade_event)
            .expect("Client is offline - failed to send AccountEvent::Trade");
        self.event_account_tx
            .send(balance_event)
            .expect("Client is offline - failed to send AccountEvent::Balances");
    }

    /// [`Order<Open>`] matches (trades) result in the [`Balance`] of the base & quote
    /// [`Symbol`] to change. The direction of each balance change will depend on if the matched
    /// [`Order<Open`] was a [`Side::Buy`] or [`Side::Sell`].
    ///
    /// A [`Side::Buy`] match causes the [`Symbol`] [`Balance`] of the base to increase by the
    /// `trade_quantity`, and the quote to decrease by the `trade_quantity * price`.
    ///
    /// A [`Side::Sell`] match causes the [`Symbol`] [`Balance`] of the base to decrease by the
    /// `trade_quantity`, and the quote to increase by the `trade_quantity * price`.
    pub fn update_balance_from_match(&mut self, order: &Order<Open>, trade_quantity: f64, fees: &SymbolFees) -> AccountEvent {
        // Todo: Turn this into template for Side::Buy balance update test case
        // 1. btc { total: 0, available: 0 }, usdt { total: 100, available: 100 }
        // 2. Open order to buy 1 btc for 100 usdt
        // 3. btc { total: 0, available: 0 }, usdt { total: 100, available: 0 }
        // 4a. Partial Fill for 0.5 btc for 50 usdt
        // 5a. btc { total: 0.5, available: 0.5 }, usdt { total: 50, available: 0 }
        // 4b. Full Fill for 1 btc for 100 usdt
        // 5b. btc { total: 1.0, available: 1.0 }, usdt { total: 0.0, available: 0.0 }

        // Todo: Turn this into template for Side::Sell balance update test case
        // 1. btc { total: 1.0, available: 1.0 }, usdt { total: 0.0, available: 0.0 }
        // 2. Open order ot sell 1 btc for 100 usdt
        // 3. btc { total: 1.0, available: 0.0 }, usdt { total: 0.0, available: 0.0 }

        // 4a. Partial fill for 0.5 btc for 50 usdt
        // 5a. btc { total: 0.5, available: 0.0 }, usdt { total: 50.0, available: 50.0 }

        let Instrument { base, quote, ..} = &order.instrument;

        let base = self
            .balances
            .get_mut(base)
            .expect(&format!("Cannot update Balance for non-configured base Symbol: {}", base));

        let quote = self
            .balances
            .get_mut(quote)
            .map(|balance| balance.)
            .expect(&format!("Cannot update Balance for non-configured quote Symbol: {}", quote));



        match order.state.side {
            Side::Buy => {
                // Base total & available increase by trade_quantity
                base.total += trade_quantity;
                base.available += trade_quantity;

                // Quote total decreases by (trade_quantity * price)
                // Note: available was already decreased by the opening of the Side::Buy order
                quote.total -= trade_quantity * order.state.price;
            }

            Side::Sell => {
                // Base total decreases by trade_quantity
                // Note: available was already decreased by the opening of the Side::Sell order
                base.total -= trade_quantity;

                // Quote total & available increase by (trade_quantity * price)
                let delta_increase = trade_quantity * order.state.price;
                quote.total += delta_increase;
                quote.available += delta_increase
            }
        }

        AccountEvent {
            received_time: Utc::now(),
            exchange: order.exchange.clone(),
            kind: AccountEventKind::Balances(vec![
                SymbolBalance::new(order.instrument.base.clone(), base.copy()),
                SymbolBalance::new(order.instrument.quote.clone(), quote.copy()),
            ])
        }
    }

    fn trade_id(&mut self) -> TradeId {
        self.trade_number += 1;
        TradeId::from(self.trade_number)
    }

    fn has_matching_ask(client_orders: &ClientOrders, trade: &PublicTrade) -> bool {
        match client_orders.asks.last() {
            Some(best_ask) if best_ask.state.price <= trade.price => true,
            _ => false,
        }
    }

    fn client_orders(&self, instrument: &Instrument) -> &ClientOrders {
        self.markets
            .get(instrument)
            .expect("received MarketEvent for unrecognised Instrument")
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

pub enum OrderFill {
    Full,
    Partial,
}