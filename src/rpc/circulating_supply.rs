use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use axum::response::Response;
use tarpc::context;

use crate::http_util::rpc_err;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;
use crate::shared::monetary_supplies;

/// Return the current monetary amount that is liquid, assuming all redemptions
/// on the old chain have successfully been made. Returned unit is in number of
/// coins. To convert to number of nau, multiply by $4*10^{30}$/
#[axum::debug_handler]
pub async fn circulating_supply(State(state): State<Arc<AppState>>) -> Result<Json<i32>, Response> {
    let s = state.load();

    let block_height = s
        .rpc_client
        .block_height(context::current(), s.token())
        .await
        .map_err(rpc_err)?
        .map_err(rpc_method_err)?;

    let (liquid_supply, _) = monetary_supplies(block_height);

    Ok(Json(liquid_supply.ceil_num_whole_coins()))
}
