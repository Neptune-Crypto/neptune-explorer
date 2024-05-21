use axum::routing::get;
use axum::routing::Router;
use clap::Parser;
use neptune_core::models::blockchain::block::block_selector::BlockSelector;
use neptune_core::rpc_server::RPCClient;
use neptune_explorer::html::page::block::block_page;
use neptune_explorer::html::page::not_found::not_found_html_fallback;
use neptune_explorer::html::page::redirect_qs_to_path::redirect_query_string_to_path;
use neptune_explorer::html::page::root::root;
use neptune_explorer::html::page::utxo::utxo_page;
use neptune_explorer::model::app_state::AppState;
use neptune_explorer::model::config::Config;
use neptune_explorer::rpc::block_digest::block_digest;
use neptune_explorer::rpc::block_info::block_info;
use neptune_explorer::rpc::utxo_digest::utxo_digest;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use tarpc::client;
use tarpc::client::RpcError;
use tarpc::context;
use tarpc::tokio_serde::formats::Json as RpcJson;
use tower_http::services::ServeDir;

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
        .route("/rpc/block_info/*selector", get(block_info))
        .route("/rpc/block_digest/*selector", get(block_digest))
        .route("/rpc/utxo_digest/:index", get(utxo_digest))
        // -- Dynamic HTML pages --
        .route("/", get(root))
        .route("/block/*selector", get(block_page))
        .route("/utxo/:value", get(utxo_page))
        // -- Rewrite query-strings to path --
        .route("/rqs", get(redirect_query_string_to_path))
        // -- Static files --
        .nest_service(
            "/css",
            ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/web/css")),
        )
        .nest_service(
            "/image",
            ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/web/image")),
        )
        // handle route not-found
        .fallback(not_found_html_fallback(shared_state.clone()))
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
