
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

// Todo: Add proper error enum for BinanceMessage in Barter-Data
//  eg/ enum BinanceMessage { Error, BinancePayload }
// Todo: Can I optimise the use of generics? eg/ Socket::Item,

pub trait Transformer<ExchangeMessage, Output>
where
    ExchangeMessage: DeserializeOwned,
{
    type OutputIter: IntoIterator<Item = Output>;
    fn transform(&mut self, input: ExchangeMessage) -> Result<Self::OutputIter, SocketError>;
}

#[pin_project]
pub struct ExchangeSocket<Socket, SocketItem, StreamParser, StreamTransformer, ExchangeMessage, Output>
where
    Socket: Sink<SocketItem> + Stream,
    StreamParser: ProtocolParser<ExchangeMessage>,
    StreamTransformer: Transformer<ExchangeMessage, Output>,
    ExchangeMessage: DeserializeOwned,
{
    #[pin]
    socket: Socket,
    parser: StreamParser,
    transformer: StreamTransformer,
    socket_item_marker: PhantomData<SocketItem>,
    exchange_message_marker: PhantomData<ExchangeMessage>,
    output_marker: PhantomData<Output>,
}

impl<Socket, SocketItem, StreamItem, StreamParser, StreamTransformer, ExchangeMessage, Output> Stream
    for ExchangeSocket<Socket, SocketItem, StreamParser, StreamTransformer, ExchangeMessage, Output>
where
    Socket: Sink<SocketItem> + Stream<Item = StreamItem>,
    StreamParser: ProtocolParser<ExchangeMessage, Input = StreamItem>,
    StreamTransformer: Transformer<ExchangeMessage, Output>,
    ExchangeMessage: DeserializeOwned,
{
    type Item = Result<StreamTransformer::OutputIter, SocketError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.socket.poll_next(cx) {
            Poll::Ready(Some(input)) => {
                // Parse ExchangeMessage from Socket Stream<Item = StreamItem> & transform to Output
                match StreamParser::parse(input) {
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

impl<Socket, SocketItem, StreamParser, StreamTransformer, ExchangeMessage, Output> Sink<SocketItem>
    for ExchangeSocket<Socket, SocketItem, StreamParser, StreamTransformer, ExchangeMessage, Output>
where
    Socket: Sink<SocketItem> + Stream,
    StreamParser: ProtocolParser<ExchangeMessage>,
    StreamTransformer: Transformer<ExchangeMessage, Output>,
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

impl<Socket, SocketItem, StreamParser, StreamTransformer, ExchangeMessage, Output>
    ExchangeSocket<Socket, SocketItem, StreamParser, StreamTransformer, ExchangeMessage, Output>
where
    Socket: Sink<SocketItem> + Stream,
    StreamParser: ProtocolParser<ExchangeMessage>,
    StreamTransformer: Transformer<ExchangeMessage, Output>,
    ExchangeMessage: DeserializeOwned,
{
    pub fn new(socket: Socket, parser: StreamParser, transformer: StreamTransformer) -> Self {
        Self {
            socket,
            parser,
            transformer,
            socket_item_marker: PhantomData::default(),
            exchange_message_marker: PhantomData::default(),
            output_marker: PhantomData::default()
        }
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