
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
use pin_project_lite::pin_project;
use futures::Stream;
use serde::de::DeserializeOwned;
use thiserror::Error;
use pin_project::pin_project as heavy_pin_project;

#[derive(Debug, Error)]
pub enum SocketError {
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON SerDe error: {0}")]
    SerdeJsonError(#[from] serde_json::Error)
}

#[heavy_pin_project]
pub struct Test<T>
where
    T: Stream + Debug // Todo: This works with multiple impls, work out how to use this...
{
    #[pin]
    st: T,
}

pin_project! {
    pub struct ExchangeSocket<Socket, P, T, ExchangeMessage, Output>
    where
        Socket: Stream,
        P: ProtocolParser<ExchangeMessage>,
        T: Transformer<ExchangeMessage, Output>,
        ExchangeMessage: DeserializeOwned,
    {
        #[pin]
        socket: Socket,
        parser: P,
        transformer: T,
        exchange_message_marker: PhantomData<ExchangeMessage>,
        output_marker: PhantomData<Output>,
    }
}

impl<Socket, Input, P, T, ExchangeMessage, Output> Stream for ExchangeSocket<Socket, P, T, ExchangeMessage, Output>
where
    Socket: Stream<Item = Input>,
    P: ProtocolParser<ExchangeMessage, Input = Input>,
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