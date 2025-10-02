use std::sync::Arc;

use anyhow::Context;
use arc_swap::ArcSwap;
use clap::Parser;
use neptune_cash::api::export::Network;
use neptune_cash::application::rpc::auth;
use neptune_cash::prelude::twenty_first::tip5::Digest;
use neptune_cash::protocol::consensus::block::block_selector::BlockSelector;
use tokio::sync::Mutex;

use crate::model::config::Config;
use crate::model::transparent_utxo_tuple::TransparentUtxoTuple;
use crate::neptune_rpc;

#[derive(Debug, Clone)]
pub struct AppStateInner {
    pub network: Network,
    pub config: Config,
    pub rpc_client: neptune_rpc::AuthenticatedClient,
    pub genesis_digest: Digest,

    /// Whenever an announcement of type transparent transaction info is fetched
    /// from the RPC endpoint, we learn information about UTXOs. Since we expect
    /// transparent transactions to be rare, it is okay to cache this in RAM
    /// instead of storing it on disk.
    pub transparent_utxos_cache: Arc<Mutex<Vec<TransparentUtxoTuple>>>,
}

impl AppStateInner {
    pub fn token(&self) -> auth::Token {
        self.rpc_client.token
    }
}

#[derive(Clone)]
pub struct AppState(Arc<ArcSwap<AppStateInner>>);

impl AppState {
    fn new(app_state_inner: AppStateInner) -> Self {
        Self(Arc::new(ArcSwap::from_pointee(app_state_inner)))
    }
}

impl std::ops::Deref for AppState {
    type Target = Arc<ArcSwap<AppStateInner>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AppState {
    pub async fn init() -> Result<Self, anyhow::Error> {
        let rpc_client = neptune_rpc::gen_authenticated_rpc_client()
            .await
            .with_context(|| "Failed to create RPC client")?;
        let genesis_digest = rpc_client
            .block_digest(
                tarpc::context::current(),
                rpc_client.token,
                BlockSelector::Genesis,
            )
            .await
            .with_context(|| "Failed calling neptune-core api: block_digest")?
            .with_context(|| "Failed calling neptune-core api method: block_digest")?
            .with_context(|| "neptune-core failed to provide a genesis block")?;

        Ok(AppState::new(AppStateInner {
            network: rpc_client.network,
            config: Config::parse(),
            rpc_client,
            genesis_digest,
            transparent_utxos_cache: Arc::new(Mutex::new(vec![])),
        }))
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
    pub fn set_rpc_client(&self, rpc_client: neptune_rpc::AuthenticatedClient) {
        let inner = self.0.load();

        let new_inner = AppStateInner {
            network: rpc_client.network,
            rpc_client,
            config: inner.config.clone(),
            genesis_digest: inner.genesis_digest,
            transparent_utxos_cache: inner.transparent_utxos_cache.clone(),
        };
        self.0.store(Arc::new(new_inner));
    }
}
