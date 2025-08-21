use crate::http_util::rpc_err;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;
use axum::extract::State;
use axum::response::Json;
use axum::response::Response;
use neptune_cash::models::blockchain::block::block_header::BlockPow;
use neptune_cash::prelude::twenty_first::tip5::Digest;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use tarpc::context;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowSolution {
    pow: BlockPow,
    proposal_id: Digest,
}

#[axum::debug_handler]
pub async fn provide_pow_solution(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PowSolution>,
) -> Result<Json<bool>, Response> {
    let s = state.load();
    let result = s
        .rpc_client
        .provide_pow_solution(
            context::current(),
            s.token(),
            payload.pow,
            payload.proposal_id,
        )
        .await
        .map_err(rpc_err)?
        .map_err(rpc_method_err)?;

    Ok(Json(result))
}
