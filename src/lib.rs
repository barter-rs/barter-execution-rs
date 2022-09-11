#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

use self::model::ConnectionStatus;
use barter_integration::model::Instrument;
use tokio::sync::mpsc;

///! # Barter-Execution

/// Contains `ExchangeClient` implementations for specific exchanges.
pub mod exchange;

pub mod error;
pub mod event_loop;
pub mod model;
pub mod simulated;


#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn it_works() {



    }

}