use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use tower_http::{
    services::ServeFile,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::ops::Deref;
use thiserror::Error;
use html_escaper::{Escape, Trusted};

use neptune_core::config_models::network::Network;
use neptune_core::models::blockchain::block::block_height::BlockHeight;
use neptune_core::rpc_server::{BlockInfo, BlockSelector, RPCClient};
use neptune_core::prelude::twenty_first::error::TryFromHexDigestError;
use neptune_core::prelude::twenty_first::math::digest::Digest;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tarpc::client::RpcError;
use tarpc::tokio_serde::formats::Json as RpcJson;
use tarpc::{client, context};

// note: http StatusCodes are defined at:
// https://docs.rs/http/1.1.0/http/status/struct.StatusCode.html

#[derive(Debug, clap::Parser, Clone)]
#[clap(name = "neptune-explorer", about = "Neptune Block Explorer")]
pub struct Config {
    /// Sets the server address to connect to.
    #[clap(long, default_value = "9799", value_name = "PORT")]
    port: u16,
}

pub struct AppState {
    network: Network,
    #[allow(dead_code)]
    config: Config,
    rpc_client: RPCClient,
    genesis_digest: Digest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum PathBlockSelector {
    #[serde(rename = "genesis")]
    Genesis,
    #[serde(rename = "tip")]
    Tip,
    #[serde(rename = "digest")]
    Digest,
    #[serde(rename = "height")]
    Height,
    #[serde(rename = "height_or_digest")]
    HeightOrDigest,
}

#[derive(Error, Debug)]
pub enum PathBlockSelectorError {
    #[error("Genesis does not accept an argument")]
    GenesisNoArg,

    #[error("Tip does not accept an argument")]
    TipNoArg,

    #[error("Digest could not be parsed")]
    DigestNotParsed(#[from] TryFromHexDigestError),

    #[error("Height could not be parsed")]
    HeightNotParsed(#[from] std::num::ParseIntError),
}
impl PathBlockSelectorError {
    fn as_response_tuple(&self) -> (StatusCode, String) {
        (StatusCode::NOT_FOUND, self.to_string())
    }
}
impl IntoResponse for PathBlockSelectorError {
    fn into_response(self) -> Response {
        self.as_response_tuple().into_response()
    }
}
impl From<PathBlockSelectorError> for Response {
    fn from(e: PathBlockSelectorError) -> Response {
        e.as_response_tuple().into_response()
    }
}

fn not_found_err() -> Response {
    (StatusCode::NOT_FOUND, "Not Found".to_string()).into_response()
}
fn rpc_err(e: RpcError) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}

impl PathBlockSelector {
    fn as_block_selector(&self, value: &str) -> Result<BlockSelector, PathBlockSelectorError> {
        match self {
            PathBlockSelector::Genesis if !value.is_empty() => {
                Err(PathBlockSelectorError::GenesisNoArg)
            }
            PathBlockSelector::Genesis => Ok(BlockSelector::Genesis),
            PathBlockSelector::Tip if !value.is_empty() => Err(PathBlockSelectorError::TipNoArg),
            PathBlockSelector::Tip => Ok(BlockSelector::Tip),
            PathBlockSelector::Digest => Ok(BlockSelector::Digest(Digest::try_from_hex(value)?)),
            PathBlockSelector::Height => Ok(BlockSelector::Height(BlockHeight::from(
                u64::from_str(value)?,
            ))),
            PathBlockSelector::HeightOrDigest => {
                Ok(match u64::from_str(value) {
                    Ok(height) => BlockSelector::Height(BlockHeight::from(height)),
                    Err(_) => BlockSelector::Digest(Digest::try_from_hex(value)?),
                })
            }
        }
    }
}

async fn block_digest_with_value_worker(
    state: Arc<AppState>,
    path_block_selector: PathBlockSelector,
    value: &str,
) -> Result<Json<Digest>, impl IntoResponse> {
    let block_selector = path_block_selector.as_block_selector(&value)?;

    match state
        .rpc_client
        .block_digest(context::current(), block_selector)
        .await
        .map_err(rpc_err)?
    {
        Some(digest) => Ok(Json(digest)),
        None => Err(not_found_err()),
    }
}

#[axum::debug_handler]
async fn block_digest(
    Path(path_block_selector): Path<PathBlockSelector>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Digest>, impl IntoResponse> {
    block_digest_with_value_worker(state, path_block_selector, "").await
}

#[axum::debug_handler]
async fn block_digest_with_value(
    Path((path_block_selector, value)): Path<(PathBlockSelector, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Digest>, impl IntoResponse> {
    block_digest_with_value_worker(state, path_block_selector, &value).await
}

async fn block_info_with_value_worker(
    state: Arc<AppState>,
    path_block_selector: PathBlockSelector,
    value: &str,
) -> Result<BlockInfo, Response> {
    let block_selector = path_block_selector.as_block_selector(&value)?;

    match state
        .rpc_client
        .block_info(context::current(), block_selector)
        .await
        .map_err(rpc_err)?
    {
        Some(info) => Ok(info),
        None => Err(not_found_err()),
    }
}

#[axum::debug_handler]
async fn block_info(
    Path(path_block_selector): Path<PathBlockSelector>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<BlockInfo>, Response> {
    let block_info = block_info_with_value_worker(state, path_block_selector, "").await?;
    Ok(Json(block_info))
}

#[axum::debug_handler]
async fn block_info_with_value(
    Path((path_block_selector, value)): Path<(PathBlockSelector, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<BlockInfo>, Response> {
    let block_info = block_info_with_value_worker(state, path_block_selector, &value).await?;
    Ok(Json(block_info))
}

#[axum::debug_handler]
async fn utxo_digest(
    Path(index): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Digest>, impl IntoResponse> {
    match state
        .rpc_client
        .utxo_digest(context::current(), index)
        .await
        .map_err(rpc_err)?
    {
        Some(digest) => Ok(Json(digest)),
        None => Err(not_found_err()),
    }
}

#[axum::debug_handler]
async fn root(
    State(state): State<Arc<AppState>>,
) -> Html<String> {

    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/root.html")]
    pub struct RootHtmlPage(Arc<AppState>);
    impl Deref for RootHtmlPage {
        type Target = AppState;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    let root_page = RootHtmlPage(state);
    Html(root_page.to_string())
}

async fn block_page(
    Path(path_block_selector): Path<PathBlockSelector>,
    state: State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    let value_path: Path<(PathBlockSelector, String)> = Path((path_block_selector, "".to_string()));
    block_page_with_value(value_path, state).await
}

#[derive(boilerplate::Boilerplate)]
#[boilerplate(filename = "web/html/components/header.html")]
pub struct HeaderHtml{
    site_name: String,
    state: Arc<AppState>,
}

#[axum::debug_handler]
async fn block_page_with_value(
    Path((path_block_selector, value)): Path<(PathBlockSelector, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {

    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/block_info.html")]
    pub struct BlockInfoHtmlPage{
        header: HeaderHtml,
        block_info: BlockInfo
    }

    let header = HeaderHtml{site_name: "Neptune Explorer".to_string(), state: state.clone()};

    let block_info = block_info_with_value_worker(state, path_block_selector, &value).await?;
    let block_info_page = BlockInfoHtmlPage{header, block_info};
    Ok(Html(block_info_page.to_string()))
}


#[axum::debug_handler]
async fn utxo_page(
    Path(index): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {

    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/utxo.html")]
    pub struct UtxoHtmlPage{
        header: HeaderHtml,
        index: u64,
        digest: Digest,
    }

    let digest = match state
        .rpc_client
        .utxo_digest(context::current(), index)
        .await
        .map_err(rpc_err)?
    {
        Some(digest) => digest,
        None => return Err(not_found_err()),
    };

    let header = HeaderHtml{site_name: "Neptune Explorer".to_string(), state: state.clone()};

    let utxo_page = UtxoHtmlPage{index, header, digest};
    Ok(Html(utxo_page.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), RpcError> {
    let rpc_client = rpc_client().await;
    let network = rpc_client.network(context::current()).await?;
    let genesis_digest = rpc_client.block_digest(context::current(), BlockSelector::Genesis).await?.expect("Genesis block should be found");

    let shared_state = Arc::new(AppState {
        rpc_client,
        config: Config::parse(),
        network,
        genesis_digest,
    });

    let app = Router::new()
        // -- RPC calls --
        .route("/rpc/block_info/:selector", get(block_info))
        .route("/rpc/block_info/:selector/:value", get(block_info_with_value))
        .route(
            "/rpc/block_digest/:selector/:value",
            get(block_digest_with_value),
        )
        .route("/rpc/block_digest/:selector", get(block_digest))
        .route("/rpc/utxo_digest/:index", get(utxo_digest))

        // -- Dynamic HTML pages --
        .route("/", get(root))
        .route("/block/:selector", get(block_page))
        .route("/block/:selector/:value", get(block_page_with_value))
        .route("/utxo/:value", get(utxo_page))

        // -- Static files --
        .route_service("/css/styles.css", ServeFile::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/web/css/styles.css")))

        // add state
        .with_state(shared_state);

    println!("Running on http://localhost:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

async fn rpc_client() -> RPCClient {
    // Create connection to neptune-core RPC server
    let args: Config = Config::parse();
    let server_socket = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), args.port);
    let transport = tarpc::serde_transport::tcp::connect(server_socket, RpcJson::default).await;
    let transport = match transport {
        Ok(transp) => transp,
        Err(err) => {
            eprintln!("{err}");
            panic!("Connection to neptune-core failed. Is a node running?");
        }
    };
    RPCClient::new(client::Config::default(), transport).spawn()
}
