use crate::html::component::header::HeaderHtml;
use crate::http_util::not_found_html_err;
use crate::model::app_state::AppState;
use axum::extract::State;
use axum::response::Html;
use axum::response::Response;
use html_escaper::Escape;
// use html_escaper::Trusted;
use std::sync::Arc;

// #[axum::debug_handler]
pub fn not_found_page(
    State(state): State<Arc<AppState>>,
    error_msg: Option<String>,
) -> Html<String> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/not_found.html")]
    #[allow(dead_code)]
    pub struct NotFoundHtmlPage {
        header: HeaderHtml,
        error_msg: String,
    }

    let header = HeaderHtml {
        site_name: "Neptune Explorer".to_string(),
        state: state.clone(),
    };

    let not_found_page = NotFoundHtmlPage {
        header,
        error_msg: error_msg.unwrap_or_default(),
    };
    Html(not_found_page.to_string())
}

pub fn not_found_html_response(
    State(state): State<Arc<AppState>>,
    error_msg: Option<String>,
) -> Response {
    not_found_html_err(not_found_page(State(state), error_msg))
}
