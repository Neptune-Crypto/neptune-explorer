use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use axum::response::Response;
use neptune_cash::api::export::BlockHeight;
use neptune_cash::protocol::consensus::block::block_height::BLOCKS_PER_GENERATION;
use neptune_cash::protocol::consensus::block::block_height::NUM_BLOCKS_SKIPPED_BECAUSE_REBOOT;
use neptune_cash::protocol::consensus::block::Block;
use neptune_cash::protocol::consensus::block::PREMINE_MAX_SIZE;
use tarpc::context;

use crate::http_util::rpc_err;
use crate::http_util::rpc_method_err;
use crate::model::app_state::AppState;

/// Return the number of coins that are liquid, assuming all redemptions on the
/// old chain have successfully been made.
#[axum::debug_handler]
pub async fn circulating_supply(State(state): State<Arc<AppState>>) -> Result<Json<f64>, Response> {
    let s = state.load();

    // TODO: Remove this local declaration once version of neptune-core with
    // this value public is released.
    let generation_0_subsidy = Block::block_subsidy(BlockHeight::genesis().next());
    let block_height: u64 = s
        .rpc_client
        .block_height(context::current(), s.token())
        .await
        .map_err(rpc_err)?
        .map_err(rpc_method_err)?
        .into();
    let effective_block_height = block_height + NUM_BLOCKS_SKIPPED_BECAUSE_REBOOT;
    let (num_generations, num_blocks_in_generation): (u64, u32) = (
        effective_block_height / BLOCKS_PER_GENERATION,
        (effective_block_height % BLOCKS_PER_GENERATION)
            .try_into()
            .expect("There are fewer than u32::MAX blocks per generation"),
    );

    let mut liquid_supply = PREMINE_MAX_SIZE;
    let mut liquid_subsidy = generation_0_subsidy.half();
    let blocks_per_generation: u32 = BLOCKS_PER_GENERATION
        .try_into()
        .expect("There are fewer than u32::MAX blocks per generation");
    for _ in 0..num_generations {
        liquid_supply += liquid_subsidy.scalar_mul(blocks_per_generation);
        liquid_subsidy = liquid_subsidy.half();
    }

    liquid_supply += liquid_subsidy.scalar_mul(num_blocks_in_generation);

    // How much of timelocked miner rewards have been unlocked? Assume that the
    // timelock is exactly one generation long. In reality the timelock is
    // is defined in relation to timestamp and not block heights, so this is
    // only a (pretty good) approximation.

    let mut released_subsidy = generation_0_subsidy.half();
    for _ in 1..num_generations {
        liquid_supply += released_subsidy.scalar_mul(blocks_per_generation);
        released_subsidy = released_subsidy.half();
    }

    if num_generations > 0 {
        liquid_supply += released_subsidy.scalar_mul(num_blocks_in_generation);
    }

    Ok(Json(liquid_supply.to_nau_f64()))
}
