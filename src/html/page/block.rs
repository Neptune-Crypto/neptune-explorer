use crate::html::component::header::HeaderHtml;
use crate::html::page::not_found::not_found_html_response;
use crate::model::app_state::AppState;
use crate::model::path_block_selector::PathBlockSelector;
use axum::extract::rejection::PathRejection;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Html;
use axum::response::Response;
use html_escaper::Escape;
use html_escaper::Trusted;
use neptune_core::models::blockchain::block::block_info::BlockInfo;
use std::sync::Arc;
use tarpc::context;

pub async fn block_page(
    user_input_maybe: Result<Path<PathBlockSelector>, PathRejection>,
    state: State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    let Path(path_block_selector) = user_input_maybe
        .map_err(|e| not_found_html_response(state.clone(), Some(e.to_string())))?;

    let value_path: Path<(PathBlockSelector, String)> = Path((path_block_selector, "".to_string()));
    block_page_with_value(Ok(value_path), state).await
}

#[axum::debug_handler]
pub async fn block_page_with_value(
    user_input_maybe: Result<Path<(PathBlockSelector, String)>, PathRejection>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/block_info.html")]
    pub struct BlockInfoHtmlPage {
        header: HeaderHtml,
        block_info: BlockInfo,
    }

    let Path((path_block_selector, value)) = user_input_maybe
        .map_err(|e| not_found_html_response(State(state.clone()), Some(e.to_string())))?;

    let header = HeaderHtml {
        site_name: "Neptune Explorer".to_string(),
        state: state.clone(),
    };

    let block_selector = path_block_selector
        .as_block_selector(&value)
        .map_err(|e| not_found_html_response(State(state.clone()), Some(e.to_string())))?;

    let block_info = match state
        .clone()
        .rpc_client
        .block_info(context::current(), block_selector)
        .await
        .map_err(|e| not_found_html_response(State(state.clone()), Some(e.to_string())))?
    {
        Some(info) => Ok(info),
        None => Err(not_found_html_response(
            State(state),
            Some("Block does not exist".to_string()),
        )),
    }?;

    let block_info_page = BlockInfoHtmlPage { header, block_info };
    Ok(Html(block_info_page.to_string()))
}
