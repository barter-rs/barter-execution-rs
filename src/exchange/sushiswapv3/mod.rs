use crate::{
    AccountEvent, ClientId, ExecutionClient, ExecutionError, Open, Order, OrderId, RequestCancel,
    RequestOpen, SymbolBalance
};
use tokio::sync::mpsc::UnboundedSender;


struct Config {
    eth_node_url: String,
    eth_account_address: String,
}

struct SushiSwapV3 {

}

impl ExecutionClient for SushiSwapV3 {
    const CLIENT: ClientId = ClientId::SushiSwapV3;
    type Config = Config;

    async fn init(config: Self::Config, event_tx: UnboundedSender<AccountEvent>) -> Self {
        todo!()
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError> {
        todo!()
    }

    async fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError> {
        todo!()
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Result<Vec<Order<Open>>, ExecutionError> {
        todo!()
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Result<Vec<OrderId>, ExecutionError> {
        todo!()
    }

    async fn cancel_orders_all(&self) -> Result<(), ExecutionError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let node_url = env!("ETH_NODE_URL");
        let account_address = env!("ETH_ACCOUNT_ADDRESS");


    }
}