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

    // let bundle_signer = env::var("FLASHBOTS_IDENTIFIER").unwrap_or_else(|e| {
    //     panic!("Please set the FLASHBOTS_IDENTIFIER environment variable");
    // });

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
