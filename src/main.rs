use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

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

struct AppState {
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

    let network = state.network;
    let genesis_block_hex = state.genesis_digest.to_hex();

    let html = format!(r#"
        <html>
        <head>
         <title>Neptune Block Explorer: (network: {network})</title>
         <style>
          div.indent {{position: relative; left: 20px;}}
          body {{font-family: arial, helvetica;}}
          </style>
        </head>
        <body>
        <h1>Neptune Block Explorer (network: {network})</h1>
        <script>
        function handle_submit(form){{
            let value = form.height_or_digest.value;
            var is_digest = value.length == 80;
            var type = is_digest ? "digest" : "height";
            var uri = form.action + "/" + type + "/" + value;
            window.location.href = uri;
            return false;
        }}
        </script>
        <form action="/block" method="get" onsubmit="return handle_submit(this)">
        Block height or digest:
        <input type="text" size="80" name="height_or_digest"/>
        <input type="submit" name="height" value="Lookup Block"/>
        </form>

        Quick Lookup:
         <a href="/block/genesis">Genesis Block</a> |
         <a href="/block/tip">Tip</a><br/>

        <h2>REST RPCs</h2>

        <h3>/block_info</h3>
        <div class="indent">
            <h4>Examples</h4>

            <a href="/rpc/block_info/genesis">/rpc/block_info/genesis</a><br/>
            <a href="/rpc/block_info/tip">/rpc/block_info/tip</a><br/>
            <a href="/rpc/block_info/height/2">/rpc/block_info/height/2</a><br/>
            <a href="/rpc/block_info/digest/{genesis_block_hex}">/rpc/block_info/digest/{genesis_block_hex}</a><br/>
            <a href="/rpc/block_info/height_or_digest/1">/rpc/block_info/height_or_digest/1</a><br/>
        </div>

        <h3>/block_digest</h3>
        <div class="indent">
            <h4>Examples</h4>

            <a href="/rpc/block_digest/genesis">/rpc/block_digest/genesis</a><br/>
            <a href="/rpc/block_digest/tip">/rpc/block_digest/tip</a><br/>
            <a href="/rpc/block_digest/height/2">/rpc/block_digest/height/2</a><br/>
            <a href="/rpc/block_digest/digest/{genesis_block_hex}">/rpc/block_digest/digest/{genesis_block_hex}</a><br/>
            <a href="/rpc/block_digest/height_or_digest/{genesis_block_hex}">/rpc/block_digest/height_or_digest/{genesis_block_hex}</a><br/>
        </div>

        <h3>/utxo_digest</h3>
        <div class="indent">
            <h4>Examples</h4>

            <a href="/rpc/utxo_digest/2">/rpc/utxo_digest/2</a><br/>
        </div>

        </body>
        </html>
    "#);

    Html(html)
}

async fn block_page(
    Path(path_block_selector): Path<PathBlockSelector>,
    state: State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    let value_path: Path<(PathBlockSelector, String)> = Path((path_block_selector, "".to_string()));
    block_page_with_value(value_path, state).await
}

#[axum::debug_handler]
async fn block_page_with_value(
    Path((path_block_selector, value)): Path<(PathBlockSelector, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {

    let block_info = block_info_with_value_worker(state, path_block_selector, &value).await?;
    let BlockInfo {height, digest, timestamp, num_inputs, num_outputs, num_uncle_blocks, difficulty, mining_reward, fee, is_genesis, is_tip, ..} = block_info;

    let digest_hex = digest.to_hex();

    let prev_link = match is_genesis {
        true => "".to_string(),
        false => format!("<a href='/block/height/{}'>Previous Block</a>", height.previous())
    };

    let next_link = match is_tip {
        true => "".to_string(),
        false => format!("<a href='/block/height/{}'>Next Block</a>", height.next())
    };

    let special_block_notice = match (is_genesis, is_tip) {
        (true, false) => "<p>This is the Genesis Block</p>",
        (false, true) => "<p>This is the Latest Block (tip)</p>",
        _ => "",
    };

    let timestamp_display = timestamp.standard_format();

    let html = format!( r#"
    <html>
        <head>
         <title>Neptune Block Explorer: Block Height {height}</title>
         <style>
          div.indent {{position: relative; left: 20px;}}
          body {{font-family: arial, helvetica;}}
          table.alt {{margin-top: 10px; margin-bottom: 10px; padding: 10px; border-collapse: collapse; }}
          table.alt td, th {{ padding: 5px;}}
          table.alt tr:nth-child(odd) td{{background:#eee;}}
          table.alt tr:nth-child(even) td{{background:#fff;}}
          </style>
        </head>
        <body>
        <h1>Block height: {height}</h1>
        <b>Digest:</b> {digest_hex}

        {special_block_notice}

        <table class="alt">
        <tr><td>Created</td><td>{timestamp_display}</td></tr>
        <tr><td>Inputs</td><td>{num_inputs}<br/></td></tr>
        <tr><td>Outputs</td><td>{num_outputs}</td></tr>
        <tr><td>Uncle blocks</td><td>{num_uncle_blocks}</td></tr>
        <tr><td>Difficulty</td><td>{difficulty}</td></tr>
        <tr><td>Mining Reward</td><td>{mining_reward}</td></tr>
        <tr><td>Fee</td><td>{fee}</td></tr>
        </table>

        <p>
        <a href="/">Home</a>
        {prev_link}
        {next_link}
        </p>

        </body>
    </html>
    "# );

    Ok(Html(html))
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
        .route("/rpc/block_info/:selector", get(block_info))
        .route("/rpc/block_info/:selector/:value", get(block_info_with_value))
        .route(
            "/rpc/block_digest/:selector/:value",
            get(block_digest_with_value),
        )
        .route("/rpc/block_digest/:selector", get(block_digest))
        .route("/rpc/utxo_digest/:index", get(utxo_digest))
        .route("/", get(root))
        .route("/block/:selector", get(block_page))
        .route("/block/:selector/:value", get(block_page_with_value))
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
