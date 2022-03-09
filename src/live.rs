use crate::socket::{
    ExchangeSocket, SocketError, Transformer,
    protocol::websocket::{WebSocketParser, WsStream}
};
use barter_data::client::binance::BinanceMessage;
use barter_data::model::{MarketData, Trade};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::info;

/// Eg/ Get data from the exchange like barter-data-rs

pub struct BinanceRequest;

pub struct BarterMessage;

pub struct Binance {
    socket: ExchangeSocket<WsStream, WsMessage, WebSocketParser, BinanceTransformer, BinanceMessage, MarketData>
}

pub struct BinanceTransformer;

impl Transformer<BinanceMessage, MarketData> for BinanceTransformer {
    type OutputIter = std::option::IntoIter<MarketData>;

    fn transform(&mut self, input: BinanceMessage) -> Result<Self::OutputIter, SocketError> {
        info!(payload = &*format!("{:?}", input), "received BinanceMessage");

        let market_data = match input {
            BinanceMessage::Trade(binance_trade) => {
                Some(MarketData::Trade(Trade::from(binance_trade)))
            }
            _ => None
        }.into_iter();

        Ok(market_data)
    }
}