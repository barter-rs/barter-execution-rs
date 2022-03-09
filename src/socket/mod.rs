
pub mod protocol;

use crate::socket::protocol::ProtocolParser;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use serde::de::DeserializeOwned;
use thiserror::Error;
use pin_project::pin_project;
use futures::{Sink, Stream};

pub trait Transformer<ExchangeMessage, Output>
where
    ExchangeMessage: DeserializeOwned,
{
    type OutputIter: IntoIterator<Item = Output>;
    fn transform(&mut self, input: ExchangeMessage) -> Result<Self::OutputIter, SocketError>;
}

#[pin_project]
pub struct ExchangeSocket<Socket, SocketItem, P, T, ExchangeMessage, Output>
where
    Socket: Sink<SocketItem> + Stream,
    P: ProtocolParser<ExchangeMessage>,
    T: Transformer<ExchangeMessage, Output>,
    ExchangeMessage: DeserializeOwned,
{
    #[pin]
    socket: Socket,
    parser: P,
    transformer: T,
    socket_item_marker: PhantomData<SocketItem>,
    exchange_message_marker: PhantomData<ExchangeMessage>,
    output_marker: PhantomData<Output>,
}

impl<Socket, SocketItem, P, T, ExchangeMessage, Output> Stream
    for ExchangeSocket<Socket, SocketItem, P, T, ExchangeMessage, Output>
where
    Socket: Sink<SocketItem> + Stream<Item = SocketItem>,
    P: ProtocolParser<ExchangeMessage, Input = SocketItem>,
    T: Transformer<ExchangeMessage, Output>,
    ExchangeMessage: DeserializeOwned,
{
    type Item = Result<T::OutputIter, SocketError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.socket.poll_next(cx) {
            Poll::Ready(Some(input)) => {
                // Parse ExchangeMessage from Socket Stream<Item = Input> & transform to Output
                match P::parse(input) {
                    Ok(Some(exchange_message)) => {
                        Poll::Ready(Some(this.transformer.transform(exchange_message)))
                    }
                    Ok(None) => {
                        // If parser succeeds but returns None it's a safe-to-skip message
                        Poll::Pending
                    }
                    Err(err) => {
                        Poll::Ready(Some(Err(err)))
                    }
                }
            }
            Poll::Ready(None) => {
                Poll::Ready(None)
            }
            Poll::Pending => {
                Poll::Pending
            }
        }
    }
}

impl<Socket, SocketItem, P, T, ExchangeMessage, Output> Sink<SocketItem>
    for ExchangeSocket<Socket, SocketItem, P, T, ExchangeMessage, Output>
where
    Socket: Sink<SocketItem> + Stream<Item = SocketItem>,
    P: ProtocolParser<ExchangeMessage, Input = SocketItem>,
    T: Transformer<ExchangeMessage, Output>,
    ExchangeMessage: DeserializeOwned,
{
    type Error = SocketError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_ready(cx).map_err(|_| SocketError::SinkError)
    }

    fn start_send(self: Pin<&mut Self>, item: SocketItem) -> Result<(), Self::Error> {
        self.project().socket.start_send(item).map_err(|_| SocketError::SinkError)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_flush(cx).map_err(|_| SocketError::SinkError)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_close(cx).map_err(|_| SocketError::SinkError)
    }
}

#[derive(Debug, Error)]
pub enum SocketError {
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON SerDe error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Sink error")]
    SinkError,
}