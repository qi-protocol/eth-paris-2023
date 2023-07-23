mod bindings;
mod bundler;

use crate::bundler::{
    bundler::{BabyBundler, EthApiServer},
    server::JsonRpcServer,
};
use anyhow::Result;
use dotenv::dotenv;
use env_logger::Env;
use ethers::{
    providers::{Provider, Ws},
    types::U256,
};
use std::sync::Arc;
use std::{env, future::pending};

use aa_bundler_primitives::Wallet;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    dotenv().ok();
    let goerli_url = env::var("WSS_RPC").expect("WSS_RPC not set");

    let goerli_provider = Arc::new(
        Provider::<Ws>::connect(goerli_url.clone())
            .await
            .ok()
            .ok_or(anyhow::anyhow!("Error connecting to Goerli"))
            .unwrap(),
    );

    let phrase = env::var("PHRASE").expect("Please set the PHRASe environment variable");
    let wallet = Wallet::from_phrase(&phrase, &U256::from(80001)).unwrap();
    log::info!("{:?}", wallet.signer);

    let baby_bundler = BabyBundler::new(
        goerli_provider.clone(),
        U256::max_value(),
        U256::max_value(),
        wallet,
    );

    let server = JsonRpcServer::new("127.0.0.1:3000".to_string())
        .with_proxy(goerli_url.clone())
        .with_cors(vec!["*".to_string()]);

    let _handle = server.start(baby_bundler.into_rpc()).await?;
    let _ = pending::<Result<()>>().await;
    Ok(())
}


#[cfg(test)]
mod test {
    use jsonrpsee::rpc_params;
    use jsonrpsee::http_client::HttpClientBuilder;
    use jsonrpsee::core::client::ClientT;
    use tracing;
    use tracing_subscriber::fmt;
    use ethers::{
        types::Address,
        providers::{Provider, Middleware, Ws},
        
    };
    use aa_bundler_primitives::{UserOperation, UserOperationHash};
    use std::env;
    use dotenv::dotenv;
    use std::sync::Arc;
    use alloy_primitives::{Address as alloy_Address, U256 as alloy_U256};
    use alloy_sol_types::{sol, SolCall};

    sol! {
        #[derive(Debug)]
        function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts);
    }

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {

        fmt::Subscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .init();

        // test eth_chainId
        let url = "http://127.0.0.1:3000";
        let client = HttpClientBuilder::default().build(url)?;
        let params = rpc_params![];
        let response: Result<String, _> = client.request("eth_chainId", params).await;
        assert_eq!(response.unwrap(), "0x13881");

        // test eth_supportedEntryPoints
        let params = rpc_params![];
        let response: Result<Vec<Address>, _> = client.request("eth_supportedEntryPoints", params).await;
        assert_eq!(response.unwrap()[0], "0x5ff137d4b0fdcd49dca30c7cf57e578a026d2789".parse::<Address>().unwrap());

        // test eth_sendUserOperation
        dotenv().ok();
        let goerli_url = env::var("WSS_RPC").expect("WSS_RPC not set");
        let provider = Arc::new(
            Provider::<Ws>::connect(goerli_url.clone())
                .await
                .ok()
                .ok_or(anyhow::anyhow!("Error connecting to Goerli"))
                .unwrap(),
        );

        // SimpleAccount address
        let account = env::var("ACCOUNT_ADDRESS").expect("ACCOUNT_ADDRESS not set");
        // UserOperation Signature
        let signature = env::var("UO_SIGNATURE").expect("UO_SIGNATURE not set");

        let balance = provider
            .get_balance(account.clone().parse::<Address>().unwrap(), None)
            .await
            .unwrap();

        if balance == 0.into() {
            log::warn!("bundler account has zero balance");
        };

        let path = vec![
            // WETH address
            alloy_Address::parse_checksummed("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", None).unwrap(),
            // USDt address
            alloy_Address::parse_checksummed("0xdAC17F958D2ee523a2206206994597C13D831ec7", None).unwrap(),
        ];

        // get call data using alloy_sol_types
        let swap_eth = swapExactETHForTokensCall {
            amountOutMin: alloy_U256::from(0),
            path: path.clone(),
            to: account.parse::<alloy_Address>().unwrap(),
            deadline: alloy_U256::from(0),
        };
        let call_data = swap_eth.encode();

        // hard code user operation gas fields for testing
        let uo = UserOperation::default()
            .call_data(call_data.into())
            .sender(account.parse::<Address>().unwrap())
            .verification_gas_limit(100_000.into())
            .pre_verification_gas(21_000.into())
            .max_priority_fee_per_gas(1_000_000_000.into())
            .call_gas_limit(200_000.into())
            .max_fee_per_gas(3_000_000_000_u64.into())
            .max_priority_fee_per_gas(1_000_000_000.into())
            .signature(signature.parse().unwrap());

        let uos =  vec![
            uo.clone(),
        ];


        // Send user operation via eth_sendUserOperation to the JSON RPC server
        let params = rpc_params![
            uos,
            "0x5ff137d4b0fdcd49dca30c7cf57e578a026d2789".parse::<Address>().unwrap()
        ];
        let _response: Result<UserOperationHash, _> = client.request("eth_supportedEntryPoints", params).await;


        Ok(())
    }



}