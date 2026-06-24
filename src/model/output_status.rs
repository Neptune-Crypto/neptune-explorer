//! Tracking the status of a transaction output (addition record).
//!
//! Given an [`AdditionRecord`] (the commitment to a transaction output), an
//! exchange such as safetrade wants to know whether that output is:
//!
//!   * [`OutputStatus::NotKnown`]  – not in the mempool and not in any canonical
//!     block,
//!   * [`OutputStatus::InMempool`] – present as an output of a transaction that
//!     is currently in the mempool, or
//!   * [`OutputStatus::Mined`]     – mined into a canonical block (of a known
//!     height, with the block's digest).
//!
//! This module provides:
//!   * [`AdditionRecordHex`], a newtype that parses/serializes an
//!     [`AdditionRecord`] from/to the 80-char hex form the explorer uses
//!     everywhere (mirroring [`crate::model::height_or_digest`]), and
//!   * [`resolve_output_status`], the shared resolver used by both the HTML page
//!     and the JSON endpoint so they cannot disagree.

use std::collections::HashSet;
use std::str::FromStr;

use chrono::DateTime;
use chrono::Utc;
use neptune_cash::api::export::AdditionRecord;
use neptune_cash::api::export::BlockHeight;
use neptune_cash::api::export::Digest;
use neptune_cash::application::rpc::auth;
use neptune_cash::application::rpc::server::error::RpcError;
use neptune_cash::protocol::consensus::block::block_selector::BlockSelector;
use serde::de::Error as _;
use serde::Deserialize;
use serde::Deserializer;
use tarpc::client::RpcError as TransportError;
use tarpc::context;

use crate::model::app_state::AppStateInner;

/// A transaction output (addition record) identified by the hex encoding of its
/// `canonical_commitment` [`Digest`] — the exact 80-char hex string the explorer
/// renders for addition records elsewhere (see `announcement.html`).
///
/// Note: [`AdditionRecord`] has no `FromStr` of its own, and `Digest`'s `FromStr`
/// parses a *different* (comma-separated decimal) form, so user input MUST be
/// parsed with [`Digest::try_from_hex`]. This mirrors
/// [`crate::model::height_or_digest::HeightOrDigest`].
#[derive(Debug, Clone, Copy)]
pub struct AdditionRecordHex(AdditionRecord);

#[derive(Debug, thiserror::Error)]
pub enum AdditionRecordHexParseError {
    #[error("invalid addition-record hex (expected an 80-character hex digest): {0}")]
    InvalidHex(String),
}

impl AdditionRecordHex {
    pub fn addition_record(&self) -> AdditionRecord {
        self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.canonical_commitment.to_hex()
    }
}

impl std::fmt::Display for AdditionRecordHex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for AdditionRecordHex {
    type Err = AdditionRecordHexParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let digest = Digest::try_from_hex(s.trim())
            .map_err(|e| AdditionRecordHexParseError::InvalidHex(e.to_string()))?;
        Ok(Self(AdditionRecord::new(digest)))
    }
}

// note: axum uses serde Deserialize for Path elements.
impl<'de> Deserialize<'de> for AdditionRecordHex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(D::Error::custom)
    }
}

impl From<AdditionRecordHex> for AdditionRecord {
    fn from(v: AdditionRecordHex) -> Self {
        v.0
    }
}

/// The resolved status of a transaction output (addition record).
#[derive(Debug, Clone)]
pub enum OutputStatus {
    /// Not found in the mempool nor in any canonical block.
    NotKnown,
    /// Present as an output of a transaction currently in the mempool.
    InMempool,
    /// Mined into a canonical block.
    ///
    /// `height` is `None` only in the (practically impossible) race where the
    /// containing block was reorged away between the two RPC calls; the digest
    /// is always known.
    Mined {
        block_digest: Digest,
        height: Option<BlockHeight>,
    },
}

/// A resolved [`OutputStatus`] together with the freshness of the mempool data
/// it relied on, so callers can disclose staleness to API consumers.
#[derive(Debug, Clone)]
pub struct ResolvedOutputStatus {
    pub status: OutputStatus,
    /// When the mempool snapshot consulted for this answer was taken. `Some` for
    /// `NotKnown` / `InMempool` (the mempool was checked, and the answer may be
    /// up to [`MEMPOOL_OUTPUTS_TTL_SECS`] seconds stale); `None` for `Mined`,
    /// which is a fresh on-chain answer that does not consult the mempool.
    pub mempool_checked_at: Option<DateTime<Utc>>,
}

/// Error while resolving an output's status, preserving the distinction between
/// a transport-level failure and an RPC-method error so callers can map them to
/// the right HTTP status.
///
/// This distinction matters for safetrade: a transport error must NOT be
/// reported as `NotKnown` (a false "this output does not exist" answer), so the
/// resolver surfaces the error instead of swallowing it.
#[derive(Debug)]
pub enum OutputStatusError {
    /// Could not reach / talk to neptune-core (tarpc transport).
    Transport(TransportError),
    /// neptune-core rejected or failed the RPC call.
    Method(RpcError),
    /// The connected node does not maintain a UTXO index, so the unbounded
    /// `utxo_origin_block` lookup must not be performed. Callers surface this as
    /// HTTP 503. Enforced inside [`resolve_output_status`] (not only in the
    /// handlers) so no current or future caller can bypass the guard.
    IndexUnavailable,
}

/// User-facing reason the tx-output feature is unavailable without a UTXO index.
pub const INDEX_REQUIRED_MESSAGE: &str =
    "Transaction-output tracking requires the connected neptune-core node to run with --utxo-index.";

/// How long a snapshot of the mempool's output addition records is reused before
/// it is refreshed. Keeps a public polling endpoint from doing an O(mempool-size)
/// RPC scan on every request: the scan runs at most once per this interval. It is
/// also reported to API consumers (the `mempool_cache_ttl_seconds` JSON field) so
/// they know an `in_mempool` / `not_known` answer can lag by up to this much.
pub const MEMPOOL_OUTPUTS_TTL_SECS: u64 = 5;

/// Short-TTL cache of every addition record currently in the mempool, used for
/// O(1) "is this output in the mempool?" membership checks. Lives in
/// [`AppStateInner`] behind an async mutex. `refreshed_at` is wall-clock so it
/// can be surfaced to API consumers as the freshness of their answer.
#[derive(Debug, Default)]
pub struct MempoolOutputsCache {
    outputs: HashSet<AdditionRecord>,
    refreshed_at: Option<DateTime<Utc>>,
}

impl MempoolOutputsCache {
    fn is_fresh(&self, now: DateTime<Utc>) -> bool {
        self.refreshed_at.is_some_and(|t| {
            now.signed_duration_since(t).num_seconds() < MEMPOOL_OUTPUTS_TTL_SECS as i64
        })
    }
}

/// Resolve the [`OutputStatus`] of an addition record using only RPC methods
/// that exist at the explorer's pinned neptune-core revision:
/// `utxo_origin_block`, `block_info`, `mempool_tx_ids`, `mempool_tx_kernel`.
///
/// Precedence: **mined wins**. If the output has been mined into a canonical
/// block we report `Mined` even if a (now redundant) copy still lingers in the
/// mempool. Only if it is not in any canonical block do we scan the mempool.
///
/// DoS notes:
/// * `utxo_origin_block(.., None)` is an indexed, constant-time lookup when the
///   node runs with `--utxo-index`, but a full tip→genesis scan otherwise. This
///   function returns [`OutputStatusError::IndexUnavailable`] up front unless the
///   node maintains the index (`AppStateInner::maintains_utxo_index`), so the
///   unbounded `None` below is never reached without the index.
/// * The mempool check would otherwise be `O(mempool size)` RPC round-trips
///   (`mempool_tx_ids` + one `mempool_tx_kernel` per tx), since no single RPC
///   exposes mempool addition records. It is served from a short-TTL snapshot
///   ([`MempoolOutputsCache`]) so that scan runs at most once per TTL regardless
///   of request volume.
pub async fn resolve_output_status(
    state: &AppStateInner,
    addition_record: AdditionRecord,
) -> Result<ResolvedOutputStatus, OutputStatusError> {
    // Guard: the `utxo_origin_block(.., None)` lookup below is an indexed,
    // constant-time call only when the node maintains a UTXO index, and a full
    // tip->genesis scan otherwise. Enforce the invariant HERE so no caller can
    // bypass it (handlers also surface this as 503).
    if !state.maintains_utxo_index {
        return Err(OutputStatusError::IndexUnavailable);
    }

    let token = state.token();

    // 1. MINED (canonical) — utxo_origin_block returns the canonical block
    //    digest that created this output, or None. Reached via Deref to the
    //    underlying RPCClient (AuthenticatedClient adds no wrapper for it).
    let origin_digest = state
        .rpc_client
        .utxo_origin_block(context::current(), token, addition_record, None)
        .await
        .map_err(OutputStatusError::Transport)?
        .map_err(OutputStatusError::Method)?;

    if let Some(block_digest) = origin_digest {
        // Second call: turn the canonical digest into a height for the
        // "mined in canonical block of height n" rendering + link.
        let height = state
            .rpc_client
            .block_info(
                context::current(),
                token,
                BlockSelector::Digest(block_digest),
            )
            .await
            .map_err(OutputStatusError::Transport)?
            .map_err(OutputStatusError::Method)?
            .map(|block_info| block_info.height);

        return Ok(ResolvedOutputStatus {
            status: OutputStatus::Mined {
                block_digest,
                height,
            },
            // The mempool was not consulted: a mined answer is fresh on-chain.
            mempool_checked_at: None,
        });
    }

    // 2. IN MEMPOOL — consult a short-TTL snapshot of every mempool output, so a
    //    public polling endpoint doesn't issue an O(mempool-size) RPC scan on
    //    every request. The scan refreshes the snapshot at most once per TTL;
    //    concurrent callers serialize on the mutex, so only one scan runs.
    let mempool_checked_at = {
        let now = Utc::now();
        let mut cache = state.mempool_outputs_cache.lock().await;
        if !cache.is_fresh(now) {
            cache.outputs = fetch_mempool_outputs(state, token).await?;
            cache.refreshed_at = Some(now);
        }
        let found = cache.outputs.contains(&addition_record);
        let checked_at = cache.refreshed_at;
        if found {
            return Ok(ResolvedOutputStatus {
                status: OutputStatus::InMempool,
                mempool_checked_at: checked_at,
            });
        }
        checked_at
    };

    // 3. NOT KNOWN.
    Ok(ResolvedOutputStatus {
        status: OutputStatus::NotKnown,
        mempool_checked_at,
    })
}

/// Fetch the set of all addition records currently in the mempool. This is the
/// expensive part — `mempool_tx_ids` plus one `mempool_tx_kernel` per tx, since
/// no single RPC exposes mempool addition records. Callers run it behind
/// [`MempoolOutputsCache`] so it executes at most once per TTL.
async fn fetch_mempool_outputs(
    state: &AppStateInner,
    token: auth::Token,
) -> Result<HashSet<AdditionRecord>, OutputStatusError> {
    let tx_ids = state
        .rpc_client
        .mempool_tx_ids(context::current(), token)
        .await
        .map_err(OutputStatusError::Transport)?
        .map_err(OutputStatusError::Method)?;

    let mut outputs = HashSet::new();
    for tx_id in tx_ids {
        // A tx evicted between mempool_tx_ids and mempool_tx_kernel yields None;
        // skip it.
        if let Some(kernel) = state
            .rpc_client
            .mempool_tx_kernel(context::current(), token, tx_id)
            .await
            .map_err(OutputStatusError::Transport)?
            .map_err(OutputStatusError::Method)?
        {
            outputs.extend(kernel.outputs.iter().copied());
        }
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 80 hex chars == Digest::BYTES (40) * 2; all-zero limbs are canonical BFEs.
    fn zero_hex() -> String {
        "0".repeat(80)
    }

    #[test]
    fn addition_record_hex_roundtrips_via_hex() {
        let hex = zero_hex();
        let parsed: AdditionRecordHex = hex.parse().unwrap();
        // Display and to_hex agree and round-trip the input.
        assert_eq!(parsed.to_hex(), hex);
        assert_eq!(parsed.to_string(), hex);
        // Converts to an AdditionRecord whose commitment matches.
        let ar: AdditionRecord = parsed.into();
        assert_eq!(ar.canonical_commitment.to_hex(), hex);
    }

    #[test]
    fn addition_record_hex_trims_surrounding_whitespace() {
        let hex = zero_hex();
        let parsed: AdditionRecordHex = format!("  {hex}\n").parse().unwrap();
        assert_eq!(parsed.to_hex(), hex);
    }

    #[test]
    fn addition_record_hex_rejects_invalid_input() {
        // Not hex at all.
        assert!("not-hex".parse::<AdditionRecordHex>().is_err());
        // Empty.
        assert!("".parse::<AdditionRecordHex>().is_err());
        // Correct charset but wrong length (not 40 bytes).
        assert!("00".parse::<AdditionRecordHex>().is_err());
        // Right length but non-canonical BField limbs (all 0xff > modulus).
        assert!("f".repeat(80).parse::<AdditionRecordHex>().is_err());
    }
}
