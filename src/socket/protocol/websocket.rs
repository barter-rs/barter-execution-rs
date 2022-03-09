use serde::de::DeserializeOwned;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use crate::socket::{
    SocketError,
    protocol::ProtocolParser
};
use tokio_tungstenite::tungstenite::{Message as WsMessage, Error as WsError};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tracing::{trace, warn};

pub type WsStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WebSocketParser;

impl<ExchangeMessage> ProtocolParser<ExchangeMessage> for WebSocketParser
where
    ExchangeMessage: DeserializeOwned,
{
    type Input = Result<WsMessage, WsError>;

    fn parse(input: Self::Input) -> Result<Option<ExchangeMessage>, SocketError> {
        match input {
            Ok(ws_message) => match ws_message {
                WsMessage::Text(text) => process_payload(text.into_bytes()),
                WsMessage::Binary(binary) => process_payload(binary),
                WsMessage::Ping(ping) => process_ping(ping),
                WsMessage::Pong(pong) => process_pong(pong),
                WsMessage::Close(close_frame) => process_close_frame(close_frame),
            },
            Err(ws_err) => Err(SocketError::WebSocketError(ws_err))
        }
    }
}

pub fn process_payload<ExchangeMessage>(payload: Vec<u8>) -> Result<Option<ExchangeMessage>, SocketError>
where
    ExchangeMessage: DeserializeOwned,
{
    serde_json::from_slice::<ExchangeMessage>(&payload)
        .map(Option::Some)
        .map_err(|err| {
            warn!(
                error = &*format!("{:?}", err),
                payload = &*format!("{:?}", payload),
                action = "skipping message",
                "failed to deserialize WebSocket Message into domain specific Message"
            );
            SocketError::SerdeJsonError(err)
        })
}

pub fn process_ping<ExchangeMessage>(ping: Vec<u8>) -> Result<Option<ExchangeMessage>, SocketError> {
    trace!(payload = &*format!("{:?}", ping), "received Ping WebSocket message");
    Ok(None)
}

pub fn process_pong<ExchangeMessage>(pong: Vec<u8>) -> Result<Option<ExchangeMessage>, SocketError> {
    trace!(payload = &*format!("{:?}", pong), "received Pong WebSocket message");
    Ok(None)
}

pub fn process_close_frame<ExchangeMessage>(close_frame: Option<CloseFrame>) -> Result<Option<ExchangeMessage>, SocketError> {
    trace!(payload = &*format!("{:?}", close_frame), "received CloseFrame WebSocket message");
    Ok(None)
}