use crate::html::component::header::HeaderHtml;
use crate::html::page::not_found::not_found_html_response;
use crate::model::app_state::AppState;
use axum::extract::rejection::PathRejection;
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
    index_maybe: Result<Path<u64>, PathRejection>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/utxo.html")]
    pub struct UtxoHtmlPage {
        header: HeaderHtml,
        index: u64,
        digest: Digest,
    }

    let Path(index) = index_maybe
        .map_err(|e| not_found_html_response(State(state.clone()), Some(e.to_string())))?;

    let digest = match state
        .rpc_client
        .utxo_digest(context::current(), index)
        .await
        .map_err(|e| not_found_html_response(State(state.clone()), Some(e.to_string())))?
    {
        Some(digest) => digest,
        None => {
            return Err(not_found_html_response(
                State(state.clone()),
                Some("The requested UTXO does not exist".to_string()),
            ))
        }
    };

    let header = HeaderHtml {
        state: state.clone(),
    };

    let utxo_page = UtxoHtmlPage {
        index,
        header,
        digest,
    };
    Ok(Html(utxo_page.to_string()))
}
