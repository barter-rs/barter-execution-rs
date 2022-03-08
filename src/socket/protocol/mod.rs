pub mod websocket;

use crate::socket::SocketError;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Sink;
use pin_project_lite::pin_project;
use serde::de::DeserializeOwned;
use tokio_tungstenite::tungstenite::{Message as WsMessage, Error as WsError};
use crate::socket::protocol::websocket::WsStream;

pub trait ProtocolParser<ExchangeMessage>
where
    ExchangeMessage: DeserializeOwned,
{
    type Input;
    fn parse(input: Self::Input) -> Result<Option<ExchangeMessage>, SocketError>;
}



pin_project!{
    pub struct TestSink {
        #[pin]
        socket: WsStream,
    }
}

impl Sink<WsMessage> for TestSink {
    type Error = WsError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
         self.project().socket.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: WsMessage) -> Result<(), Self::Error> {
        self.project().socket.start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_close(cx)
    }
}