use crate::{
    AccountEvent, Open, Order, RequestOpen, RequestCancel, ExecutionError, SymbolBalance, OrderId,
    model::{ClientOrderId, AccountEventKind, trade::{Trade, TradeId, SymbolFees}, balance::Balance},
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
use num_traits::identities::Zero;

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

// #[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
// pub struct ClientAccount {
//     balances: HashMap<Symbol, Balance>,
//     orders: HashMap<Instrument, ClientOrders>,
// }

#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
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
        // let mut client_orders = self.client_orders(&instrument);

        if self.has_matching_bid(&instrument, &trade) {
            self.match_bids(&instrument, &trade);
        }

        // if Self::has_matching_ask(client_orders, &trade) {
        //     Do matches
        // }
    }

    fn has_matching_bid(&self, instrument: &Instrument, trade: &PublicTrade) -> bool {
        match self.client_orders(instrument).bids.last() {
            Some(best_bid) if best_bid.state.price >= trade.price => true,
            _ => false,
        }
    }

    fn client_orders(&self, instrument: &Instrument) -> &ClientOrders {
        self.markets
            .get(instrument)
            .expect("received MarketEvent for unrecognised Instrument")
    }

    fn client_orders_mut(&mut self, instrument: &Instrument) -> &mut ClientOrders {
        self.markets
            .get_mut(instrument)
            .expect("received MarketEvent for unrecognised Instrument")
    }

    fn match_bids(&mut self, instrument: &Instrument, trade: &PublicTrade) {
        // Keep track of how much trade liquidity is remaining to match with
        let mut remaining_liquidity = trade.quantity;

        let best_bid = loop {
            // Pop the best bid Order<Open>
            let best_bid = match self.client_orders_mut(instrument).bids.pop() {
                Some(best_bid) => best_bid,
                None => break None,
            };

            // If current best bid is no longer a match, or the trade liquidity is exhausted
            if best_bid.state.price < trade.price || remaining_liquidity <= 0.0 {
                break Some(best_bid);
            }

            // Liquidity is either enough for a full-fill or partial-fill
            match Self::fill_kind(&best_bid, remaining_liquidity) {
                // Full Order<Open> fill
                OrderFill::Full => {
                    // Remove trade quantity from remaining liquidity
                    let trade_quantity = best_bid.state.remaining_quantity();
                    remaining_liquidity -= trade_quantity;

                    // Update balances & send AccountEvents to client
                    self.update_client_account_from_match(best_bid, trade_quantity);

                    // Exact full fill with zero remaining trade liquidity (highly unlikely)
                    if remaining_liquidity.is_zero() {
                        break None
                    }
                }

                // Partial Order<Open> fill with zero remaining trade liquidity
                OrderFill::Partial => {
                    // Partial-fill means trade quantity is all the remaining trade liquidity
                    let trade_quantity = remaining_liquidity;

                    // Update balances & send AccountEvents to client
                    self.update_client_account_from_match(best_bid.clone(), trade_quantity);

                    break Some(best_bid)
                }
            }
        };

        // If best bid had a partial fill or is no longer a match, push it back onto the end of bids
        if let Some(best_bid) = best_bid {
            self.client_orders_mut(instrument).bids.push(best_bid)
        }
    }

    fn fill_kind(order: &Order<Open>, liquidity: f64) -> OrderFill {
        match order.state.remaining_quantity() <= liquidity {
            true => OrderFill::Full,
            false => OrderFill::Partial
        }
    }

    pub fn update_client_account_from_match(&mut self, order: Order<Open>, trade_quantity: f64) {
        // Calculate the trade fees (denominated in base or quote depending on Order Side)
        let fees = self.calculate_fees(&order, trade_quantity);

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
                fees
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

    pub fn calculate_fees(&self, order: &Order<Open>, trade_quantity: f64) -> SymbolFees {
        match order.state.side {
            Side::Buy => {
                SymbolFees::new(
                    order.instrument.base.clone(),
                    self.fees_percent * trade_quantity
                )
            }
            Side::Sell => {
                SymbolFees::new(
                    order.instrument.quote.clone(),
                    self.fees_percent * order.state.price * trade_quantity
                )
            }
        }
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
        // '--> Fees are taken into account in the opened order, so new balance update is true
        // 3. btc { total: 0, available: 0 }, usdt { total: 100, available: 0 }
        // 4a. Partial Fill for 0.5 btc for 50 usdt
        // '--> fees taken out of bought 0.5 btc, eg/ fee_percent * 0.5
        // 5a. btc { total: 0.5, available: 0.5 }, usdt { total: 50, available: 0 }
        // '--> btc total & available = fee_percent * 0.5
        // 4b. Full Fill for 1 btc for 100 usdt
        // 5b. btc { total: 1.0, available: 1.0 }, usdt { total: 0.0, available: 0.0 }

        // Todo: Turn this into template for Side::Sell balance update test case
        // 1. btc { total: 1.0, available: 1.0 }, usdt { total: 0.0, available: 0.0 }
        // 2. Open order ot sell 1 btc for 100 usdt
        // 3. btc { total: 1.0, available: 0.0 }, usdt { total: 0.0, available: 0.0 }
        // 4a. Partial fill for 0.5 btc for 50 usdt
        // 5a. btc { total: 0.5, available: 0.0 }, usdt { total: 50.0, available: 50.0 }
        // '--> usdt total & available = (trade_quantity * price) - fees

        let Instrument { base, quote, ..} = &order.instrument;

        let mut new_base_balance = *self
            .balances
            .get(base)
            .unwrap_or_else(|| panic!("Cannot update Balance for non-configured base Symbol: {}", base));

        let mut new_quote_balance = *self
            .balances
            .get(quote)
            .unwrap_or_else(|| panic!("Cannot update Balance for non-configured quote Symbol: {}", quote));

        match order.state.side {
            Side::Buy => {
                // Base total & available increase by trade_quantity minus base fees
                let base_increase = trade_quantity - fees.fees;
                new_base_balance.total += base_increase;
                new_base_balance.available += base_increase;

                // Quote total decreases by (trade_quantity * price)
                // Note: available was already decreased by the opening of the Side::Buy order
                new_quote_balance.total -= trade_quantity * order.state.price;
            }

            Side::Sell => {
                // Base total decreases by trade_quantity
                // Note: available was already decreased by the opening of the Side::Sell order
                new_base_balance.total -= trade_quantity;

                // Quote total & available increase by (trade_quantity * price) minus quote fees
                let quote_increase = (trade_quantity * order.state.price) - fees.fees;
                new_quote_balance.total += quote_increase;
                new_quote_balance.available += quote_increase
            }
        }

        *self.balances
            .get_mut(base)
            .expect(&format!("Cannot update Balance for non-configured base Symbol: {}", base))
            = new_base_balance;

        *self.balances
            .get_mut(quote)
            .expect(&format!("Cannot update Balance for non-configured quote Symbol: {}", base))
            = new_quote_balance;

        AccountEvent {
            received_time: Utc::now(),
            exchange: order.exchange.clone(),
            kind: AccountEventKind::Balances(vec![
                SymbolBalance::new(order.instrument.base.clone(), new_base_balance),
                SymbolBalance::new(order.instrument.quote.clone(), new_quote_balance),
            ])
        }
    }

    fn trade_id(&mut self) -> TradeId {
        self.trade_number += 1;
        TradeId(self.trade_number.to_string())
    }

    fn has_matching_ask(client_orders: &ClientOrders, trade: &PublicTrade) -> bool {
        match client_orders.asks.last() {
            Some(best_ask) if best_ask.state.price <= trade.price => true,
            _ => false,
        }
    }

    pub async fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>) {
        tokio::time::sleep(self.latency).await;

        let orders_open = self.markets
            .values()
            .flat_map(|market| [&market.bids, &market.asks])
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

    pub fn find_order_indexes(orders: &[Order<Open>], cid: &ClientOrderId) -> Vec<usize> {
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

        for market in self.markets.values_mut() {
            market.bids.drain(..);
            market.asks.drain(..);
        }

        response_tx
            .send(Ok(()))
            .expect("SimulatedExchange failed to send oneshot cancel_orders_all response")
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum OrderFill {
    Full,
    Partial,
}

#[cfg(test)]
mod tests {
    use barter_integration::model::InstrumentKind;
    use barter_data::builder::Streams;
    use barter_data::ExchangeId;
    use barter_data::model::subscription::SubKind;
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let exchange_id = ExchangeId::BinanceFuturesUsd;
        let instrument = Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual));

        let trades = Streams::builder()
            .subscribe([(exchange_id, instrument.clone(), SubKind::Trade)])
            .init()
            .await
            .expect("failed to initialise Trade stream");

        let (event_account_tx, event_account_rx) = mpsc::unbounded_channel();
        let (event_simulated_tx, event_simulated_rx) = mpsc::unbounded_channel();

        let exchange = SimulatedExchange {
            fees_percent: 0.0,
            latency: Default::default(),
            event_account_tx,
            event_simulated_rx,
            balances: Default::default(),
            markets: Default::default(),
            trade_number: 0
        };

        exchange.run().await;
    }

    // fn markets(bids: Vec<Order<Open>>, asks: Vec<Order<Open>>) -> HashMap<Instrument, ClientOrders> {
    //     HashMap::from([(
    //         Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual)),
    //         client_orders(bids, asks)
    //     )])
    // }
    //
    // fn client_orders(bids: Vec<Order<Open>>, asks: Vec<Order<Open>>) -> ClientOrders {
    //     ClientOrders {
    //         bids: BinaryHeap::from_iter(bids),
    //         asks: BinaryHeap::from_iter(asks),
    //     }
    // }
    //
    // fn trade(side: Side, price: f64, quantity: f64) -> MarketEvent {
    //     MarketEvent {
    //         exchange_time: Default::default(),
    //         received_time: Default::default(),
    //         exchange: Exchange::from("exchange"),
    //         instrument: Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual)),
    //         kind: DataKind::Trade(PublicTrade {
    //             id: "id".to_string(),
    //             price,
    //             quantity,
    //             side
    //         })
    //     }
    // }
    //
    // fn balances(btc: Balance, usdt: Balance) -> HashMap<Symbol, Balance> {
    //     HashMap::from_iter([
    //         (Symbol::from("btc"), btc),
    //         (Symbol::from("usdt"), usdt),
    //     ])
    // }
    //
    // #[test]
    // fn test_match_buy_order_with_input_trades() {
    //     // Open bid to Buy 1.0 btc for 1000 usdt
    //     // Open asks to Sell 1.0 btc for 2000 usdt
    //     let starting_orders = markets(
    //         vec![order_open(Side::Buy, 1000.0, 1.0, 0.0)],
    //         vec![order_open(Side::Sell, 2000.0, 1.0, 0.0)]
    //     );
    //
    //     // Available balance reflects the initial open orders
    //     let starting_balances = balances(
    //         Balance::new(10.0, 9.0),
    //         Balance::new(10000.0, 9000.0)
    //     );
    //
    //     let (trade_tx, trade_rx) = mpsc::unbounded_channel();
    //     let (event_tx, event_rx) = mpsc::unbounded_channel();
    //
    //     let simulated = SimulatedExchange {
    //         balances: starting_balances,
    //         markets: starting_orders,
    //         trade_rx,
    //         event_tx,
    //     };
    //
    //     struct TestCase {
    //         input_trade: MarketEvent,
    //         expected_orders: ClientOrders,
    //         expected_balances: HashMap<Symbol, Balance>,
    //     }
    //
    //     // Todo: What happens if we have a buy and sell order at same price?
    //
    //     let cases = vec![
    //         TestCase {
    //             // TC0: Trade does not match our open ask of (1.0 for 2000.0), so do nothing
    //             input_trade: trade(Side::Buy, 1500.0, 1.0),
    //             expected_orders: client_orders(
    //                 vec![order_open(Side::Buy, 1000.0, 1.0, 0.0)],
    //                 vec![order_open(Side::Sell, 2000.0, 1.0, 0.0)]
    //             ),
    //             expected_balances: balances(
    //                 Balance::new(10.0, 9.0),
    //                 Balance::new(10000.0, 9000.0)
    //             )
    //         },
    //         TestCase {
    //             // TC1: Trade matches 0.5 of our open ask of (1.0 for 2000.0), so Sell 0.5 btc
    //             input_trade: trade(Side::Buy, 2000.0, 0.5),
    //             expected_orders: client_orders(
    //                 vec![order_open(Side::Buy, 1000.0, 1.0, 0.0)],
    //                 vec![order_open(Side::Sell, 2000.0, 0.5, 0.5)]
    //             ),
    //             expected_balances: balances(
    //                 Balance::new(9.5, 9.0),
    //                 Balance::new(11000.0, 10000.0)
    //             )
    //         },
    //         TestCase {
    //             // TC2: Trade matches 0.5 of our open bid of (1000.0 for 1.0), so Buy 0.5 btc
    //             input_trade: trade(Side::Sell, 1000.0, 0.5),
    //             expected_orders: client_orders(
    //                 vec![order_open(Side::Buy, 1000.0, 0.5, 0.5)],
    //                 vec![order_open(Side::Sell, 2000.0, 0.5, 0.5)]
    //             ),
    //             expected_balances: balances(
    //                 Balance::new(10.0, 9.5),
    //                 Balance::new(10500.0, 10000.0)
    //             )
    //         },
    //         // Todo: More cases please
    //     ];
    //
    // }
}