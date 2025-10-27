use std::net::IpAddr;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::routing::get;
use axum::routing::post;
use axum::routing::Router;
use axum_gcra::RateLimitLayer;
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
use neptune_explorer::rpc::circulating_supply::circulating_supply;
use neptune_explorer::rpc::pow_puzzle::pow_puzzle;
use neptune_explorer::rpc::provide_pow_solution::provide_pow_solution;
use neptune_explorer::rpc::total_supply::total_supply;
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

    axum::serve(
        listener,
        routes.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
        .route("/rpc/circulating_supply", get(circulating_supply))
        .route("/rpc/total_supply", get(total_supply))
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
        // apply rate-limiting
        .route_layer(
            RateLimitLayer::<IpExtractor>::builder()
                // default quota: 1 request every 10 milliseconds per IP
                .with_default_quota(axum_gcra::gcra::Quota::simple(Duration::from_millis(10)))
                .default_handle_error(), // Handles rate limit exceeded errors gracefully
        )
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct IpExtractor {
    ip: IpAddr,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for IpExtractor
where
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        #[cfg(feature = "attacks")]
        {
            if let Some(ip_header_value) = headers.get("X-Real-IP-Override") {
                if let Ok(ip_str) = ip_header_value.to_str() {
                    if let Ok(ip) = IpAddr::from_str(ip_str) {
                        info!("hi from {ip}");
                        return Ok(IpExtractor { ip });
                    }
                }
            }
        }

        let ip = {
            // If feature flag "attacks" is disabled, or if for whatever reason
            // extracting the mock IP from the header "X-Real-IP-Override"
            // false, fall back to the default: extracting the IP from the
            // socket address.
            let socket_ip = parts
                .extensions
                .get::<axum::extract::connect_info::ConnectInfo<std::net::SocketAddr>>()
                .map(|connect_info| connect_info.0.ip());
            if let Some(ip) = socket_ip {
                ip
            } else {
                return Err(axum::response::Response::builder()
                    .status(400)
                    .body("No IP found".into())
                    .unwrap());
            }
        };

        Ok(IpExtractor { ip })
    }
}
