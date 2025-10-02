use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use chrono::DateTime;
use chrono::TimeDelta;
use chrono::Utc;
use clap::Parser;
use neptune_cash::api::export::Announcement;
use neptune_cash::api::export::Network;
use neptune_cash::application::config::data_directory::DataDirectory;
use neptune_cash::application::rpc::auth;
use neptune_cash::application::rpc::server::error::RpcError;
use neptune_cash::application::rpc::server::RPCClient;
use neptune_cash::application::rpc::server::RpcResult;
use neptune_cash::prelude::tasm_lib::prelude::Digest;
use neptune_cash::protocol::consensus::block::block_height::BlockHeight;
use neptune_cash::protocol::consensus::block::block_info::BlockInfo;
use neptune_cash::protocol::consensus::block::block_selector::BlockSelector;
use neptune_cash::util_types::mutator_set::addition_record::AdditionRecord;
use tarpc::client;
use tarpc::context;
use tarpc::tokio_serde::formats::Json as RpcJson;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::alert_email;
use crate::model::app_state::AppState;
use crate::model::config::Config;
use crate::model::transparent_utxo_tuple::TransparentUtxoTuple;

#[cfg(feature = "mock")]
const MOCK_KEY: &str = "MOCK";

#[derive(Debug, Clone)]
pub struct AuthenticatedClient {
    pub client: RPCClient,
    pub token: auth::Token,
    pub network: Network,
}

impl std::ops::Deref for AuthenticatedClient {
    type Target = RPCClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl AuthenticatedClient {
    /// Intercept and relay call to [`RPCClient::block_info`]
    pub async fn block_info(
        &self,
        ctx: ::tarpc::context::Context,
        token: auth::Token,
        block_selector: BlockSelector,
    ) -> ::core::result::Result<RpcResult<Option<BlockInfo>>, ::tarpc::client::RpcError> {
        let rpc_result = self.client.block_info(ctx, token, block_selector).await;

        // if the RPC call was successful, return that
        if let Ok(Ok(Some(_))) = rpc_result {
            return rpc_result;
        }

        // if MOCK environment variable is set and feature is enabled,
        // imagine some mock block info
        #[cfg(feature = "mock")]
        if std::env::var(MOCK_KEY).is_ok() {
            use blake3::Hasher;
            use rand::rngs::StdRng;
            use rand::Rng;
            use rand::SeedableRng;
            tracing::warn!("RPC query failed and MOCK flag set, so returning an imagined block");
            let mut hasher = Hasher::new();
            hasher.update(&block_selector.to_string().bytes().collect::<Vec<_>>());
            let mut rng = StdRng::from_seed(*hasher.finalize().as_bytes());
            let mut block_info: BlockInfo = rng.random();
            match block_selector {
                BlockSelector::Digest(digest) => {
                    block_info.digest = digest;
                }
                BlockSelector::Height(height) => {
                    block_info.height = height;
                }
                _ => {}
            };
            return Ok(Ok(Some(block_info)));
        }

        // otherwise, return the original error
        rpc_result
    }

    /// Intercept and relay call to [`RPCClient::utxo_digest`]
    pub async fn utxo_digest(
        &self,
        ctx: ::tarpc::context::Context,
        token: auth::Token,
        leaf_index: u64,
        _transparent_utxos_cache: Arc<Mutex<Vec<TransparentUtxoTuple>>>,
    ) -> ::core::result::Result<RpcResult<Option<Digest>>, ::tarpc::client::RpcError> {
        let rpc_result = self.client.utxo_digest(ctx, token, leaf_index).await;

        if let Ok(Ok(Some(_))) = rpc_result {
            return rpc_result;
        }

        // If mocking is enabled, it is possible that the cache contains a UTXO
        // for this index that was imagined in the past.
        #[cfg(feature = "mock")]
        if let Some(entry) = _transparent_utxos_cache
            .lock()
            .await
            .iter()
            .find(|tu| tu.aocl_leaf_index().is_some_and(|li| li == leaf_index))
        {
            tracing::warn!("returning a cached utxo");
            return Ok(Ok(Some(entry.addition_record().canonical_commitment)));
        }

        rpc_result
    }

    /// Intercept and relay call to [`RPCClient::announcements_in_block`]
    pub async fn announcements_in_block(
        &self,
        ctx: ::tarpc::context::Context,
        token: auth::Token,
        block_selector: BlockSelector,
    ) -> Result<Result<Option<Vec<Announcement>>, RpcError>, ::tarpc::client::RpcError> {
        let rpc_result = self
            .client
            .announcements_in_block(ctx, token, block_selector)
            .await;

        // if the RPC call was successful, return that
        if let Ok(Ok(Some(_))) = rpc_result {
            return rpc_result;
        }

        // if MOCK environment variable is set and feature is enabled,
        // imagine some mock block info
        #[cfg(feature = "mock")]
        if std::env::var(MOCK_KEY).is_ok() {
            use blake3::Hasher;
            use neptune_cash::api::export::TransparentTransactionInfo;
            use neptune_cash::prelude::triton_vm::prelude::BFieldElement;
            use rand::rngs::StdRng;
            use rand::Rng;
            use rand::SeedableRng;
            tracing::warn!(
                "RPC query failed and MOCK flag set, so returning an imagined announcement"
            );
            let mut hasher = Hasher::new();
            hasher.update(&block_selector.to_string().bytes().collect::<Vec<_>>());
            let mut rng = StdRng::from_seed(*hasher.finalize().as_bytes());

            // make sure the number of announcements matches with the block
            let block_info = self
                .block_info(ctx, token, block_selector)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            let num_announcements = block_info.num_announcements;

            let mut announcements = vec![];
            for _ in 0..num_announcements {
                let announcement = if rng.random_bool(0.5_f64) {
                    let message = (0..rng.random_range(0..256))
                        .map(|_| rng.random::<BFieldElement>())
                        .collect::<Vec<_>>();
                    Announcement::new(message)
                } else {
                    rng.random::<TransparentTransactionInfo>().to_announcement()
                };
                announcements.push(announcement);
            }

            return Ok(Ok(Some(announcements)));
        }

        // otherwise, return the original error
        rpc_result
    }

    /// Intercept and relay call to
    /// [`RPCClient::addition_record_indices_for_block`].
    ///
    /// Also take an extra argument for mocking purposes.
    pub async fn addition_record_indices_for_block(
        &self,
        ctx: ::tarpc::context::Context,
        token: auth::Token,
        block_selector: BlockSelector,
        _addition_records: &[AdditionRecord],
    ) -> ::core::result::Result<
        RpcResult<Vec<(AdditionRecord, Option<u64>)>>,
        ::tarpc::client::RpcError,
    > {
        let rpc_result = self
            .client
            .addition_record_indices_for_block(ctx, token, block_selector)
            .await;

        // if the RPC call was successful, return that
        if let Ok(Ok(_)) = rpc_result {
            return rpc_result;
        }

        // if MOCK environment variable is set and feature is enabled,
        // imagine some mock hash map
        #[cfg(feature = "mock")]
        if std::env::var(MOCK_KEY).is_ok() {
            use blake3::Hasher;
            use rand::rngs::StdRng;
            use rand::Rng;
            use rand::SeedableRng;
            tracing::warn!(
                "RPC query failed and MOCK flag set, so returning an imagined addition records"
            );
            let mut hasher = Hasher::new();
            hasher.update(&block_selector.to_string().bytes().collect::<Vec<_>>());
            let mut rng = StdRng::from_seed(*hasher.finalize().as_bytes());

            let aocl_offset = rng.random::<u64>() >> 1;
            let addition_record_indices = _addition_records
                .iter()
                .enumerate()
                .map(|(i, ar)| {
                    (
                        *ar,
                        if rng.random_bool(0.5_f64) {
                            Some(i as u64 + aocl_offset)
                        } else {
                            None
                        },
                    )
                })
                .collect::<HashMap<_, _>>();
            return Ok(Ok(Some(addition_record_indices)));
        }

        // otherwise, return the original error
        rpc_result
    }
}

/// generates RPCClient, for querying neptune-core RPC server.
pub async fn gen_authenticated_rpc_client() -> Result<AuthenticatedClient, anyhow::Error> {
    let client = gen_rpc_client().await?;

    let auth::CookieHint {
        data_directory,
        network,
    } = get_cookie_hint(&client, &None).await?;

    let token: auth::Token = auth::Cookie::try_load(&data_directory).await?.into();

    Ok(AuthenticatedClient {
        client,
        token,
        network,
    })
}

/// generates RPCClient, for querying neptune-core RPC server.
pub async fn gen_rpc_client() -> Result<RPCClient, anyhow::Error> {
    // Create connection to neptune-core RPC server
    let args: Config = Config::parse();
    let server_socket = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::LOCALHOST),
        args.neptune_rpc_port,
    );
    let transport = tarpc::serde_transport::tcp::connect(server_socket, RpcJson::default)
        .await
        .with_context(|| {
            format!("Failed to connect to neptune-core rpc service at {server_socket}")
        })?;
    Ok(RPCClient::new(client::Config::default(), transport).spawn())
}

// returns result with a CookieHint{ data_directory, network }.
//
// We use the data-dir provided by user if present.
//
// Otherwise we call cookie_hint() RPC to obtain data-dir.
// But the API might be disabled, which we detect and fallback to the default data-dir.
async fn get_cookie_hint(
    client: &RPCClient,
    data_dir: &Option<std::path::PathBuf>,
) -> anyhow::Result<auth::CookieHint> {
    async fn fallback(
        client: &RPCClient,
        data_dir: &Option<std::path::PathBuf>,
    ) -> anyhow::Result<auth::CookieHint> {
        let network = client.network(context::current()).await??;
        let data_directory = DataDirectory::get(data_dir.to_owned(), network)?;
        Ok(auth::CookieHint {
            data_directory,
            network,
        })
    }

    if data_dir.is_some() {
        return fallback(client, data_dir).await;
    }

    let result = client.cookie_hint(context::current()).await?;

    match result {
        Ok(hint) => Ok(hint),
        Err(RpcError::CookieHintDisabled) => fallback(client, data_dir).await,
        Err(e) => Err(e.into()),
    }
}

/// a tokio task that periodically pings neptune-core rpc server to ensure the
/// connection is still alive and/or attempts to re-establish connection.
///
/// If not connected, a single connection attempt is made for each timer iteration.
///
/// Whenever the connection changes state a log message is printed and an email
/// alert is sent to admin, if admin_email config field is set.  In this way,
/// the site admin gets notified of both outages and restoration of service.
pub async fn watchdog(app_state: AppState) {
    let app_started = chrono::offset::Utc::now();
    let mut was_connected = true;
    let mut since = chrono::offset::Utc::now();
    let watchdog_secs = app_state.load().config.neptune_rpc_watchdog_secs;

    debug!("neptune-core rpc watchdog started");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(watchdog_secs)).await;

        let result = app_state
            .load()
            .rpc_client
            .network(context::current())
            .await;

        let now_connected = result.is_ok();
        if now_connected != was_connected {
            // send admin alert of state change.
            let subject = match now_connected {
                true => "alert!  ** RECOVERY ** rpc connection restored",
                false => "alert!  ** OUTAGE ** rpc connection lost.",
            };

            let config = Config::parse();
            let now = chrono::offset::Utc::now();
            let duration = now.signed_duration_since(since);
            let app_duration = now.signed_duration_since(app_started);
            let body = NeptuneRpcAlertEmail {
                config,
                was_connected,
                now_connected,
                now,
                app_started,
                app_duration,
                since,
                duration,
            }
            .to_string();

            let msg = format!("alert: neptune-core rpc connection status change: previous: {was_connected}, now: {now_connected}");
            match now_connected {
                true => info!("{msg}"),
                false => warn!("{msg}"),
            }

            let _ = alert_email::send(&app_state, subject, body).await;

            was_connected = now_connected;
            since = chrono::offset::Utc::now();
        }

        if !now_connected {
            if let Ok(c) = gen_authenticated_rpc_client().await {
                app_state.set_rpc_client(c);
            }
        }
    }
}

#[derive(boilerplate::Boilerplate)]
#[boilerplate(filename = "email/neptune_rpc_alert.txt")]
pub struct NeptuneRpcAlertEmail {
    config: Config,
    was_connected: bool,
    now_connected: bool,
    app_started: DateTime<Utc>,
    app_duration: TimeDelta,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
    duration: TimeDelta,
}

#[derive(Clone, Copy, derive_more::Display)]
pub enum BlockchainState {
    Normal,
    Warn,
}

#[derive(boilerplate::Boilerplate)]
#[boilerplate(filename = "email/neptune_blockchain_alert.txt")]
pub struct NeptuneBlockchainAlertEmail {
    config: Config,
    last_height: BlockHeight,
    height: BlockHeight,
    last_blockchain_state: BlockchainState,
    blockchain_state: BlockchainState,
    app_started: DateTime<Utc>,
    app_duration: TimeDelta,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
    duration: TimeDelta,
}

/// a tokio task that periodically pings neptune-core rpc server to ensure
/// the blockchain keeps growing and has not stalled or shortened somehow.
///
/// If not connected, a single connection attempt is made for each timer iteration.
///
/// States:
///   normal:  the present tip is higher than at the last check.
///   warn:    the present tip is same or lower than at the last check.
///
/// Whenever the state changes a log message is printed and an email
/// alert is sent to admin, if admin_email config field is set.  In this way,
/// the site admin gets notified if a problem occurs, and upon recovery.
pub async fn blockchain_watchdog(app_state: AppState) {
    let mut last_height: BlockHeight = Default::default();
    let mut last_blockchain_state = BlockchainState::Normal;
    let app_started = chrono::offset::Utc::now();
    let mut since = chrono::offset::Utc::now();
    let watchdog_secs = app_state.load().config.neptune_blockchain_watchdog_secs;

    debug!("neptune-core blockchain watchdog started");

    loop {
        let result = {
            let s = app_state.load();
            s.rpc_client
                .block_height(context::current(), s.token())
                .await
        };

        if let Ok(Ok(height)) = result {
            // send admin alert if there is a state change.
            let subject = match last_blockchain_state {
                BlockchainState::Normal if height < last_height => {
                    "alert!  ** WARNING ** blockchain height is shrinking"
                }
                BlockchainState::Normal if height == last_height => {
                    "alert!  ** WARNING ** blockchain height is stalled"
                }
                BlockchainState::Warn if height > last_height => {
                    "alert!  ** Recovery ** blockchain height is growing again"
                }
                _ => "", // no state change
            };

            if !subject.is_empty() {
                let blockchain_state = match last_blockchain_state {
                    BlockchainState::Normal => BlockchainState::Warn,
                    BlockchainState::Warn => BlockchainState::Normal,
                };

                let config = Config::parse();
                let now = chrono::offset::Utc::now();
                let duration = now.signed_duration_since(since);
                let app_duration = now.signed_duration_since(app_started);
                let body = NeptuneBlockchainAlertEmail {
                    config,
                    last_height,
                    height,
                    last_blockchain_state,
                    blockchain_state,
                    now,
                    app_started,
                    app_duration,
                    since,
                    duration,
                }
                .to_string();

                let msg = format!("alert: neptune-core blockchain status change: previous: {last_blockchain_state}, now: {blockchain_state}.  prev_height: {last_height}, now_height: {height}");
                match blockchain_state {
                    BlockchainState::Normal => info!("{msg}"),
                    BlockchainState::Warn => warn!("{msg}"),
                };

                let _ = alert_email::send(&app_state, subject, body).await;

                last_blockchain_state = blockchain_state;
            }

            // update state.
            last_height = height;
            since = chrono::offset::Utc::now();
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(watchdog_secs)).await;
    }
}
