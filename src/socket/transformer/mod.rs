use crate::socket::SocketError;
use serde::de::DeserializeOwned;

pub trait Transformer<ExchangeMessage, Output>
where
    ExchangeMessage: DeserializeOwned,
{
    type OutputIter: IntoIterator<Item = Output>;
    fn transform(&mut self, input: ExchangeMessage) -> Result<Self::OutputIter, SocketError>;
}