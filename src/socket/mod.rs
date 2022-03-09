
pub mod protocol;
pub mod transformer;

use std::fmt::Debug;
use std::marker::PhantomData;
use crate::socket::{
    protocol::ProtocolParser,
    transformer::Transformer
};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::{Sink, Stream};
use serde::de::DeserializeOwned;
use thiserror::Error;
use pin_project::pin_project;

#[derive(Debug, Error)]
pub enum SocketError {
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON SerDe error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Sink error")]
    SinkError,
}

#[pin_project]
pub struct ExchangeSocket<Socket, SinkInput, P, T, ExchangeMessage, Output>
where
    Socket: Sink<SinkInput> + Stream,
    P: ProtocolParser<ExchangeMessage>,
    T: Transformer<ExchangeMessage, Output>,
    ExchangeMessage: DeserializeOwned,
{
    #[pin]
    socket: Socket,
    parser: P,
    transformer: T,
    sink_input_marker: PhantomData<SinkInput>,
    exchange_message_marker: PhantomData<ExchangeMessage>,
    output_marker: PhantomData<Output>,
}

impl<Socket, SinkInput, StreamInput, P, T, ExchangeMessage, Output> Stream
    for ExchangeSocket<Socket, SinkInput, P, T, ExchangeMessage, Output>
where
    Socket: Sink<SinkInput> + Stream<Item = StreamInput>,
    P: ProtocolParser<ExchangeMessage, Input = StreamInput>,
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

impl<Socket, SinkInput, StreamInput, P, T, ExchangeMessage, Output> Sink<SinkInput>
    for ExchangeSocket<Socket, SinkInput, P, T, ExchangeMessage, Output>
where
    Socket: Sink<SinkInput> + Stream<Item = StreamInput>,
    P: ProtocolParser<ExchangeMessage, Input = StreamInput>,
    T: Transformer<ExchangeMessage, Output>,
    ExchangeMessage: DeserializeOwned,
{
    type Error = SocketError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_ready(cx).map_err(|_| SocketError::SinkError)
    }

    fn start_send(self: Pin<&mut Self>, item: SinkInput) -> Result<(), Self::Error> {
        self.project().socket.start_send(item).map_err(|_| SocketError::SinkError)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_flush(cx).map_err(|_| SocketError::SinkError)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_close(cx).map_err(|_| SocketError::SinkError)
    }
}













