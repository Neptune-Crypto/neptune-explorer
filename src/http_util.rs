use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Response;
use tarpc::client::RpcError;

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

pub fn rpc_err(e: RpcError) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}
