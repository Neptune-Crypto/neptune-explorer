use crate::html::page::not_found::not_found_html_response;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;
use crate::model::app_state::AppStateInner;
use axum::extract::State;
use axum::response::Html;
use axum::response::Response;
use html_escaper::Escape;
use neptune_cash::models::blockchain::block::block_height::BlockHeight;
use std::sync::Arc;
use tarpc::context;

#[axum::debug_handler]
pub async fn root(State(state_rw): State<Arc<AppState>>) -> Result<Html<String>, Response> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/root.html")]
    pub struct RootHtmlPage<'a> {
        tip_height: BlockHeight,
        state: &'a AppStateInner,
    }

    let state = &state_rw.load();

    let tip_height = state
        .rpc_client
        .block_height(context::current(), state.token())
        .await
        .map_err(|e| not_found_html_response(state, Some(e.to_string())))?
        .map_err(rpc_method_err)?;

    let root_page = RootHtmlPage { tip_height, state };
    Ok(Html(root_page.to_string()))
}
