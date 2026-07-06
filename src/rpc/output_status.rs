use std::sync::Arc;

use axum::extract::Path;
use axum::extract::State;
use axum::response::Json;
use axum::response::Response;
use serde::Serialize;

use crate::http_util::rpc_err;
use crate::http_util::rpc_method_err;
use crate::http_util::service_unavailable_err;
use crate::model::app_state::AppState;
use crate::model::output_status::resolve_output_status;
use crate::model::output_status::AdditionRecordHex;
use crate::model::output_status::MempoolOutputInfo;
use crate::model::output_status::OutputStatus;
use crate::model::output_status::OutputStatusError;
use crate::model::output_status::INDEX_REQUIRED_MESSAGE;
use crate::model::output_status::MEMPOOL_OUTPUTS_TTL_SECS;

/// Machine-readable status of a transaction output (addition record), for
/// programmatic polling by exchanges such as safetrade.
///
/// `status` is one of `"not_known"`, `"in_mempool"`, `"mined"`. The block fields
/// are populated only when `status == "mined"`.
///
/// Freshness: a `mined` answer is computed fresh from the node on every request.
/// An `in_mempool` / `not_known` answer is derived from a mempool snapshot cached
/// for `mempool_cache_ttl_seconds`, taken at `mempool_checked_at`, so mempool
/// status can lag by up to the TTL. Primitive-witness-backed transactions are
/// excluded from the public mempool view because they are local-only.
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
    /// Maximum staleness (seconds) of the mempool-derived part of this answer.
    pub mempool_cache_ttl_seconds: u64,
    /// RFC 3339 time the mempool snapshot behind this answer was taken. `null`
    /// for `mined` (the mempool was not consulted).
    pub mempool_checked_at: Option<String>,
    /// Mempool transaction id that created the output (`null` unless in mempool).
    pub mempool_tx_id: Option<String>,
    /// Transaction fee formatted in NPT (`null` unless in mempool).
    pub mempool_fee: Option<String>,
    /// Number of transaction inputs (`null` unless in mempool).
    pub mempool_num_inputs: Option<usize>,
    /// Number of transaction outputs (`null` unless in mempool).
    pub mempool_num_outputs: Option<usize>,
    /// `"proof_collection"` | `"single_proof"` (`null` unless in mempool).
    pub mempool_proof_quality: Option<&'static str>,
    /// One-based position among publishable mempool transactions sorted by fee
    /// density (`null` unless in mempool).
    pub mempool_queue_position: Option<usize>,
    /// Number of publishable transactions in the fee-density queue snapshot
    /// (`null` unless in mempool).
    pub mempool_queue_len: Option<usize>,
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

    // Checked here for a clean early response, and also enforced inside
    // `resolve_output_status` (`IndexUnavailable`) so the guard can't be bypassed.
    if !s.maintains_utxo_index {
        return Err(service_unavailable_err(INDEX_REQUIRED_MESSAGE));
    }

    let resolved = resolve_output_status(&s, addition_record_hex.addition_record())
        .await
        .map_err(|e| match e {
            OutputStatusError::Transport(t) => rpc_err(t),
            OutputStatusError::Method(m) => rpc_method_err(m),
            OutputStatusError::IndexUnavailable => service_unavailable_err(INDEX_REQUIRED_MESSAGE),
        })?;

    let mempool_checked_at = resolved.mempool_checked_at.map(|t| t.to_rfc3339());
    let response = match resolved.status {
        OutputStatus::NotKnown => OutputStatusResponse {
            addition_record: addition_record_hex.to_hex(),
            status: "not_known",
            block_height: None,
            block_digest: None,
            block_url: None,
            mempool_cache_ttl_seconds: MEMPOOL_OUTPUTS_TTL_SECS,
            mempool_checked_at,
            mempool_tx_id: None,
            mempool_fee: None,
            mempool_num_inputs: None,
            mempool_num_outputs: None,
            mempool_proof_quality: None,
            mempool_queue_position: None,
            mempool_queue_len: None,
        },
        OutputStatus::InMempool(info) => {
            let mempool = mempool_response_fields(info);
            OutputStatusResponse {
                addition_record: addition_record_hex.to_hex(),
                status: "in_mempool",
                block_height: None,
                block_digest: None,
                block_url: None,
                mempool_cache_ttl_seconds: MEMPOOL_OUTPUTS_TTL_SECS,
                mempool_checked_at,
                mempool_tx_id: Some(mempool.tx_id),
                mempool_fee: Some(mempool.fee),
                mempool_num_inputs: Some(mempool.num_inputs),
                mempool_num_outputs: Some(mempool.num_outputs),
                mempool_proof_quality: Some(mempool.proof_quality),
                mempool_queue_position: Some(mempool.queue_position),
                mempool_queue_len: Some(mempool.queue_len),
            }
        }
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
                mempool_cache_ttl_seconds: MEMPOOL_OUTPUTS_TTL_SECS,
                mempool_checked_at,
                mempool_tx_id: None,
                mempool_fee: None,
                mempool_num_inputs: None,
                mempool_num_outputs: None,
                mempool_proof_quality: None,
                mempool_queue_position: None,
                mempool_queue_len: None,
            }
        }
    };

    Ok(Json(response))
}

struct MempoolResponseFields {
    tx_id: String,
    fee: String,
    num_inputs: usize,
    num_outputs: usize,
    proof_quality: &'static str,
    queue_position: usize,
    queue_len: usize,
}

fn mempool_response_fields(info: MempoolOutputInfo) -> MempoolResponseFields {
    MempoolResponseFields {
        tx_id: info.transaction_id.to_string(),
        fee: format!("{} NPT", info.fee.display_n_decimals(8)),
        num_inputs: info.num_inputs,
        num_outputs: info.num_outputs,
        proof_quality: proof_quality_key(info.proof_type),
        queue_position: info.queue_position,
        queue_len: info.queue_len,
    }
}

fn proof_quality_key(proof_type: neptune_cash::api::export::TransactionProofType) -> &'static str {
    use neptune_cash::api::export::TransactionProofType;

    match proof_type {
        TransactionProofType::PrimitiveWitness => "primitive_witness",
        TransactionProofType::ProofCollection => "proof_collection",
        TransactionProofType::SingleProof => "single_proof",
    }
}
