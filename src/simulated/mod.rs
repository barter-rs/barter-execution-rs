mod account;

use self::account::ClientAccount;
use crate::{
    AccountEvent, Open, Order, RequestOpen, RequestCancel, ExecutionError, SymbolBalance, OrderId,
    model::ClientOrderId,
};
use barter_integration::model::{Instrument, Side};
use barter_data::model::PublicTrade;
use std::{
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

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
    pub latency: Duration,
    pub event_account_tx: mpsc::UnboundedSender<AccountEvent>,
    pub event_simulated_rx: mpsc::UnboundedReceiver<SimulatedEvent>,
    pub account: ClientAccount,
}

// Todo: Make matches and therefore trade notifications, then make balance updates after? ie/ return Vec<Trade>
// Todo: Add notifications for any relevant operations, eg/ balance updates upon opens?
// Todo: Make it work with Candles, too. Could use eg/ trait { price() -> f64, volume() -> f64 }
// Todo: Check bids and asks are sorted correct, ie/ asks are reverse sorted
// Todo: is_full_fill may live somewhere else along with enum eg/ Orders
// Todo: Ensure in barter-rs core we are applying AccountEvent::Balances correctly
//   '--> only apply what's in array, rather than upserting the whole array
// Todo: Identify what part of the code sends AccountEvents, perhaps event_tx can be owned by it to further split this file
// Todo: Account get & get_mut panic is not consistent with SimulatedExchange self.account.orders.get_checked w/ error handling
//   '--> I think it's okay since Exchange wants to blow up if the data feed is not configured correctly, but client shouldn't able to blow up exchange

impl SimulatedExchange {
    pub async fn run(mut self) {
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                SimulatedEvent::MarketTrade((instrument, trade)) => {
                    self.account.match_orders(instrument, trade)
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

        let orders_open = self
            .account
            .orders
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

        let balances = self
            .account
            .balances
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
                    .account
                    .orders
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
                    .account
                    .orders
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

        for instrument_orders in self.account.orders.values_mut() {
            instrument_orders.bids.drain(..);
            instrument_orders.asks.drain(..);
        }

        response_tx
            .send(Ok(()))
            .expect("SimulatedExchange failed to send oneshot cancel_orders_all response")
    }
}



#[cfg(test)]
mod tests {
    use barter_integration::model::{Exchange, InstrumentKind};
    use barter_data::builder::Streams;
    use barter_data::ExchangeId;
    use barter_data::model::{DataKind, MarketEvent};
    use barter_data::model::subscription::SubKind;
    use super::*;

    // #[tokio::test]
    // async fn it_works() {
    //     let exchange_id = ExchangeId::BinanceFuturesUsd;
    //     let instrument = Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual));
    //
    //     let trades = Streams::builder()
    //         .subscribe([(exchange_id, instrument.clone(), SubKind::Trade)])
    //         .init()
    //         .await
    //         .expect("failed to initialise Trade stream");
    //
    //     let (event_account_tx, event_account_rx) = mpsc::unbounded_channel();
    //     let (event_simulated_tx, event_simulated_rx) = mpsc::unbounded_channel();
    //
    //     let exchange = SimulatedExchange {
    //         fees_percent: 0.0,
    //         latency: Default::default(),
    //         event_account_tx,
    //         event_simulated_rx,
    //         balances: Default::default(),
    //         markets: Default::default(),
    //         trade_number: 0
    //     };
    //
    //     exchange.run().await;
    // }

    // fn simulated_exchange() -> SimulatedExchange {
    //     let (event_account_tx, _) = mpsc::unbounded_channel();
    //     let (_, event_simulated_rx) = mpsc::unbounded_channel();
    //
    //     SimulatedExchange {
    //         fees_percent: 0.0,
    //         latency: Default::default(),
    //         event_account_tx,
    //         event_simulated_rx,
    //         balances: Default::default(),
    //         markets: Default::default(),
    //         trade_number: 0
    //     }
    // }
    //
    // fn balances(base: Balance, quote: Balance) -> HashMap<Symbol, Balance> {
    //     HashMap::from_iter([
    //         (Symbol::from("base"), base),
    //         (Symbol::from("quote"), quote),
    //     ])
    // }
    //
    // fn markets(bids: Vec<Order<Open>>, asks: Vec<Order<Open>>) -> HashMap<Instrument, ClientOrders> {
    //     HashMap::from([(
    //         Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual)),
    //         client_orders(bids, asks)
    //     )])
    // }
    //
    // fn client_orders(bids: Vec<Order<Open>>, asks: Vec<Order<Open>>) -> ClientOrders {
    //     ClientOrders { bids, asks, }
    // }
    //
    // fn open_order(side: Side, price: f64, quantity: f64, filled_quantity: f64) -> Order<Open> {
    //     Order {
    //         exchange: Exchange::from("exchange"),
    //         instrument: Instrument::from(("base", "quote", InstrumentKind::Spot)),
    //         cid: ClientOrderId(Uuid::new_v4()),
    //         state: Open {
    //             id: OrderId::from("id"),
    //             side,
    //             price,
    //             quantity,
    //             filled_quantity
    //         }
    //     }
    // }
    //
    // fn market_trade(side: Side, price: f64, quantity: f64) -> MarketEvent {
    //     MarketEvent {
    //         exchange_time: Default::default(),
    //         received_time: Default::default(),
    //         exchange: Exchange::from("exchange"),
    //         instrument: Instrument::from(("base", "quote", InstrumentKind::Spot)),
    //         kind: DataKind::Trade(PublicTrade {
    //             id: "id".to_string(),
    //             price,
    //             quantity,
    //             side
    //         })
    //     }
    // }
    //
    // fn trade(side: Side, price: f64, quantity: f64) -> PublicTrade {
    //     PublicTrade {
    //         id: "id".to_string(),
    //         price,
    //         quantity,
    //         side
    //     }
    // }
    //
    // #[test]
    // fn test_client_orders_has_matching_bid() {
    //     struct TestCase {
    //         client_orders: ClientOrders,
    //         input_trade: PublicTrade,
    //         expected: bool,
    //     }
    //
    //     let tests = vec![
    //         TestCase { // TC0: No matching bids for trade since no bids
    //             client_orders: client_orders(vec![], vec![]),
    //             input_trade: trade(Side::Buy, 100.0, 1.0),
    //             expected: false,
    //         },
    //         TestCase { // TC1: No matching bids for trade
    //             client_orders: client_orders(
    //                 vec![open_order(Side::Buy, 50.0, 1.0, 0.0)],
    //                 vec![]
    //             ),
    //             input_trade: trade(Side::Buy, 100.0, 1.0),
    //             expected: false,
    //         },
    //         TestCase { // TC2: Exact matching bid for trade
    //             client_orders: client_orders(
    //                 vec![open_order(Side::Buy, 100.0, 1.0, 0.0)],
    //                 vec![]
    //             ),
    //             input_trade: trade(Side::Buy, 100.0, 1.0),
    //             expected: true,
    //         },
    //         TestCase { // TC3: Matching bid for trade
    //             client_orders: client_orders(
    //                 vec![open_order(Side::Buy, 150.0, 1.0, 0.0)],
    //                 vec![]
    //             ),
    //             input_trade: trade(Side::Buy, 100.0, 1.0),
    //             expected: true,
    //         },
    //         TestCase { // TC4: No matching bid for trade even though there is an intersecting ask
    //             client_orders: client_orders(
    //                 vec![],
    //                 vec![open_order(Side::Sell, 150.0, 1.0, 0.0)]
    //             ),
    //             input_trade: trade(Side::Buy, 100.0, 1.0),
    //             expected: false,
    //         },
    //     ];
    //
    //     for (index, test) in tests.into_iter().enumerate() {
    //         let actual = test.client_orders.has_matching_bid(&test.input_trade);
    //         assert_eq!(actual, test.expected, "TC{} failed", index);
    //     }
    // }
    //
    // #[test]
    // fn test_client_orders_has_matching_ask() {
    //     struct TestCase {
    //         client_orders: ClientOrders,
    //         input_trade: PublicTrade,
    //         expected: bool,
    //     }
    //
    //     let tests = vec![
    //         TestCase { // TC0: No matching ask for trade since no asks
    //             client_orders: client_orders(vec![], vec![]),
    //             input_trade: trade(Side::Sell, 100.0, 1.0),
    //             expected: false,
    //         },
    //         TestCase { // TC1: No matching ask for trade
    //             client_orders: client_orders(
    //                 vec![],
    //                 vec![open_order(Side::Sell, 150.0, 1.0, 0.0)],
    //             ),
    //             input_trade: trade(Side::Sell, 100.0, 1.0),
    //             expected: false,
    //         },
    //         TestCase { // TC2: Exact matching ask for trade
    //             client_orders: client_orders(
    //                 vec![],
    //                 vec![open_order(Side::Sell, 100.0, 1.0, 0.0)],
    //             ),
    //             input_trade: trade(Side::Sell, 100.0, 1.0),
    //             expected: true,
    //         },
    //         TestCase { // TC3: Matching ask for trade
    //             client_orders: client_orders(
    //                 vec![],
    //                 vec![open_order(Side::Sell, 150.0, 1.0, 0.0)],
    //             ),
    //             input_trade: trade(Side::Sell, 200.0, 1.0),
    //             expected: true,
    //         },
    //         TestCase { // TC4: No matching ask for trade even though there is an intersecting bid
    //             client_orders: client_orders(
    //                 vec![open_order(Side::Sell, 150.0, 1.0, 0.0)],
    //                 vec![],
    //             ),
    //             input_trade: trade(Side::Buy, 200.0, 1.0),
    //             expected: false,
    //         },
    //     ];
    //
    //     for (index, test) in tests.into_iter().enumerate() {
    //         let actual = test.client_orders.has_matching_ask(&test.input_trade);
    //         assert_eq!(actual, test.expected, "TC{} failed", index);
    //     }
    // }

    // #[test]
    // fn test_has_matching_bid() {
    //     let mut exchange = simulated_exchange();
    //     let instrument = Instrument::from(("base", "quote", InstrumentKind::Spot));
    //
    //     struct TestCase {
    //         input_bids: Vec<Order<Open>>,
    //         input_instrument: Instrument,
    //         input_trade: PublicTrade,
    //     }
    //
    //     let tests = vec![
    //         TestCase { // TC0: No matching bids for trade since no bids
    //             input_bids: vec![],
    //             input_instrument: instrument.clone(),
    //             input_trade: trade(Side::Buy, 100.0, 1.0)
    //         },
    //         TestCase { // TC1: No matching bids for trade
    //             input_bids: vec![open_order(Side::Buy, 50.0, 1.0, 0.0)],
    //             input_instrument: instrument.clone(),
    //             input_trade: trade(Side::Buy, 100.0, 1.0)
    //         },
    //         TestCase { // TC2: Exact matching bid for trade
    //             input_bids: vec![open_order(Side::Buy, 100.0, 1.0, 0.0)],
    //             input_instrument: instrument.clone(),
    //             input_trade: trade(Side::Buy, 100.0, 1.0)
    //         },
    //         TestCase { // TC3: Matching bid for trade
    //             input_bids: vec![open_order(Side::Buy, 150.0, 1.0, 0.0)],
    //             input_instrument: instrument.clone(),
    //             input_trade: trade(Side::Buy, 100.0, 1.0)
    //         },
    //     ];
    //
    //     for (index, test) in cases.into_iter().enumerate() {
    //         exchange.markets.get_mut()
    //
    //         match (actual, test.expected) {
    //             (Ok(actual), Ok(expected)) => {
    //                 assert_eq!(actual, expected, "TC{} failed", index)
    //             }
    //             (Err(_), Err(_)) => {
    //                 // Test passed
    //             }
    //             (actual, expected) => {
    //                 // Test failed
    //                 panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
    //             }
    //         }
    //     }
    // }



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