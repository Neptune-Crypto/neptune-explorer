use crate::html::component::header::HeaderHtml;
use crate::http_util::not_found_err;
use crate::http_util::rpc_err;
use crate::model::app_state::AppState;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Html;
use axum::response::Response;
use html_escaper::Escape;
use html_escaper::Trusted;
use neptune_core::prelude::tasm_lib::Digest;
use std::sync::Arc;
use tarpc::context;

#[axum::debug_handler]
pub async fn utxo_page(
    Path(index): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/utxo.html")]
    pub struct UtxoHtmlPage {
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

    let header = HeaderHtml {
        site_name: "Neptune Explorer".to_string(),
        state: state.clone(),
    };

    let utxo_page = UtxoHtmlPage {
        index,
        header,
        digest,
    };
    Ok(Html(utxo_page.to_string()))
}
