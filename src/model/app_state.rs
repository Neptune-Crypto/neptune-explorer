use crate::model::config::Config;
use neptune_core::config_models::network::Network;
use neptune_core::prelude::twenty_first::math::digest::Digest;
use neptune_core::rpc_server::RPCClient;

#[readonly::make]
pub struct AppState {
    pub network: Network,
    pub config: Config,
    pub rpc_client: RPCClient,
    pub genesis_digest: Digest,
}

impl From<(Network, Config, RPCClient, Digest)> for AppState {
    fn from(
        (network, config, rpc_client, genesis_digest): (Network, Config, RPCClient, Digest),
    ) -> Self {
        Self {
            network,
            config,
            rpc_client,
            genesis_digest,
        }
    }
}
