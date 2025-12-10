use neptune_cash::api::export::BlockHeight;
use neptune_cash::api::export::NativeCurrencyAmount;
use neptune_cash::protocol::consensus::block::block_height::BLOCKS_PER_GENERATION;
use neptune_cash::protocol::consensus::block::block_height::NUM_BLOCKS_SKIPPED_BECAUSE_REBOOT;
use neptune_cash::protocol::consensus::block::Block;
use neptune_cash::protocol::consensus::block::PREMINE_MAX_SIZE;

/// Return the pair (liquid supply, total supply)
///
/// Assumes all redemption claims have been rewarded.
pub(crate) fn monetary_supplies(
    block_height: BlockHeight,
) -> (NativeCurrencyAmount, NativeCurrencyAmount) {
    let block_height: u64 = block_height.into();
    let generation_0_subsidy = Block::block_subsidy(BlockHeight::genesis().next());
    let effective_block_height = block_height + NUM_BLOCKS_SKIPPED_BECAUSE_REBOOT;
    let (num_generations, num_blocks_in_curr_gen): (u64, u32) = (
        effective_block_height / BLOCKS_PER_GENERATION,
        (effective_block_height % BLOCKS_PER_GENERATION)
            .try_into()
            .expect("There are fewer than u32::MAX blocks per generation"),
    );

    let mut liquid_supply = PREMINE_MAX_SIZE;
    let mut liquid_subsidy = generation_0_subsidy.half();
    let mut total_supply = PREMINE_MAX_SIZE;
    let blocks_per_generation: u32 = BLOCKS_PER_GENERATION
        .try_into()
        .expect("There are fewer than u32::MAX blocks per generation");
    for _ in 0..num_generations {
        liquid_supply += liquid_subsidy.scalar_mul(blocks_per_generation);
        total_supply += liquid_subsidy.scalar_mul(2);
        liquid_subsidy = liquid_subsidy.half();
    }

    let liquid_supply_current_generation = liquid_subsidy.scalar_mul(num_blocks_in_curr_gen);
    liquid_supply += liquid_supply_current_generation;
    total_supply += liquid_supply_current_generation.scalar_mul(2);

    // List of all burns is tracked on:
    // https://talk.neptune.cash/t/list-of-known-burns/187
    // https://web.archive.org/web/20251210115730/https://talk.neptune.cash/t/list-of-known-burns/187
    let total_burn = NativeCurrencyAmount::coins_from_str("1526642.2").unwrap();
    liquid_supply = liquid_supply - total_burn;
    total_supply = total_supply - total_burn;

    // How much of timelocked miner rewards have been unlocked? Assume that the
    // timelock is exactly one generation long. In reality the timelock is
    // is defined in relation to timestamp and not block heights, so this is
    // only a (good) approximation.

    let mut released_subsidy = generation_0_subsidy.half();
    for _ in 1..num_generations {
        liquid_supply += released_subsidy.scalar_mul(blocks_per_generation);
        released_subsidy = released_subsidy.half();
    }

    if num_generations > 0 {
        liquid_supply += released_subsidy.scalar_mul(num_blocks_in_curr_gen);
    }

    // If you want correct results for anything but main net, and claims are
    // only being refunded on main net, the size of the claims pool must be
    // subtracted here, if that the network is not main net.

    (liquid_supply, total_supply)
}
