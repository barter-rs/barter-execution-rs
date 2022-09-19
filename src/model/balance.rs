


#[derive(Debug, Clone)]
pub struct SymbolBalance {
    pub symbol: Symbol,
    pub balance: Balance,
}

#[derive(Clone, Copy, Debug)]
pub struct Balance {
    pub total: f64,
    pub available: f64,
}
