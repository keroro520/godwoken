use crate::core::Timepoint;
use crate::packed::RollupConfig;
use crate::prelude::*;

const BLOCK_INTERVAL_IN_MILLISECONDS: u64 = 36000;

impl RollupConfig {
    pub fn finality_as_duration(&self) -> u64 {
        finality_as_duration(self.finality_blocks().unpack())
    }

    pub fn finality_as_blocks(&self) -> u64 {
        finality_as_blocks(self.finality_blocks().unpack())
    }
}

pub fn finality_as_blocks(finality: u64) -> u64 {
    match Timepoint::from_full_value(finality) {
        Timepoint::BlockNumber(block_number) => block_number,
        Timepoint::Timestamp(timestamp) => timestamp / BLOCK_INTERVAL_IN_MILLISECONDS,
    }
}

pub fn finality_as_duration(finality: u64) -> u64 {
    match Timepoint::from_full_value(finality) {
        Timepoint::BlockNumber(block_number) => {
            block_number.saturating_mul(BLOCK_INTERVAL_IN_MILLISECONDS)
        }
        Timepoint::Timestamp(timestamp) => timestamp,
    }
}
