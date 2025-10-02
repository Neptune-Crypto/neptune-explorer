use std::sync::Arc;

use axum::extract::rejection::PathRejection;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Html;
use axum::response::Response;
use html_escaper::Escape;
use html_escaper::Trusted;
use neptune_cash::protocol::consensus::block::block_info::BlockInfo;
use tarpc::context;

use crate::html::component::header::HeaderHtml;
use crate::html::page::not_found::not_found_html_response;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;
use crate::model::block_selector_extended::BlockSelectorExtended;

#[axum::debug_handler]
pub async fn block_page(
    user_input_maybe: Result<Path<BlockSelectorExtended>, PathRejection>,
    State(state_rw): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/block_info.html")]
    pub struct BlockInfoHtmlPage<'a> {
        header: HeaderHtml<'a>,
        block_info: BlockInfo,
    }
    let state = &state_rw.load();

    let Path(block_selector) =
        user_input_maybe.map_err(|e| not_found_html_response(state, Some(e.to_string())))?;

    let block_info = match state
        .rpc_client
        .block_info(context::current(), state.token(), block_selector.into())
        .await
        .map_err(|e| not_found_html_response(state, Some(e.to_string())))?
        .map_err(rpc_method_err)?
    {
        Some(info) => Ok(info),
        None => Err(not_found_html_response(
            state,
            Some("Block does not exist".to_string()),
        )),
    }?;

    let header = HeaderHtml { state };

    let block_info_page = BlockInfoHtmlPage { header, block_info };
    Ok(Html(block_info_page.to_string()))
}
