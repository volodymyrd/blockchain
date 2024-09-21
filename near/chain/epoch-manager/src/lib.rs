use near_cache::SyncLruCache;
use near_primitives::epoch_block_info::BlockInfo;
use near_primitives::epoch_info::EpochInfo;
use near_primitives::epoch_manager::AllEpochConfig;
use near_primitives::hash::CryptoHash;
use near_primitives::stateless_validation::validator_assignment::ChunkValidatorAssignments;
use near_primitives::types::validator_stake::ValidatorStake;
use near_primitives::types::{BlockHeight, EpochId, EpochInfoProvider, NumSeats, ShardId};
use near_primitives::version::ProtocolVersion;
use near_store::Store;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use crate::adapter::EpochManagerAdapter;
pub use crate::reward_calculator::RewardCalculator;
pub use crate::types::{EpochInfoAggregator, RngSeed};

mod adapter;
mod reward_calculator;
pub mod types;

const EPOCH_CACHE_SIZE: usize = if cfg!(feature = "no_cache") { 1 } else { 50 };
const BLOCK_CACHE_SIZE: usize = if cfg!(feature = "no_cache") { 5 } else { 1000 }; // TODO(#5080): fix this
const AGGREGATOR_SAVE_PERIOD: u64 = 1000;

/// In the current architecture, various components have access to the same
/// shared mutable instance of [`EpochManager`]. This handle manages locking
/// required for such access.
///
/// It's up to the caller to ensure that there are no logical races when using
/// `.write` access.
#[derive(Clone)]
pub struct EpochManagerHandle {
    inner: Arc<RwLock<EpochManager>>,
}

impl EpochManagerHandle {
    pub fn write(&self) -> RwLockWriteGuard<EpochManager> {
        self.inner.write().unwrap()
    }

    pub fn read(&self) -> RwLockReadGuard<EpochManager> {
        self.inner.read().unwrap()
    }
}

/// Tracks epoch information across different forks, such as validators.
/// Note: that even after garbage collection, the data about genesis epoch should be in the store.
pub struct EpochManager {
    store: Store,
    /// Current epoch config.
    config: AllEpochConfig,
    reward_calculator: RewardCalculator,
    /// Genesis protocol version. Useful when there are protocol upgrades.
    genesis_protocol_version: ProtocolVersion,
    genesis_num_block_producer_seats: NumSeats,

    /// Cache of epoch information.
    epochs_info: SyncLruCache<EpochId, Arc<EpochInfo>>,
    /// Cache of block information.
    blocks_info: SyncLruCache<CryptoHash, Arc<BlockInfo>>,
    /// Cache of epoch id to epoch start height
    epoch_id_to_start: SyncLruCache<EpochId, BlockHeight>,
    /// Epoch validators ordered by `block_producer_settlement`.
    epoch_validators_ordered: SyncLruCache<EpochId, Arc<[(ValidatorStake, bool)]>>,
    /// Unique validators ordered by `block_producer_settlement`.
    epoch_validators_ordered_unique: SyncLruCache<EpochId, Arc<[(ValidatorStake, bool)]>>,

    /// Unique chunk producers.
    epoch_chunk_producers_unique: SyncLruCache<EpochId, Arc<[ValidatorStake]>>,
    /// Aggregator that keeps statistics about the current epoch.  It’s data are
    /// synced up to the last final block.  The information are updated by
    /// [`Self::update_epoch_info_aggregator_upto_final`] method.  To get
    /// statistics up to a last block use
    /// [`Self::get_epoch_info_aggregator_upto_last`] method.
    epoch_info_aggregator: EpochInfoAggregator,
    /// Largest final height. Monotonically increasing.
    largest_final_height: BlockHeight,
    /// Cache for chunk_validators
    chunk_validators_cache:
        SyncLruCache<(EpochId, ShardId, BlockHeight), Arc<ChunkValidatorAssignments>>,

    /// Counts loop iterations inside of aggregate_epoch_info_upto method.
    /// Used for tests as a bit of white-box testing.
    #[cfg(test)]
    epoch_info_aggregator_loop_counter: std::sync::atomic::AtomicUsize,
}
