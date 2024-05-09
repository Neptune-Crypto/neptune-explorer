use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Json;
use neptune_core::prelude::twenty_first::math::digest::Digest;
use std::sync::Arc;
use tarpc::context;

use crate::{
    http_util::{not_found_err, rpc_err},
    model::{app_state::AppState, path_block_selector::PathBlockSelector},
};

#[axum::debug_handler]
pub async fn block_digest(
    Path(path_block_selector): Path<PathBlockSelector>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Digest>, impl IntoResponse> {
    block_digest_with_value_worker(state, path_block_selector, "").await
}

#[axum::debug_handler]
pub async fn block_digest_with_value(
    Path((path_block_selector, value)): Path<(PathBlockSelector, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Digest>, impl IntoResponse> {
    block_digest_with_value_worker(state, path_block_selector, &value).await
}

async fn block_digest_with_value_worker(
    state: Arc<AppState>,
    path_block_selector: PathBlockSelector,
    value: &str,
) -> Result<Json<Digest>, impl IntoResponse> {
    let block_selector = path_block_selector.as_block_selector(value)?;

    match state
        .rpc_client
        .block_digest(context::current(), block_selector)
        .await
        .map_err(rpc_err)?
    {
        Some(digest) => Ok(Json(digest)),
        None => Err(not_found_err()),
    }
}
