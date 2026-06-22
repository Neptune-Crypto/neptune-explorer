use std::sync::Arc;

use axum::extract::Path;
use axum::extract::State;
use axum::response::Json;
use axum::response::Response;
use serde::Serialize;

use crate::http_util::rpc_err;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;
use crate::model::output_status::resolve_output_status;
use crate::model::output_status::AdditionRecordHex;
use crate::model::output_status::OutputStatus;
use crate::model::output_status::OutputStatusError;

/// Machine-readable status of a transaction output (addition record), for
/// programmatic polling by exchanges such as safetrade.
///
/// `status` is one of `"not_known"`, `"in_mempool"`, `"mined"`. The block fields
/// are populated only when `status == "mined"`.
#[derive(Debug, Serialize)]
pub struct OutputStatusResponse {
    /// The 80-char hex addition record that was queried (echoed back).
    pub addition_record: String,
    /// `"not_known"` | `"in_mempool"` | `"mined"`.
    pub status: &'static str,
    /// Canonical block height the output was mined in (`null` unless mined).
    pub block_height: Option<u64>,
    /// Canonical block digest the output was mined in (`null` unless mined).
    pub block_digest: Option<String>,
    /// Convenience explorer URL for the mining block (`null` unless mined).
    pub block_url: Option<String>,
}

/// Route: `GET /rpc/output_status/:addition_record_hex`.
///
/// Shares [`resolve_output_status`] with the HTML page so the two surfaces
/// always agree. A transport error returns 500 (NOT `not_known`) so an exchange
/// never mistakes an outage for "this output does not exist".
#[axum::debug_handler]
pub async fn output_status(
    Path(addition_record_hex): Path<AdditionRecordHex>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<OutputStatusResponse>, Response> {
    let s = state.load();

    let status = resolve_output_status(&s, addition_record_hex.addition_record())
        .await
        .map_err(|e| match e {
            OutputStatusError::Transport(t) => rpc_err(t),
            OutputStatusError::Method(m) => rpc_method_err(m),
        })?;

    let response = match status {
        OutputStatus::NotKnown => OutputStatusResponse {
            addition_record: addition_record_hex.to_hex(),
            status: "not_known",
            block_height: None,
            block_digest: None,
            block_url: None,
        },
        OutputStatus::InMempool => OutputStatusResponse {
            addition_record: addition_record_hex.to_hex(),
            status: "in_mempool",
            block_height: None,
            block_digest: None,
            block_url: None,
        },
        OutputStatus::Mined {
            block_digest,
            height,
        } => {
            let digest_hex = block_digest.to_hex();
            OutputStatusResponse {
                addition_record: addition_record_hex.to_hex(),
                status: "mined",
                block_height: height.map(u64::from),
                block_url: Some(format!("/block/digest/{digest_hex}")),
                block_digest: Some(digest_hex),
            }
        }
    };

    Ok(Json(response))
}
