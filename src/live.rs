use crate::socket::{
    ExchangeSocket, SocketError, Transformer,
    protocol::websocket::{WebSocketParser, WebSocket}
};
use barter_data::client::binance::BinanceMessage;
use barter_data::model::{MarketData, Trade};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::info;

/// Eg/ Get data from the exchange like barter-data-rs

pub struct BinanceRequest;

pub struct BarterMessage;

pub struct Binance {
    pub socket: ExchangeSocket<WebSocket, WsMessage, WebSocketParser, BinanceTransformer, BinanceMessage, MarketData>
}

pub struct BinanceTransformer;

impl Transformer<BinanceMessage, MarketData> for BinanceTransformer {
    type OutputIter = std::option::IntoIter<MarketData>;

    fn transform(&mut self, input: BinanceMessage) -> Result<Self::OutputIter, SocketError> {
        let market_data = match input {
            BinanceMessage::Trade(binance_trade) => {
                Some(MarketData::Trade(Trade::from(binance_trade)))
            }
            _ => {
                info!(payload = &*format!("{:?}", input), "received other BinanceMessage");
                None
            }
        }.into_iter();

        Ok(market_data)
    }
}

#[cfg(test)]
mod tests {
    use barter_data::client::binance::BinanceSub;
    use barter_data::Subscription;
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;
    use tracing::error;
    use super::*;


    const BASE_URI: &'static str = "wss://stream.binance.com:9443/ws";
    const TRADE_STREAM: &'static str = "@aggTrade";

    /// Connect asynchronously to an exchange's server, returning a [`WebSocketStream`].
    async fn connect(base_uri: &str) -> Result<WebSocket, SocketError> {
        println!("Establishing WebSocket connection to: {:?}", base_uri);
        connect_async(base_uri)
            .await
            .and_then(|(ws_socket, _)| Ok(ws_socket))
            .map_err(|err| SocketError::WebSocketError(err))
    }

    #[tokio::test]
    async fn it_works() {
        // Connect
        let mut binance = Binance {
            socket:  ExchangeSocket::new(
                connect(BASE_URI).await.unwrap(),
                WebSocketParser,
                BinanceTransformer)
        };

        // Subscribe
        let trade_sub = BinanceSub::new(
            TRADE_STREAM.to_owned(), "btcusdt".to_owned()
        );
         binance
             .socket
             .send(WsMessage::Text(trade_sub.as_text().unwrap()))
             .await.unwrap();

        while let Some(Ok(messages)) = binance.socket.next().await {
            messages.into_iter().for_each(|message| println!("{:?}", message))
        }
    }
}