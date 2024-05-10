use axum::routing::get;
use axum::routing::Router;
use clap::Parser;
use neptune_core::rpc_server::BlockSelector;
use neptune_core::rpc_server::RPCClient;
use neptune_explorer::html::page::block::block_page;
use neptune_explorer::html::page::block::block_page_with_value;
use neptune_explorer::html::page::root::root;
use neptune_explorer::html::page::utxo::utxo_page;
use neptune_explorer::model::app_state::AppState;
use neptune_explorer::model::config::Config;
use neptune_explorer::rpc::block_digest::block_digest;
use neptune_explorer::rpc::block_digest::block_digest_with_value;
use neptune_explorer::rpc::block_info::block_info;
use neptune_explorer::rpc::block_info::block_info_with_value;
use neptune_explorer::rpc::utxo_digest::utxo_digest;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use tarpc::client;
use tarpc::client::RpcError;
use tarpc::context;
use tarpc::tokio_serde::formats::Json as RpcJson;
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() -> Result<(), RpcError> {
    let rpc_client = rpc_client().await;
    let network = rpc_client.network(context::current()).await?;
    let genesis_digest = rpc_client
        .block_digest(context::current(), BlockSelector::Genesis)
        .await?
        .expect("Genesis block should be found");

    let shared_state = Arc::new(AppState::from((
        network,
        Config::parse(),
        rpc_client,
        genesis_digest,
    )));

    let app = Router::new()
        // -- RPC calls --
        .route("/rpc/block_info/:selector", get(block_info))
        .route(
            "/rpc/block_info/:selector/:value",
            get(block_info_with_value),
        )
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
        .route_service(
            "/css/pico.min.css",
            ServeFile::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/web/css/pico.min.css"
            )),
        )
        .route_service(
            "/css/styles.css",
            ServeFile::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/web/css/styles.css"
            )),
        )
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
