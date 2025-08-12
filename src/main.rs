use anyhow::Context;
use axum::routing::get;
use axum::routing::post;
use axum::routing::Router;
use neptune_explorer::alert_email;
use neptune_explorer::html::page::announcement::announcement_page;
use neptune_explorer::html::page::block::block_page;
use neptune_explorer::html::page::not_found::not_found_html_fallback;
use neptune_explorer::html::page::redirect_qs_to_path::redirect_query_string_to_path;
use neptune_explorer::html::page::root::root;
use neptune_explorer::html::page::utxo::utxo_page;
use neptune_explorer::model::app_state::AppState;
use neptune_explorer::neptune_rpc;
use neptune_explorer::rpc::block_digest::block_digest;
use neptune_explorer::rpc::block_info::block_info;
use neptune_explorer::rpc::pow_puzzle::pow_puzzle;
use neptune_explorer::rpc::provide_pow_solution::provide_pow_solution;
use neptune_explorer::rpc::utxo_digest::utxo_digest;
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let filter = EnvFilter::from_default_env()
        // Set the base level when not matched by other directives to INFO.
        .add_directive("neptune_explorer=info".parse()?);

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app_state = AppState::init().await?;

    let routes = setup_routes(app_state.clone());

    let port = app_state.load().config.listen_port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .with_context(|| format!("Failed to bind to port {port}"))?;

    // this will log warnings if smtp not configured or mis-configured.
    alert_email::check_alert_params();

    tokio::task::spawn(neptune_rpc::watchdog(app_state.clone()));
    tokio::task::spawn(neptune_rpc::blockchain_watchdog(app_state));

    info!("Running on http://localhost:{port}");

    axum::serve(listener, routes)
        .await
        .with_context(|| "Axum server encountered an error")?;
    Ok(())
}

pub fn setup_routes(app_state: AppState) -> Router {
    Router::new()
        // -- RPC calls --
        .route("/rpc/block_info/*selector", get(block_info))
        .route("/rpc/block_digest/*selector", get(block_digest))
        .route("/rpc/utxo_digest/:index", get(utxo_digest))
        .route("/rpc/pow_puzzle/*address", get(pow_puzzle))
        .route("/rpc/provide_pow_solution", post(provide_pow_solution))
        // -- Dynamic HTML pages --
        .route("/", get(root))
        .route("/block/*selector", get(block_page))
        .route("/utxo/:value", get(utxo_page))
        .route("/announcement/*selector", get(announcement_page))
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
        .fallback(not_found_html_fallback)
        // add state
        .with_state(app_state.into())
}
