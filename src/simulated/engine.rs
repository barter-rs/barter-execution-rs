use crate::{
    AccountEvent, Open, Order,
    model::balance::Balance,
};
use barter_integration::model::{Instrument, InstrumentKind, Symbol};
use barter_data::model::{DataKind, MarketEvent, PublicTrade};
use std::{
    time::Duration,
    collections::{BinaryHeap, HashMap},
    sync::Arc
};
use std::ops::RemAssign;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use barter_data::builder::Streams;
use barter_data::ExchangeId;
use barter_data::model::subscription::SubKind;

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Config {
    pub fees_percent: f64,
    pub latency: Duration,
    pub instruments: Vec<Instrument>,
    pub balances: HashMap<Symbol, Balance>,
}

#[derive(Debug)]
pub struct Exchange {
    pub fees_percent: f64,
    pub latency: Duration,
    pub event_tx: mpsc::UnboundedSender<AccountEvent>,
    pub trade_rx: RwLock<mpsc::UnboundedReceiver<MarketEvent>>,
    pub balances: HashMap<Symbol, RwLock<Balance>>,
    pub markets: HashMap<Instrument, RwLock<ClientOrders>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClientOrders {
    pub bids: BinaryHeap<Order<Open>>,
    pub asks: BinaryHeap<Order<Open>>,
}

// Todo: check which way round the bids and ask ordering is

impl Exchange {
    pub async fn run(self: Arc<Self>) {
        while let Some(market) = self.trade_rx.write().recv().await {
            // Get ClientOrders associated with input MarketEvent
            let client_orders = self.client_orders(&market.instrument);

            // Extract MarketEvent PublicTrade
            let trade = match market.kind {
                DataKind::Trade(trade) => trade,
                _ => continue
            };

            // Check if there is a matching client bid
            if Self::is_matching_bid(client_orders, &trade) {
                self.fill_bids(client_orders, &trade);
            }

            // Check if there is a matching client bid
            if Self::is_matching_ask(client_orders, &trade) {
                self.fill_bids(client_orders, &trade);
            }

            // Asks
            if let Some(order) = client_orders.asks.peek() {
                if order.state.price <= trade.price {

                    client_orders.asks.

                    // match
                }
            }


            match (client_orders.bids.peek(), client_orders.asks.peek()) {
                (Some(), _) => {}
            }

            println!("{market:?}");
        };
    }

    fn client_orders(&self, instrument: &Instrument) -> &RwLock<ClientOrders> {
        self.markets
            .get(instrument)
            .expect("received MarketEvent for unrecognised Instrument")
    }

    fn is_matching_bid(client_orders: &RwLock<ClientOrders>, trade: &PublicTrade) -> bool {
        match client_orders.read().bids.peek() {
            Some(order) if order.state.price >= trade.price => true,
            _ => false
        }
    }

    fn fill_bids(&self, client_orders: &RwLock<ClientOrders>, trade: &mut PublicTrade) {
        let bids = &mut client_orders.write().bids;

        let top_bid = bids.pop().unwrap();

        let order_quantity = top_bid.state.remaining_quantity();
        let trade_quantity = trade.quantity;

        if order_quantity > trade_quantity {

        }
    }

    fn is_matching_ask(client_orders: &RwLock<ClientOrders>, trade: &PublicTrade) -> bool {
        match client_orders.read().asks.peek() {
            Some(order) if order.state.price <= trade.price => true,
            _ => false
        }
    }
}

#[tokio::test]
async fn it_works() {
    let exchange_id = ExchangeId::BinanceFuturesUsd;
    let instrument = Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual));

    let trades = Streams::builder()
        .subscribe([(exchange_id, instrument.clone(), SubKind::Trade)])
        .init()
        .await
        .expect("failed to initialise Trade stream");

    let (event_tx, event_rx) = mpsc::unbounded_channel();

    let exchange = Arc::new(Exchange {
        fees_percent: 0.0,
        latency: Default::default(),
        event_tx,
        trade_rx: RwLock::new(trades.join::<MarketEvent>().await),
        balances: Default::default(),
        markets: Default::default()
    });

    exchange.run().await;
}