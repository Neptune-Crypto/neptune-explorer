use crate::http_util::not_found_html_err;
use crate::http_util::not_found_html_handler;
use crate::model::app_state::AppStateInner;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::Response;
use html_escaper::Escape;

pub fn not_found_page(error_msg: Option<String>) -> Html<String> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/not_found.html")]
    #[allow(dead_code)]
    pub struct NotFoundHtmlPage {
        error_msg: String,
    }

    let not_found_page = NotFoundHtmlPage {
        error_msg: error_msg.unwrap_or_default(),
    };
    Html(not_found_page.to_string())
}

pub fn not_found_html_response(_state: &AppStateInner, error_msg: Option<String>) -> Response {
    not_found_html_err(not_found_page(error_msg))
}

#[axum::debug_handler]
pub async fn not_found_html_fallback() -> (StatusCode, Html<String>) {
    not_found_html_handler(not_found_page(None))
}
