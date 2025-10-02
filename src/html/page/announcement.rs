use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::rejection::PathRejection;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Html;
use axum::response::Response;
use html_escaper::Escape;
use html_escaper::Trusted;
use neptune_cash::api::export::BlockHeight;
use neptune_cash::prelude::tasm_lib::prelude::Digest;
use neptune_cash::prelude::triton_vm::prelude::BFieldCodec;
use neptune_cash::prelude::twenty_first::tip5::Tip5;
use neptune_cash::util_types::mutator_set::addition_record::AdditionRecord;
use tarpc::context;

use crate::html::component::header::HeaderHtml;
use crate::html::page::not_found::not_found_html_response;
use crate::http_util::rpc_method_err;
use crate::model::announcement_selector::AnnouncementSelector;
use crate::model::announcement_type::AnnouncementType;
use crate::model::app_state::AppState;
use crate::model::transparent_utxo_tuple::TransparentUtxoTuple;

#[axum::debug_handler]
pub async fn announcement_page(
    maybe_path: Result<Path<AnnouncementSelector>, PathRejection>,
    State(state_rw): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    #[derive(Debug, Clone, boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/announcement.html")]
    pub struct AnnouncementHtmlPage<'a> {
        header: HeaderHtml<'a>,
        index: usize,
        num_announcements: usize,
        block_hash: Digest,
        block_height: BlockHeight,
        announcement_type: AnnouncementType,
        addition_record_indices: HashMap<AdditionRecord, Option<u64>>,
    }

    let state = &state_rw.load();

    let Path(AnnouncementSelector {
        block_selector,
        index,
    }) = maybe_path.map_err(|e| not_found_html_response(state, Some(e.to_string())))?;

    let block_info = state
        .rpc_client
        .block_info(context::current(), state.token(), block_selector)
        .await
        .map_err(|e| not_found_html_response(state, Some(e.to_string())))?
        .map_err(rpc_method_err)?
        .ok_or(not_found_html_response(
            state,
            Some("The requested block does not exist".to_string()),
        ))?;
    let block_hash = block_info.digest;
    let block_height = block_info.height;

    let announcements = state
        .rpc_client
        .announcements_in_block(context::current(), state.token(), block_selector)
        .await
        .map_err(|e| not_found_html_response(state, Some(e.to_string())))?
        .map_err(rpc_method_err)?
        .expect(
            "block guaranteed to exist because we got here; getting its announcements should work",
        );
    let num_announcements = announcements.len();
    let announcement = announcements
        .get(index)
        .ok_or(not_found_html_response(
            state,
            Some("The requested announcement does not exist".to_string()),
        ))?
        .clone();
    let announcement_type = AnnouncementType::parse(announcement);

    let mut addition_record_indices = HashMap::<AdditionRecord, Option<u64>>::new();
    if let AnnouncementType::TransparentTxInfo(tx_info) = announcement_type.clone() {
        let addition_records = tx_info
            .outputs
            .iter()
            .map(|output| output.addition_record())
            .collect::<Vec<_>>();
        addition_record_indices = state
            .rpc_client
            .addition_record_indices_for_block(
                context::current(),
                state.token(),
                block_selector,
                &addition_records,
            )
            .await
            .map_err(|e| not_found_html_response(state, Some(e.to_string())))?
            .map_err(rpc_method_err)?
            .into_iter()
            .collect::<HashMap<_, _>>();

        let mut transparent_utxos_cache = state.transparent_utxos_cache.lock().await;

        for input in &tx_info.inputs {
            let addition_record = input.addition_record();
            if let Some(existing_entry) = transparent_utxos_cache
                .iter_mut()
                .find(|tu| tu.addition_record() == addition_record)
            {
                existing_entry.upgrade_with_transparent_input(input, block_hash);
            } else {
                tracing::info!("Adding transparent UTXO (input side) to cache.");
                transparent_utxos_cache.push(TransparentUtxoTuple::new_from_transparent_input(
                    input, block_hash,
                ));
            }
        }

        for output in &tx_info.outputs {
            let addition_record = output.addition_record();
            if let Some(existing_entry) = transparent_utxos_cache
                .iter_mut()
                .find(|tu| tu.addition_record() == addition_record)
            {
                existing_entry.upgrade_with_transparent_output(block_hash);
            } else {
                tracing::info!("Adding transparent UTXO (output side) to cache.");
                transparent_utxos_cache.push(TransparentUtxoTuple::new_from_transparent_output(
                    output,
                    addition_record_indices
                        .get(&addition_record)
                        .cloned()
                        .unwrap_or(None),
                    block_hash,
                ));
            }
        }
    }

    let header = HeaderHtml { state };

    let utxo_page = AnnouncementHtmlPage {
        index,
        header,
        block_hash,
        block_height,
        num_announcements,
        announcement_type,
        addition_record_indices,
    };
    Ok(Html(utxo_page.to_string()))
}
