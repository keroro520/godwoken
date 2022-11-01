use crate::core::Timepoint;
use crate::finality::finality_as_blocks;
use crate::packed::{GlobalState, RollupConfig};
use crate::prelude::Unpack;
use sparse_merkle_tree::H256;

#[derive(Clone, Default)]
pub struct RollupContext {
    pub rollup_script_hash: H256,
    pub rollup_config: RollupConfig,

    // TODO @keroro520 Ideally this field should be without Option, and a default value 0 indicates
    // that the change is activated from genesis. While considering there are still many test cases
    // that use // block-number-based finality rules, I disable this change by default. It should
    // be refactor as soon as possible.
    //
    /// Fork blocks configurations, #{change => activation_height}
    pub timestamp_based_finality_fork_block: Option<u64>,
}

impl RollupContext {
    /// Returns the version of global state for `block_number`.
    pub fn determine_global_state_version(&self, block_number: u64) -> u8 {
        match &self.timestamp_based_finality_fork_block {
            None => {
                // timestamp_based_finality is disabled
                1
            }
            Some(fork_block) if *fork_block < block_number => {
                // not activated yet
                1
            }
            Some(_) => {
                // activated
                2
            }
        }
    }
}

// TODO @keroro520 move to a standalone file

/// Even after Godwoken has upgraded to v2, there are still some entities with number-based timepoint.
/// This structure includes number-based and timestamp-based finalized timepoints, so it can be used for different cases.
#[derive(Clone, Debug, Default)]
pub struct CompatibleFinalizedTimepoint {
    finalized_timestamp: Option<u64>,
    finalized_block_number: u64,
}

impl CompatibleFinalizedTimepoint {
    pub fn from_global_state(global_state: &GlobalState, rollup_config_finality: u64) -> Self {
        match Timepoint::from_full_value(global_state.last_finalized_block_number().unpack()) {
            Timepoint::BlockNumber(finalized_block_number) => Self {
                finalized_timestamp: None,
                finalized_block_number,
            },
            Timepoint::Timestamp(finalized_timestamp) => {
                let global_block_number = global_state.block().count().unpack().saturating_sub(1);
                let finality_as_blocks = finality_as_blocks(rollup_config_finality);
                Self {
                    finalized_timestamp: Some(finalized_timestamp),
                    finalized_block_number: global_block_number.saturating_sub(finality_as_blocks),
                }
            }
        }
    }

    // Test use only!
    pub fn from_block_number(block_number: u64, rollup_config_finality: u64) -> Self {
        let finality_as_blocks = finality_as_blocks(rollup_config_finality);
        Self {
            finalized_timestamp: None,
            finalized_block_number: block_number.saturating_sub(finality_as_blocks),
        }
    }

    /// Returns true if `timepoint` is finalized.
    pub fn is_finalized(&self, timepoint: &Timepoint) -> bool {
        match timepoint {
            Timepoint::BlockNumber(block_number) => *block_number <= self.finalized_block_number,
            Timepoint::Timestamp(timestamp) => {
                match self.finalized_timestamp {
                    Some(finalized_timestamp) => *timestamp <= finalized_timestamp,
                    None => {
                        // it should never happen
                        false
                    }
                }
            }
        }
    }
}
