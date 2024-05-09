use axum::extract::Path;
use axum::extract::State;
use axum::response::Json;
use axum::response::Response;
use neptune_core::rpc_server::BlockInfo;
use std::sync::Arc;
use tarpc::context;

use crate::{
    http_util::{not_found_err, rpc_err},
    model::{app_state::AppState, path_block_selector::PathBlockSelector},
};

#[axum::debug_handler]
pub async fn block_info(
    Path(path_block_selector): Path<PathBlockSelector>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<BlockInfo>, Response> {
    let block_info = block_info_with_value_worker(state, path_block_selector, "").await?;
    Ok(Json(block_info))
}

#[axum::debug_handler]
pub async fn block_info_with_value(
    Path((path_block_selector, value)): Path<(PathBlockSelector, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<BlockInfo>, Response> {
    let block_info = block_info_with_value_worker(state, path_block_selector, &value).await?;
    Ok(Json(block_info))
}

pub(crate) async fn block_info_with_value_worker(
    state: Arc<AppState>,
    path_block_selector: PathBlockSelector,
    value: &str,
) -> Result<BlockInfo, Response> {
    let block_selector = path_block_selector.as_block_selector(value)?;

    match state
        .rpc_client
        .block_info(context::current(), block_selector)
        .await
        .map_err(rpc_err)?
    {
        Some(info) => Ok(info),
        None => Err(not_found_err()),
    }
}
