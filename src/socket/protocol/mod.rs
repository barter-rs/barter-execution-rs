pub mod websocket;

use crate::socket::{
    SocketError,
};
use serde::de::DeserializeOwned;

pub trait ProtocolParser<ExchangeMessage>
where
    ExchangeMessage: DeserializeOwned,
{
    type Input;
    fn parse(input: Self::Input) -> Result<Option<ExchangeMessage>, SocketError>;
}