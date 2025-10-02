use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use axum::response::Response;
use tarpc::context;

use crate::http_util::rpc_err;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;
use crate::shared::monetary_supplies;

/// Return the total monetary supply, the sum of the timeloced and liquid
/// supply. Assumes all redemptions on the old chain have successfully been
/// made. Returned unit is nau, Neptune Atomic Units. To convert to number of
/// coins, divide by $4*10^{30}$.
#[axum::debug_handler]
pub async fn total_supply(State(state): State<Arc<AppState>>) -> Result<Json<f64>, Response> {
    let s = state.load();

    let block_height = s
        .rpc_client
        .block_height(context::current(), s.token())
        .await
        .map_err(rpc_err)?
        .map_err(rpc_method_err)?;

    let (_, total_supply) = monetary_supplies(block_height);

    Ok(Json(total_supply.to_nau_f64()))
}
