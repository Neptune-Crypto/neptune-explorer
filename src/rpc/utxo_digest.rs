use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Json;
use neptune_cash::prelude::twenty_first::tip5::Digest;
use std::sync::Arc;
use tarpc::context;

use crate::http_util::rpc_method_err;
use crate::{
    http_util::{not_found_err, rpc_err},
    model::app_state::AppState,
};

#[axum::debug_handler]
pub async fn utxo_digest(
    Path(index): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Digest>, impl IntoResponse> {
    let s = state.load();
    match s
        .rpc_client
        .utxo_digest(context::current(), s.token(), index)
        .await
        .map_err(rpc_err)?
        .map_err(rpc_method_err)?
    {
        Some(digest) => Ok(Json(digest)),
        None => Err(not_found_err()),
    }
}
