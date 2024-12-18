use crate::http_util::not_found_err;
use crate::http_util::rpc_err;
use crate::model::app_state::AppState;
use crate::model::block_selector_extended::BlockSelectorExtended;
use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Json;
use neptune_cash::prelude::twenty_first::math::digest::Digest;
use std::sync::Arc;
use tarpc::context;

#[axum::debug_handler]
pub async fn block_digest(
    Path(selector): Path<BlockSelectorExtended>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Digest>, impl IntoResponse> {
    match state
        .load()
        .rpc_client
        .block_digest(context::current(), selector.into())
        .await
        .map_err(rpc_err)?
    {
        Some(digest) => Ok(Json(digest)),
        None => Err(not_found_err()),
    }
}
