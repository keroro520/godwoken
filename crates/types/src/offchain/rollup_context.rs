use crate::packed::RollupConfig;
use sparse_merkle_tree::H256;

#[derive(Clone, Default)]
pub struct RollupContext {
    pub rollup_script_hash: H256,
    pub rollup_config: RollupConfig,

    // Fork blocks configurations
    //
    /// `None` disables this fork change, by default;
    /// `Some<u64>` indicates the activation height for the fork change.
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
                // timestamp_based_finality has not been activated yet
                1
            }
            Some(_) => {
                // timestamp_based_finality has been activated
                2
            }
        }
    }
}
