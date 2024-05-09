use crate::html::component::header::HeaderHtml;
use crate::model::app_state::AppState;
use crate::model::path_block_selector::PathBlockSelector;
use crate::rpc::block_info::block_info_with_value_worker;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Html;
use axum::response::Response;
use html_escaper::Escape;
use html_escaper::Trusted;
use neptune_core::rpc_server::BlockInfo;
use std::sync::Arc;

pub async fn block_page(
    Path(path_block_selector): Path<PathBlockSelector>,
    state: State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    let value_path: Path<(PathBlockSelector, String)> = Path((path_block_selector, "".to_string()));
    block_page_with_value(value_path, state).await
}

#[axum::debug_handler]
pub async fn block_page_with_value(
    Path((path_block_selector, value)): Path<(PathBlockSelector, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/block_info.html")]
    pub struct BlockInfoHtmlPage {
        header: HeaderHtml,
        block_info: BlockInfo,
    }

    let header = HeaderHtml {
        site_name: "Neptune Explorer".to_string(),
        state: state.clone(),
    };

    let block_info = block_info_with_value_worker(state, path_block_selector, &value).await?;
    let block_info_page = BlockInfoHtmlPage { header, block_info };
    Ok(Html(block_info_page.to_string()))
}
