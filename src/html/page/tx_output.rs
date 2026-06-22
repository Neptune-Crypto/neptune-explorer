use std::sync::Arc;

use axum::extract::rejection::PathRejection;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Html;
use axum::response::Response;
use boilerplate::Trusted;
use thousands::Separable;

use crate::html::component::header::HeaderHtml;
use crate::html::page::not_found::not_found_html_response;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;
use crate::model::output_status::resolve_output_status;
use crate::model::output_status::AdditionRecordHex;
use crate::model::output_status::OutputStatus;
use crate::model::output_status::OutputStatusError;

/// HTML page reporting the status of a transaction output (addition record):
/// not known, in mempool, or mined into a canonical block (with a link to it).
///
/// Route: `/output/:addition_record_hex` (80-char hex of the canonical
/// commitment). The status logic is shared with the JSON endpoint via
/// [`resolve_output_status`] so the two surfaces can never disagree.
#[axum::debug_handler]
pub async fn tx_output_page(
    user_input_maybe: Result<Path<AdditionRecordHex>, PathRejection>,
    State(state_rw): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/tx_output.html")]
    pub struct TxOutputHtmlPage<'a> {
        header: HeaderHtml<'a>,
        addition_record_hex: String,
        in_mempool: bool,
        /// `Some` iff mined into a canonical block.
        mined_block_digest_hex: Option<String>,
        /// Comma-formatted height of the mining block; `None` if mined but the
        /// height is momentarily unavailable (reorg race).
        mined_height: Option<String>,
    }

    let state = &state_rw.load();

    let Path(addition_record_hex) =
        user_input_maybe.map_err(|e| not_found_html_response(state, Some(e.to_string())))?;

    // Note: an RPC error is surfaced as an error page here (NOT reported as
    // "not known"), so an exchange never sees a false negative from a transport
    // hiccup.
    let status = resolve_output_status(state, addition_record_hex.addition_record())
        .await
        .map_err(|e| match e {
            OutputStatusError::Transport(t) => not_found_html_response(state, Some(t.to_string())),
            OutputStatusError::Method(m) => rpc_method_err(m),
        })?;

    let (in_mempool, mined_block_digest_hex, mined_height) = match status {
        OutputStatus::NotKnown => (false, None, None),
        OutputStatus::InMempool => (true, None, None),
        OutputStatus::Mined {
            block_digest,
            height,
        } => (
            false,
            Some(block_digest.to_hex()),
            height.map(|h| u64::from(h).separate_with_commas()),
        ),
    };

    let header = HeaderHtml { state };

    let page = TxOutputHtmlPage {
        header,
        addition_record_hex: addition_record_hex.to_hex(),
        in_mempool,
        mined_block_digest_hex,
        mined_height,
    };
    Ok(Html(page.to_string()))
}
