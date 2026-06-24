use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Response;
use neptune_cash::application::rpc::server::error::RpcError;
use tarpc::client::RpcError as TarpcError;

// note: http StatusCodes are defined at:
// https://docs.rs/http/1.1.0/http/status/struct.StatusCode.html

pub fn not_found_err() -> Response {
    (StatusCode::NOT_FOUND, "Not Found".to_string()).into_response()
}

pub fn not_found_html_err(html: Html<String>) -> Response {
    (StatusCode::NOT_FOUND, html).into_response()
}

pub fn not_found_html_handler(html: Html<String>) -> (StatusCode, Html<String>) {
    (StatusCode::NOT_FOUND, html)
}

pub fn rpc_err(e: TarpcError) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}")).into_response()
}

/// 503 with a styled HTML body, for a feature that is disabled because of the
/// connected node's configuration (e.g. the node maintains no UTXO index).
pub fn service_unavailable_html(html: Html<String>) -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, html).into_response()
}

/// 503 with a plain-text message, for disabled JSON/REST endpoints.
pub fn service_unavailable_err(message: &str) -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, message.to_string()).into_response()
}

pub fn rpc_method_err(e: RpcError) -> Response {
    let status_code = match e {
        RpcError::Auth(_) => StatusCode::UNAUTHORIZED,
        _ => StatusCode::BAD_REQUEST,
    };
    (status_code, format!("{e:?}")).into_response()
}
