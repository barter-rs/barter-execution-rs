use crate::{
    AccountEvent, ClientId, ExecutionClient, ExecutionError, Open, Order, OrderId, RequestCancel,
    RequestOpen, SymbolBalance
};
use std::str::FromStr;
use tokio::sync::mpsc::UnboundedSender;
use async_trait::async_trait;
use web3::contract::{Contract, Options};
use web3::contract::tokens::Tokenize;
use web3::ethabi::Address;
use web3::transports::WebSocket as Web3Socket;
use web3::types::{H160, TransactionParameters, U256};
use web3::Web3;

const WEI_PER_ETH: f64 = 1_000_000_000_000_000_000.0;

#[derive(Debug)] pub struct WeiAmount(U256);
#[derive(Debug)] pub struct EthAmount(f64);

impl From<WeiAmount> for EthAmount {
    fn from(wei: WeiAmount) -> Self {
        EthAmount(wei.0.as_u128() as f64 / WEI_PER_ETH) // Todo: make fallible
    }
}

pub struct Config {
    eth_node_url: String,
    eth_account_address: String,
    account_private_key: secp256k1::SecretKey,
}

pub struct SushiSwapV3 {
    private_key: secp256k1::SecretKey,
    web3: Web3<Web3Socket>,
    accounts: Vec<web3::types::Address>,
}

#[async_trait]
impl ExecutionClient for SushiSwapV3 {
    const CLIENT: ClientId = ClientId::SushiSwapV3;
    type Config = Config;

    async fn init(config: Self::Config, event_tx: UnboundedSender<AccountEvent>) -> Self {
        // Establish WebSocket connection to Infura ETH Node
        let websocket = Web3Socket::new(&config.eth_node_url)
            .await
            .unwrap();

        // Construct Web3 instance
        let web3 = web3::Web3::new(websocket);

        // Fetch (empty) collection of Web3 accounts
        let mut accounts = web3.eth().accounts().await.unwrap();

        // Add configured ETH account address to accounts
        accounts.push(H160::from_str(&config.eth_account_address).unwrap());

        Self {
            private_key: config.account_private_key,
            web3,
            accounts
        }
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError> {
        todo!()
    }

    async fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError> {
        for account in &self.accounts {
            let wei_balance = self.web3.eth().balance(account.clone(), None).await.unwrap();
            let eth_balance = EthAmount::from(WeiAmount(wei_balance));
            println!("Account {account:?}, Balance: {eth_balance:?}");
        }

        Ok(vec![])
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

impl SushiSwapV3 {
    pub async fn fetch_erc20_smart_contract_meta(&self, address: web3::types::Address) {
        let token_contract = Contract::from_json(
            self.web3.eth(),
            address,
            include_bytes!("abi/erc20.json")
        ).unwrap();

        let token_name: String = token_contract
            .query("name", (), None, Options::default(), None)
            .await
            .unwrap();

        let total_supply: U256 = token_contract
            .query("totalSupply", (), None, Options::default(), None)
            .await
            .unwrap();

        println!("Token: {token_name} has total supply: {total_supply}");
    }

    pub async fn swap(&self) {
        let router02_address = Address::from_str(
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
        ).unwrap();

        // Contract to allow us to query (idempotent) or call (changes state) contract functions
        let router02_contract = Contract::from_json(
            self.web3.eth(),
            router02_address,
            include_bytes!("abi/uniswap_router02.json"),
        ).unwrap();

        // This is the WETH address related to the contract, not ours
        let wrapped_eth_address: Address = router02_contract
            .query("WETH", (), None, Options::default(), None)
            .await
            .unwrap();
        println!("WETH address: {wrapped_eth_address:?}");

        // Dai Token Address: 0xc7AD46e0b8a400Bb3C915120d284AafbA8fc4735
        let dai_address = Address::from_str("0xc7ad46e0b8a400bb3c915120d284aafba8fc4735").unwrap();
        let valid_timestamp = Self::valid_timestamp(300000);

        // Estimate Gas
        let gas_estimate = router02_contract
            .estimate_gas(
                "swapExactETHForTokens",
                (
                    U256::from_dec_str("106662000000").unwrap(),               // amountOutMin
                    vec![wrapped_eth_address, dai_address],                           // Address[]
                    self.accounts[0],                                                 // Recipient Address (ours)
                    U256::from_dec_str(&valid_timestamp.to_string()).unwrap(), // Timeout
                ),
                self.accounts[0],
                Options {
                    value: Some(U256::exp10(18).checked_div(20.into()).unwrap()),
                    gas: Some(500_000.into()),
                    ..Default::default()
                },
            )
            .await
            .expect("Error");
        let gas_price = self.recent_gas_price().await;

        println!("Estimated Gas cost: {gas_estimate:?}");
        println!("Estimated Gas Price: {gas_price:?}");

        // Configure Transaction Data
        let transaction_data = router02_contract
            .abi()
            .function("swapExactETHForTokens")
            .unwrap()
            .encode_input(
                &(
                    U256::from_dec_str("106662000000").unwrap(),               // amountOutMin
                    vec![wrapped_eth_address, dai_address],                           // Token addresses
                    self.accounts[0],                                                 // Recipient Address (ours)
                    U256::from_dec_str(&valid_timestamp.to_string()).unwrap(), // Timeout
                ).into_tokens()
            )
            .unwrap();

        // Configure Transaction Parameters
        // Transaction ID (increment nonce by one each time)
        let nonce = self
            .web3
            .eth()
            .transaction_count(self.accounts[0], None)
            .await.unwrap();

        let transaction = TransactionParameters {
            nonce: Some(nonce),
            to: Some(router02_address), // Todo: Should this be my address like in .encode_input?
            gas: gas_estimate, // Maximum amount of gas allocated for the transaction
            gas_price: Some(gas_price), // Price per unit of gas, higher increases chance of transaction in block
            value: U256::exp10(18).checked_div(20.into()).unwrap(), // Amount of ETH in Wei units
            data: web3::types::Bytes(transaction_data),
            ..Default::default()
        };
        println!("Unsigned Transaction: {transaction:?}");

        // Sign transaction with wallet private key
        let signed_transaction = self
            .web3
            .accounts()
            .sign_transaction(transaction, &self.private_key)
            .await
            .unwrap();

        // Send transaction
        let result = self
            .web3
            .eth()
            .send_raw_transaction(signed_transaction.raw_transaction)
            .await
            .unwrap();

        println!("Successful transaction with hash: {:?}", result);
    }

    pub fn valid_timestamp(millis_in_future: u128) -> u128 {
        let start = std::time::SystemTime::now();
        let since_epoch = start.duration_since(std::time::UNIX_EPOCH).unwrap();
        since_epoch.as_millis().checked_add(millis_in_future).unwrap()
    }

    pub async fn recent_gas_price(&self) -> U256 {
        self.web3.eth().gas_price().await.unwrap()
    }

}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;
    use web3::types::Address;
    use crate::exchange::sushiswapv3::SushiSwapV3;
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let (
            event_tx,
            event_rx
        ) = mpsc::unbounded_channel();

        let config = Config {
            eth_node_url: env!("ETH_NODE_URL").to_string(),
            eth_account_address: env!("ETH_ACCOUNT_ADDRESS").to_string(), // 0x not included
            account_private_key: secp256k1::SecretKey::from_str(env!("ACCOUNT_PRIVATE_KEY")).unwrap()
        };

        let sushi = SushiSwapV3::init(config, event_tx).await;

        let _ = sushi.fetch_balances().await;

        sushi.fetch_erc20_smart_contract_meta(
            Address::from_str("0x42447D5f59d5BF78a82C34663474922bdf278162").unwrap()
        ).await;

        sushi.swap().await;
    }
}