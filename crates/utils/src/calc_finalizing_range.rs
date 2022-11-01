use anyhow::Result;
use gw_store::{traits::chain_store::ChainStore, transaction::StoreTransaction};
use gw_types::offchain::CompatibleFinalizedTimepoint;
use gw_types::{
    core::Timepoint,
    packed::{FinalizingRange, L2Block, RollupConfig},
    prelude::*,
};

/// Calculates FinalizingRange for the block.
///
/// FinalizingRange represents a range of block numbers, in the form of (from_block_number, to_block_number]:
///   - when from_block_number < to_block_number, blocks {from_block_number+1, from_block_number+2, ..., to_block_number} are finalizing;
///   - when from_block_number = to_block_number, no any blocks are finalizing
pub fn calc_finalizing_range(
    rollup_config: &RollupConfig,
    db: &StoreTransaction,
    current_block: &L2Block,
) -> Result<FinalizingRange> {
    let current_block_hash = current_block.hash();
    let current_block_number: u64 = current_block.raw().number().unpack();
    let current_global_state = db
        .get_block_post_global_state(&current_block_hash.into())?
        .expect("get current block global state");
    let compatible_finalized_timepoint = CompatibleFinalizedTimepoint::from_global_state(
        &current_global_state,
        rollup_config.finality_blocks().unpack(),
    );

    if current_block_number == 0 {
        return Ok(Default::default());
    }

    let parent_hash = current_block.raw().parent_block_hash();
    let parent_finalizing_range = db
        .get_block_finalizing_range(&parent_hash.unpack())
        .expect("get parent finalizing range");

    let from_number: u64 = parent_finalizing_range.to_block_number().unpack();
    let mut to_number = from_number;
    while to_number + 1 < current_block_number {
        let older_block_number = to_number + 1;
        let older_block_hash = db
            .get_block_hash_by_number(older_block_number)?
            .expect("get finalizing block hash");
        let older_global_state = db
            .get_block_post_global_state(&older_block_hash)?
            .expect("get finalizing block global state");
        let older_global_state_version: u8 = older_global_state.version().into();
        let older_timepoint = if older_global_state_version < 2 {
            Timepoint::from_block_number(older_block_number)
        } else {
            Timepoint::from_timestamp(older_global_state.tip_block_timestamp().unpack())
        };
        if compatible_finalized_timepoint.is_finalized(&older_timepoint) {
            to_number = to_number + 1;
        } else {
            break;
        }
    }

    Ok(FinalizingRange::new_builder()
        .from_block_number(from_number.pack())
        .to_block_number(to_number.pack())
        .build())
}
