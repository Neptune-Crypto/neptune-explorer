use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Json;
use neptune_core::prelude::twenty_first::math::digest::Digest;
use std::sync::Arc;
use tarpc::context;

use crate::{
    http_util::{not_found_err, rpc_err},
    model::app_state::AppState,
};

#[axum::debug_handler]
pub async fn utxo_digest(
    Path(index): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Digest>, impl IntoResponse> {
    match state
        .read()
        .await
        .rpc_client
        .utxo_digest(context::current(), index)
        .await
        .map_err(rpc_err)?
    {
        Some(digest) => Ok(Json(digest)),
        None => Err(not_found_err()),
    }
}
