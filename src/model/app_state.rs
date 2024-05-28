use crate::model::config::Config;
use crate::neptune_rpc;
use anyhow::Context;
use arc_swap::ArcSwap;
use clap::Parser;
use neptune_core::config_models::network::Network;
use neptune_core::models::blockchain::block::block_selector::BlockSelector;
use neptune_core::prelude::twenty_first::math::digest::Digest;
use neptune_core::rpc_server::RPCClient;
use std::sync::Arc;

pub struct AppStateInner {
    pub network: Network,
    pub config: Config,
    pub rpc_client: RPCClient,
    pub genesis_digest: Digest,
}

#[derive(Clone)]
pub struct AppState(Arc<ArcSwap<AppStateInner>>);

impl std::ops::Deref for AppState {
    type Target = Arc<ArcSwap<AppStateInner>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<(Network, Config, RPCClient, Digest)> for AppState {
    fn from(
        (network, config, rpc_client, genesis_digest): (Network, Config, RPCClient, Digest),
    ) -> Self {
        Self(Arc::new(ArcSwap::from_pointee(AppStateInner {
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

    /// Sets the rpc_client
    ///
    /// This method exists because it is sometimes necessary
    /// to re-establish connection to the neptune RPC server.
    ///
    /// This is achieved via ArcSwap which is faster than
    /// RwLock for our use-case that is heavy reads and few
    /// if any mutations.  ArcSwap is effectively lock-free.
    ///
    /// Note that this method takes &self, so interior
    /// mutability occurs.
    pub fn set_rpc_client(&self, rpc_client: RPCClient) {
        let inner = self.0.load();

        let new_inner = AppStateInner {
            rpc_client,
            network: inner.network,
            config: inner.config.clone(),
            genesis_digest: inner.genesis_digest,
        };
        self.0.store(Arc::new(new_inner));
    }
}
