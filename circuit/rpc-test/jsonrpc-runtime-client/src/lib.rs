#![warn(missing_docs)]
//! Create implements wrapper over RPC client re-used from Parity's Bridge Relayer.
//! Underlying websocket vendor is https://docs.rs/jsonrpsee

pub use relay_substrate_client::{
    rpc::Substrate as SubstrateRPC, Client as SubstrateClient, ConnectionParams,
};

/// Implement Chain with Polkadot-like types for relay-client
pub mod polkadot_like_chain;
pub use polkadot_like_chain::PolkadotLike;

/// Useful Substrate network RPC queries
pub mod useful_queries;
pub use useful_queries::{get_first_header, get_metadata};

/// Run single transaction proof relay and stop.
pub async fn create_rpc_client(
    sub_params: &ConnectionParams,
) -> Result<SubstrateClient<PolkadotLike>, String> {
    let sub_client = SubstrateClient::<PolkadotLike>::try_connect(sub_params.clone())
        .await
        .map_err(|e| e.to_string())?;

    Ok(sub_client)
}
