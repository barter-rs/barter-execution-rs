use barter_integration::model::Symbol;
use serde::{Deserialize, Serialize};

/// Todo:
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SymbolBalance {
    pub symbol: Symbol,
    pub balance: Balance,
}

/// Todo:
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Balance {
    pub total: f64,
    pub available: f64,
}
