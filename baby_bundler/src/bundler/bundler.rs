use crate::bindings::entrypointgoerli::entrypointgoerli;
use aa_bundler_primitives::{
    UserOperation, UserOperationHash,
    UserOperationReceipt, Wallet,
};
use mev_share_rpc_api::{BundleItem, FlashbotsSignerLayer, MevApiClient, SendBundleRequest};
use tokio::task::JoinHandle;
use url::Url;
use async_trait::async_trait;
use ethers::{
    prelude::{SignerMiddleware, LocalWallet},
    providers::{Middleware, Ws, Provider},
    signers::Signer,
    types::{transaction::eip2718::TypedTransaction, Address, H160, U256, U64},
};
use jsonrpsee::http_client::{transport::Error as HttpError, HttpClientBuilder};
use jsonrpsee::{
    core::RpcResult,
    proc_macros::rpc,
    tracing::info,
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tower::ServiceBuilder;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware, PendingBundleError::BundleNotIncluded};

/// A simplified bundler implementation based on AA-Bundler
/// https://github.com/Vid201/aa-bundler
pub struct BabyBundler<M: Middleware> {
    /// The Provider that connects to Goerli
    pub eth_provider: Arc<M>,
    /// Goerli Chain ID
    pub eth_chain_id: U64,
    /// Entry point address
    pub entry_point: Address,
    /// Max verification gas
    pub max_verification_gas: U256,
    /// Call gas Limit
    pub call_gas_limit: U256,
    /// Bundler wallet
    pub wallet: Wallet,
}

impl<M> BabyBundler<M>
where
    M: Middleware + 'static,
    M::Provider: Send + Sync + 'static,
{
    pub fn new(
        eth_provider: Arc<M>,
        max_verification_gas: U256,
        call_gas_limit: U256,
        wallet: Wallet,
    ) -> Self {
	// let bundle_signer = env::var("FLASHBOTS_IDENTIFIER").unwrap_or_else(|e| {
	//     panic!("Please set the FLASHBOTS_IDENTIFIER environment variable");
	// });
        Self {
            eth_provider,
            eth_chain_id: U64::from(5),
            entry_point: H160::from_str("0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789").unwrap(),
            max_verification_gas,
            call_gas_limit,
            wallet,
        }
    }
}

/// Eth API trait ported from AA-Bundler
///  https://github.com/Vid201/aa-bundler/blob/main/crates/rpc/src/eth_api.rs
#[derive(Serialize, Deserialize, Clone)]
pub struct EstimateUserOperationGasResponse {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
}

#[rpc(server, namespace = "eth")]
pub trait EthApi {
    #[method(name = "chainId")]
    async fn chain_id(&self) -> RpcResult<U64>;
    #[method(name = "supportedEntryPoints")]
    async fn supported_entry_points(&self) -> RpcResult<Vec<Address>>;
    #[method(name = "sendUserOperation")]
    async fn send_user_operation(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<UserOperationHash>;
    #[method(name = "estimateUserOperationGas")]
    async fn estimate_user_operation_gas(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<EstimateUserOperationGasResponse>;
    #[method(name = "getUserOperationReceipt")]
    async fn get_user_operation_receipt(
        &self,
        user_operation_hash: UserOperationHash,
    ) -> RpcResult<Option<UserOperationReceipt>>;
}

#[async_trait]
impl<M> EthApiServer for BabyBundler<M>
where
    M: Middleware + 'static,
    M::Provider: Send + Sync,
{
    async fn chain_id(&self) -> RpcResult<U64> {
        Ok(U64::from(80001))
    }

    async fn supported_entry_points(&self) -> RpcResult<Vec<Address>> {
        Ok(vec![H160::from_str(
            "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
        )
        .unwrap()])
    }

    async fn send_user_operation(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<UserOperationHash> {

	dotenv().ok();
	let goerli_url = env::var("WSS_RPC").expect("WSS_RPC not set");

	let provider = Arc::new(
		Provider::<Ws>::connect(goerli_url.clone())
			.await
			.ok()
			.ok_or(anyhow::anyhow!("Error connecting to Goerli"))
			.unwrap(),
	);

        let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER")
            .expect("FLASHBOTS_IDENTIFIER environment variable is not set");

        let bundle_signer = _bundle_signer.parse::<LocalWallet>().unwrap();
	let signing_middleware = FlashbotsSignerLayer::new(bundle_signer.clone());
	let service_builder = ServiceBuilder::new()
		.map_err(HttpError::Http)
		.layer(signing_middleware);

        let entry_point_instance =
            entrypointgoerli::entrypointgoerli::new(entry_point, provider.clone());

        let nonce = provider
            .clone()
            .get_transaction_count(self.wallet.signer.address(), None)
            .await
            .unwrap();

        let mut user_op_vec = Vec::new();
        user_op_vec.push(user_operation.clone());
        let mut tx: TypedTransaction = entry_point_instance
            .handle_ops(user_op_vec, self.wallet.signer.address())
            .tx
            .clone();
        tx.set_nonce(nonce).set_chain_id(U64::from(80001));

	let typed_tx = TypedTransaction::Eip1559(tx.clone().into());
	let raw_tx = self.wallet.signer.clone().sign_transaction(&typed_tx).await.unwrap();
	let raw_signed_tx = tx.rlp_signed(&raw_tx);

        // Add tx to Flashbots bundle
        let mut bundle_req = BundleRequest::new();
        bundle_req = bundle_req.push_transaction(raw_signed_tx.clone());
	let tx_hash = bundle_req.transaction_hashes()[0];

	// Build bundle
	let mut bundle_body = Vec::new();
	bundle_body.push(BundleItem::Hash { hash: tx_hash });
	bundle_body.push(BundleItem::Tx { tx: raw_signed_tx, can_revert: false });

	let bundle = SendBundleRequest { bundle_body, ..Default::default() };

	// Set up the rpc client
	let url = "https://relay.flashbots.net:443";
	let client = HttpClientBuilder::default()
		.set_middleware(service_builder)
		.build(url)
		.expect("Failed to create http client");

	// Send bundle
	let res = client.send_bundle(bundle.clone()).await.unwrap();
	log::info!("Got a bundle response: {:?}", res);

        return Ok(UserOperationHash(res.bundle_hash));
    }

    // TODO: Implement this
    async fn estimate_user_operation_gas(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<EstimateUserOperationGasResponse> {
        info!("{:?}", user_operation);
        info!("{:?}", entry_point);
        Ok(EstimateUserOperationGasResponse {
            pre_verification_gas: U256::from(0),
            verification_gas_limit: U256::from(0),
            call_gas_limit: U256::from(self.call_gas_limit),
        })
    }

    // TODO: Implement this
    async fn get_user_operation_receipt(
        &self,
        user_operation_hash: UserOperationHash,
    ) -> RpcResult<Option<UserOperationReceipt>> {
        info!("{:?}", user_operation_hash);
        Ok(None)
    }
}