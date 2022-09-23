use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side, Symbol};
use futures::StreamExt;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use uuid::Uuid;
use barter_data::builder::Streams;
use barter_data::ExchangeId;
use barter_data::model::{DataKind, MarketEvent, PublicTrade};
use barter_data::model::subscription::SubKind;
use crate::model::ClientOrderId;
use crate::{AccountEvent, ExecutionError, Open, Order, OrderId};
use crate::model::balance::Balance;

// #[tokio::test]
// async fn it_works() {
//     let exchange_id = ExchangeId::BinanceFuturesUsd;
//     let instrument = Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual));
//
//     let order_books = Streams::builder()
//         .subscribe([
//             (exchange_id, instrument.clone(), SubKind::Trade)
//         ])
//         .init()
//         .await
//         .expect("failed to initialise OrderBook stream");
//
//     let mut order_books = order_books.join::<MarketEvent>().await;
//
//     let order = Order {
//         exchange: Exchange::from(exchange_id),
//         instrument,
//         cid: ClientOrderId(Uuid::new_v4()),
//         state: Open {
//             id: OrderId::from("order_id"),
//             side: Side::Buy,
//             price: 18800.0,
//             quantity: 1.0,
//             filled_quantity: 0.0
//         }
//     };
//
//     while let Some(event) = order_books.recv().await {
//         match event.kind {
//             DataKind::Trade(trade) => println!("{trade:?}"),
//             DataKind::Candle(_) => {}
//             DataKind::OrderBook(_) => {}
//         }
//     }
//
// }


pub struct SimulatedExchange {
    pub fees_percent: f64,
    pub latency: Duration,
    pub event_tx: mpsc::UnboundedSender<AccountEvent>,
    pub trade_rx: mpsc::UnboundedReceiver<MarketEvent>,
    pub balances: HashMap<Symbol, RwLock<Balance>>,
    pub markets: HashMap<Instrument, RwLock<ClientOrders>>,
}

pub struct ClientOrders {
    pub bids: BinaryHeap<Order<Open>>,
    pub asks: BinaryHeap<Order<Open>>,
}

impl SimulatedExchange {
    // Todo: balance would change when:
    // 1. open an order (available)
    // 2. make a trade (total and/or available)
    pub async fn run(mut self: Arc<Self>) {
        // while let Some(market) = self.trade_rx.recv().await {
        //     let client_orders = self
        //         .markets
        //         .get_mut(&market.instrument)
        //         .expect("received MarketEvent for unrecognised Instrument");
            // Check if we have a match
            // Simulate match
            // Send trade
        // }
    }

}


#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use chrono::Utc;
    use tokio::sync::mpsc;
    use barter_data::model::{Level, OrderBook, PublicTrade};
    use barter_data::test_util::market_trade;
    // use crate::simulated::SimulatedMarket;
    // use crate::test_util::order_open;
    // use super::*;
    //
    // PublicTrade { id: "1459757567", price: 18990.7, quantity: 0.012, side: Sell }
    // PublicTrade { id: "1459757568", price: 18990.8, quantity: 0.034, side: Buy }
    // PublicTrade { id: "1459757569", price: 18990.7, quantity: 0.001, side: Sell }
    // PublicTrade { id: "1459757570", price: 18990.8, quantity: 0.263, side: Buy }
    //
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











































