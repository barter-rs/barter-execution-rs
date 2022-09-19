use barter_integration::model::Symbol;
use serde::{Deserialize, Serialize};

/// [`Balance`] associated with a [`Symbol`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SymbolBalance {
    pub symbol: Symbol,
    pub balance: Balance,
}

impl SymbolBalance {
    /// Construct a new [`SymbolBalance`] from a [`Symbol`] and it's associated [`Balance`].
    pub fn new<S>(symbol: S, balance: Balance) -> Self
    where
        S: Into<Symbol>
    {
        Self { symbol: symbol.into(), balance }
    }
}

/// Total and available balance values.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Balance {
    pub total: f64,
    pub available: f64,
}

impl Balance {
    /// Calculate the used (`total` - `available`) balance.
    pub fn used(&self) -> f64 {
        self.total - self.available
    }
}
