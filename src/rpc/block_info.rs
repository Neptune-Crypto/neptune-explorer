use crate::http_util::not_found_err;
use crate::http_util::rpc_err;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;
use crate::model::block_selector_extended::BlockSelectorExtended;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Json;
use axum::response::Response;
use neptune_cash::models::blockchain::block::block_info::BlockInfo;
use std::sync::Arc;
use tarpc::context;

#[axum::debug_handler]
pub async fn block_info(
    Path(selector): Path<BlockSelectorExtended>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<BlockInfo>, Response> {
    let s = state.load();
    let block_info = s
        .rpc_client
        .block_info(context::current(), s.token(), selector.into())
        .await
        .map_err(rpc_err)?
        .map_err(rpc_method_err)?
        .ok_or_else(not_found_err)?;

    Ok(Json(block_info))
}
