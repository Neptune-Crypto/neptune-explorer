use crate::model::config::Config;
use crate::neptune_rpc;
use anyhow::Context;
use clap::Parser;
use neptune_core::config_models::network::Network;
use neptune_core::models::blockchain::block::block_selector::BlockSelector;
use neptune_core::prelude::twenty_first::math::digest::Digest;
use neptune_core::rpc_server::RPCClient;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppStateInner {
    pub network: Network,
    pub config: Config,
    pub rpc_client: RPCClient,
    pub genesis_digest: Digest,
}

#[derive(Clone)]
pub struct AppState(Arc<RwLock<AppStateInner>>);

impl std::ops::Deref for AppState {
    type Target = Arc<RwLock<AppStateInner>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<(Network, Config, RPCClient, Digest)> for AppState {
    fn from(
        (network, config, rpc_client, genesis_digest): (Network, Config, RPCClient, Digest),
    ) -> Self {
        Self(Arc::new(RwLock::new(AppStateInner {
            network,
            config,
            rpc_client,
            genesis_digest,
        })))
    }
}

impl AppState {
    pub async fn init() -> Result<Self, anyhow::Error> {
        let rpc_client = neptune_rpc::gen_rpc_client()
            .await
            .with_context(|| "Failed to create RPC client")?;
        let network = rpc_client
            .network(tarpc::context::current())
            .await
            .with_context(|| "Failed calling neptune-core api: network")?;
        let genesis_digest = rpc_client
            .block_digest(tarpc::context::current(), BlockSelector::Genesis)
            .await
            .with_context(|| "Failed calling neptune-core api: block_digest")?
            .with_context(|| "neptune-core failed to provide a genesis block")?;

        Ok(Self::from((
            network,
            Config::parse(),
            rpc_client,
            genesis_digest,
        )))
    }
}
