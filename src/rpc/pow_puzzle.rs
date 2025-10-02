use std::sync::Arc;

use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Json;
use neptune_cash::application::rpc::server::error::RpcError;
use neptune_cash::application::rpc::server::proof_of_work_puzzle::ProofOfWorkPuzzle;
use neptune_cash::state::wallet::address::generation_address::GenerationReceivingAddress;
use tarpc::context;

use crate::http_util::not_found_err;
use crate::http_util::rpc_err;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;

#[axum::debug_handler]
pub async fn pow_puzzle(
    Path(address): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProofOfWorkPuzzle>, impl IntoResponse> {
    let s = state.load();
    let Ok(receiving_address) = GenerationReceivingAddress::from_bech32m(&address, s.network)
    else {
        return Err(rpc_method_err(RpcError::Failed(address)));
    };
    match s
        .rpc_client
        .pow_puzzle_external_key(context::current(), s.token(), receiving_address.into())
        .await
        .map_err(rpc_err)?
        .map_err(rpc_method_err)?
    {
        Some(pow_puzzle) => Ok(Json(pow_puzzle)),
        None => Err(not_found_err()),
    }
}
