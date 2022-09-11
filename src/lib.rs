#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

use self::model::{ConnectionStatus};
use barter_integration::model::Instrument;

///! # Barter-Execution

/// Contains `ExchangeClient` implementations for specific exchanges.
pub mod exchange;

pub mod error;
pub mod event_loop;
pub mod model;
pub mod simulated;

/// Responsibilities:
/// - Determines best way to action an [`ExchangeRequest`] given the constraints of the exchange.
pub trait ExchangeClient {
    fn instruments(&self) -> &[Instrument];
    fn connection_status(&self) -> ConnectionStatus;

    fn fetch_orders_open(&self) -> ();
    fn fetch_balances(&self) -> ();

    fn open_order(&self) -> ();
    fn open_order_batch(&self) -> ();

    fn cancel_order_by_id(&self) -> ();
    fn cancel_order_by_instrument(&self) -> ();
    fn cancel_order_by_batch(&self) -> ();
    fn cancel_order_all(&self) -> ();
}



#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn it_works() {



    }

}