use std::collections::BTreeMap;

use crate::errors::RuntimeError;
use borsh::{BorshDeserialize, BorshSerialize};
use near_parameters::config::CongestionControlConfig;
use near_primitives_core::types::{Gas, ShardId};
use near_schema_checker_lib::ProtocolSchema;
use ordered_float::NotNan;

/// This class combines the congestion control config, congestion info and
/// missed chunks count. It contains the main congestion control logic and
/// exposes methods that can be used for congestion control.
///
/// Use this struct to make congestion control decisions, by looking at the
/// congestion info of a previous chunk produced on a remote shard. For building
/// up a congestion info for the local shard, this struct should not be
/// necessary. Use `CongestionInfo` directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CongestionControl {
    config: CongestionControlConfig,
    /// Finalized congestion info of a previous chunk.
    info: CongestionInfo,
    /// How many block heights had no chunk since the last successful chunk on
    /// the respective shard.
    missed_chunks_count: u64,
}

impl CongestionControl {
    pub fn new(
        config: CongestionControlConfig,
        info: CongestionInfo,
        missed_chunks_count: u64,
    ) -> Self {
        Self {
            config,
            info,
            missed_chunks_count,
        }
    }

    pub fn config(&self) -> &CongestionControlConfig {
        &self.config
    }

    pub fn congestion_info(&self) -> &CongestionInfo {
        &self.info
    }

    pub fn congestion_level(&self) -> f64 {
        let incoming_congestion = self.incoming_congestion();
        let outgoing_congestion = self.outgoing_congestion();
        let memory_congestion = self.memory_congestion();
        let missed_chunks_congestion = self.missed_chunks_congestion();

        incoming_congestion
            .max(outgoing_congestion)
            .max(memory_congestion)
            .max(missed_chunks_congestion)
    }

    fn incoming_congestion(&self) -> f64 {
        self.info.incoming_congestion(&self.config)
    }

    fn outgoing_congestion(&self) -> f64 {
        self.info.outgoing_congestion(&self.config)
    }

    fn memory_congestion(&self) -> f64 {
        self.info.memory_congestion(&self.config)
    }

    fn missed_chunks_congestion(&self) -> f64 {
        if self.missed_chunks_count <= 1 {
            return 0.0;
        }

        clamped_f64_fraction(
            self.missed_chunks_count as u128,
            self.config.max_congestion_missed_chunks,
        )
    }

    /// How much gas another shard can send to us in the next block.
    pub fn outgoing_gas_limit(&self, sender_shard: ShardId) -> Gas {
        let congestion = self.congestion_level();

        // note: using float equality is okay here because
        // `clamped_f64_fraction` clamps to exactly 1.0.
        if congestion == 1.0 {
            // Red traffic light: reduce to minimum speed
            if sender_shard == self.info.allowed_shard() as u64 {
                self.config.allowed_shard_outgoing_gas
            } else {
                0
            }
        } else {
            mix(
                self.config.max_outgoing_gas,
                self.config.min_outgoing_gas,
                congestion,
            )
        }
    }

    /// How much data another shard can send to us in the next block.
    pub fn outgoing_size_limit(&self, sender_shard: ShardId) -> Gas {
        if sender_shard == self.info.allowed_shard() as u64 {
            // The allowed shard is allowed to send more data to us.
            self.config.outgoing_receipts_big_size_limit
        } else {
            // Other shards have a low standard limit.
            self.config.outgoing_receipts_usual_size_limit
        }
    }

    /// How much gas we accept for executing new transactions going to any
    /// uncongested shards.
    pub fn process_tx_limit(&self) -> Gas {
        mix(
            self.config.max_tx_gas,
            self.config.min_tx_gas,
            self.incoming_congestion(),
        )
    }

    /// Whether we can accept new transaction with the receiver set to this shard.
    ///
    /// If the shard doesn't accept new transaction, provide the reason for
    /// extra debugging information.
    pub fn shard_accepts_transactions(&self) -> ShardAcceptsTransactions {
        let incoming_congestion = self.incoming_congestion();
        let outgoing_congestion = self.outgoing_congestion();
        let memory_congestion = self.memory_congestion();
        let missed_chunks_congestion = self.missed_chunks_congestion();

        let congestion_level = incoming_congestion
            .max(outgoing_congestion)
            .max(memory_congestion)
            .max(missed_chunks_congestion);

        // Convert to NotNan here, if not possible, the max above is already meaningless.
        let congestion_level =
            NotNan::new(congestion_level).unwrap_or_else(|_| NotNan::new(1.0).unwrap());
        if *congestion_level < self.config.reject_tx_congestion_threshold {
            return ShardAcceptsTransactions::Yes;
        }

        let reason = if missed_chunks_congestion >= *congestion_level {
            RejectTransactionReason::MissedChunks {
                missed_chunks: self.missed_chunks_count,
            }
        } else if incoming_congestion >= *congestion_level {
            RejectTransactionReason::IncomingCongestion { congestion_level }
        } else if outgoing_congestion >= *congestion_level {
            RejectTransactionReason::OutgoingCongestion { congestion_level }
        } else {
            RejectTransactionReason::MemoryCongestion { congestion_level }
        };
        ShardAcceptsTransactions::No(reason)
    }
}

/// Result of [`CongestionControl::shard_accepts_transactions`].
pub enum ShardAcceptsTransactions {
    Yes,
    No(RejectTransactionReason),
}

/// Detailed information for why a shard rejects new transactions.
pub enum RejectTransactionReason {
    IncomingCongestion { congestion_level: NotNan<f64> },
    OutgoingCongestion { congestion_level: NotNan<f64> },
    MemoryCongestion { congestion_level: NotNan<f64> },
    MissedChunks { missed_chunks: u64 },
}

/// Stores the congestion level of a shard.
///
/// The CongestionInfo is a part of the ChunkHeader. It is versioned and each
/// version should not be changed. Rather a new version with the desired changes
/// should be added and used in place of the old one. When adding new versions
/// please also update the default.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    ProtocolSchema,
)]
pub enum CongestionInfo {
    V1(CongestionInfoV1),
}

impl Default for CongestionInfo {
    fn default() -> Self {
        Self::V1(CongestionInfoV1::default())
    }
}

impl CongestionInfo {
    // A helper method to compare the congestion info from the chunk extra of
    // the previous chunk and the header of the current chunk. It returns true
    // if the congestion info was correctly set in the chunk header based on the
    // information from the chunk extra.
    //
    // TODO(congestion_control) validate allowed shard
    pub fn validate_extra_and_header(extra: &CongestionInfo, header: &CongestionInfo) -> bool {
        match (extra, header) {
            (CongestionInfo::V1(extra), CongestionInfo::V1(header)) => {
                extra.delayed_receipts_gas == header.delayed_receipts_gas
                    && extra.buffered_receipts_gas == header.buffered_receipts_gas
                    && extra.receipt_bytes == header.receipt_bytes
                    && extra.allowed_shard == header.allowed_shard
            }
        }
    }

    pub fn delayed_receipts_gas(&self) -> u128 {
        match self {
            CongestionInfo::V1(inner) => inner.delayed_receipts_gas,
        }
    }

    pub fn buffered_receipts_gas(&self) -> u128 {
        match self {
            CongestionInfo::V1(inner) => inner.buffered_receipts_gas,
        }
    }

    pub fn receipt_bytes(&self) -> u64 {
        match self {
            CongestionInfo::V1(inner) => inner.receipt_bytes,
        }
    }

    pub fn allowed_shard(&self) -> u16 {
        match self {
            CongestionInfo::V1(inner) => inner.allowed_shard,
        }
    }

    pub fn set_allowed_shard(&mut self, allowed_shard: u16) {
        match self {
            CongestionInfo::V1(inner) => inner.allowed_shard = allowed_shard,
        }
    }

    pub fn add_receipt_bytes(&mut self, bytes: u64) -> Result<(), RuntimeError> {
        match self {
            CongestionInfo::V1(inner) => {
                inner.receipt_bytes = inner.receipt_bytes.checked_add(bytes).ok_or_else(|| {
                    RuntimeError::UnexpectedIntegerOverflow("add_receipt_bytes".into())
                })?;
            }
        }
        Ok(())
    }

    pub fn remove_receipt_bytes(&mut self, bytes: u64) -> Result<(), RuntimeError> {
        match self {
            CongestionInfo::V1(inner) => {
                inner.receipt_bytes = inner.receipt_bytes.checked_sub(bytes).ok_or_else(|| {
                    RuntimeError::UnexpectedIntegerOverflow("remove_receipt_bytes".into())
                })?;
            }
        }
        Ok(())
    }

    pub fn add_delayed_receipt_gas(&mut self, gas: Gas) -> Result<(), RuntimeError> {
        match self {
            CongestionInfo::V1(inner) => {
                inner.delayed_receipts_gas = inner
                    .delayed_receipts_gas
                    .checked_add(gas as u128)
                    .ok_or_else(|| {
                        RuntimeError::UnexpectedIntegerOverflow("add_delayed_receipt_gas".into())
                    })?;
            }
        }
        Ok(())
    }

    pub fn remove_delayed_receipt_gas(&mut self, gas: Gas) -> Result<(), RuntimeError> {
        match self {
            CongestionInfo::V1(inner) => {
                inner.delayed_receipts_gas = inner
                    .delayed_receipts_gas
                    .checked_sub(gas as u128)
                    .ok_or_else(|| {
                        RuntimeError::UnexpectedIntegerOverflow("remove_delayed_receipt_gas".into())
                    })?;
            }
        }
        Ok(())
    }

    pub fn add_buffered_receipt_gas(&mut self, gas: Gas) -> Result<(), RuntimeError> {
        match self {
            CongestionInfo::V1(inner) => {
                inner.buffered_receipts_gas = inner
                    .buffered_receipts_gas
                    .checked_add(gas as u128)
                    .ok_or_else(|| {
                        RuntimeError::UnexpectedIntegerOverflow("add_buffered_receipt_gas".into())
                    })?;
            }
        }
        Ok(())
    }

    pub fn remove_buffered_receipt_gas(&mut self, gas: Gas) -> Result<(), RuntimeError> {
        match self {
            CongestionInfo::V1(inner) => {
                inner.buffered_receipts_gas = inner
                    .buffered_receipts_gas
                    .checked_sub(gas as u128)
                    .ok_or_else(|| {
                        RuntimeError::UnexpectedIntegerOverflow(
                            "remove_buffered_receipt_gas".into(),
                        )
                    })?;
            }
        }
        Ok(())
    }

    /// Congestion level ignoring the chain context (missed chunks count).
    pub fn localized_congestion_level(&self, config: &CongestionControlConfig) -> f64 {
        let incoming_congestion = self.incoming_congestion(config);
        let outgoing_congestion = self.outgoing_congestion(config);
        let memory_congestion = self.memory_congestion(config);
        incoming_congestion
            .max(outgoing_congestion)
            .max(memory_congestion)
    }

    pub fn incoming_congestion(&self, config: &CongestionControlConfig) -> f64 {
        clamped_f64_fraction(
            self.delayed_receipts_gas(),
            config.max_congestion_incoming_gas,
        )
    }

    pub fn outgoing_congestion(&self, config: &CongestionControlConfig) -> f64 {
        clamped_f64_fraction(
            self.buffered_receipts_gas(),
            config.max_congestion_outgoing_gas,
        )
    }

    pub fn memory_congestion(&self, config: &CongestionControlConfig) -> f64 {
        clamped_f64_fraction(
            self.receipt_bytes() as u128,
            config.max_congestion_memory_consumption,
        )
    }

    /// Computes and sets the `allowed_shard` field.
    ///
    /// If in a fully congested state, decide which shard of the shards is
    /// allowed to forward gas to `own_shard` this round. In this case, we stop all
    /// of the shards from sending anything to `own_shard`. But to guarantee
    /// progress, we allow one shard to send `allowed_shard_outgoing_gas`
    /// in the next chunk.
    ///
    /// It is also used to determine the size limit for outgoing receipts from sender shards.
    /// Only the allowed shard can send receipts of size `outgoing_receipts_big_size_limit`.
    /// Other shards can only send receipts of size `outgoing_receipts_usual_size_limit`.
    pub fn finalize_allowed_shard(
        &mut self,
        own_shard: ShardId,
        all_shards: &[ShardId],
        congestion_seed: u64,
    ) {
        let allowed_shard = Self::get_new_allowed_shard(own_shard, all_shards, congestion_seed);
        self.set_allowed_shard(allowed_shard as u16);
    }

    fn get_new_allowed_shard(
        own_shard: ShardId,
        all_shards: &[ShardId],
        congestion_seed: u64,
    ) -> ShardId {
        if let Some(index) = congestion_seed.checked_rem(all_shards.len() as u64) {
            // round robin for other shards based on the seed
            return *all_shards
                .get(index as usize)
                .expect("`checked_rem` should have ensured array access is in bound");
        }
        // checked_rem failed, hence all_shards.len() is 0
        // own_shard is the only choice.
        return own_shard;
    }
}

/// The block congestion info contains the congestion info for all shards in the
/// block extended with the missed chunks count.
#[derive(Clone, Debug, Default)]
pub struct BlockCongestionInfo {
    /// The per shard congestion info. It's important that the data structure is
    /// deterministic because the allowed shard id selection depends on the
    /// order of shard ids in this map. Ideally it should also be sorted by shard id.
    shards_congestion_info: BTreeMap<ShardId, ExtendedCongestionInfo>,
}

impl BlockCongestionInfo {
    pub fn new(shards_congestion_info: BTreeMap<ShardId, ExtendedCongestionInfo>) -> Self {
        Self {
            shards_congestion_info,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ShardId, &ExtendedCongestionInfo)> {
        self.shards_congestion_info.iter()
    }

    pub fn all_shards(&self) -> Vec<ShardId> {
        self.shards_congestion_info.keys().copied().collect()
    }

    pub fn get(&self, shard_id: &ShardId) -> Option<&ExtendedCongestionInfo> {
        self.shards_congestion_info.get(shard_id)
    }

    pub fn get_mut(&mut self, shard_id: &ShardId) -> Option<&mut ExtendedCongestionInfo> {
        self.shards_congestion_info.get_mut(shard_id)
    }

    pub fn insert(
        &mut self,
        shard_id: ShardId,
        value: ExtendedCongestionInfo,
    ) -> Option<ExtendedCongestionInfo> {
        self.shards_congestion_info.insert(shard_id, value)
    }

    pub fn is_empty(&self) -> bool {
        self.shards_congestion_info.is_empty()
    }
}

/// The extended congestion info contains the congestion info and extra
/// information extracted from the block that is needed for congestion control.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExtendedCongestionInfo {
    pub congestion_info: CongestionInfo,
    pub missed_chunks_count: u64,
}

impl ExtendedCongestionInfo {
    pub fn new(congestion_info: CongestionInfo, missed_chunks_count: u64) -> Self {
        Self {
            congestion_info,
            missed_chunks_count,
        }
    }
}

/// Stores the congestion level of a shard.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    ProtocolSchema,
)]
pub struct CongestionInfoV1 {
    /// Sum of gas in currently delayed receipts.
    pub delayed_receipts_gas: u128,
    /// Sum of gas in currently buffered receipts.
    pub buffered_receipts_gas: u128,
    /// Size of borsh serialized receipts stored in state because they
    /// were delayed, buffered, postponed, or yielded.
    pub receipt_bytes: u64,
    /// If fully congested, only this shard can forward receipts.
    pub allowed_shard: u16,
}

/// Returns `value / max` clamped to te range [0,1].
#[inline]
fn clamped_f64_fraction(value: u128, max: u64) -> f64 {
    assert!(max > 0);
    if max as u128 <= value {
        1.0
    } else {
        value as f64 / max as f64
    }
}

/// linearly interpolate between two values
///
/// This method treats u16 as a fraction of u16::MAX.
/// This makes multiplication of numbers on the upper end of `u128` better behaved
/// than using f64 which lacks precision for such high numbers and might have platform incompatibilities.
fn mix(left: u64, right: u64, ratio: f64) -> u64 {
    debug_assert!(ratio >= 0.0);
    debug_assert!(ratio <= 1.0);

    // Note on precision: f64 is only precise to 53 binary digits. That is
    // enough to represent ~9 PGAS without error. Precision above that is
    // rounded according to the IEEE 754-2008 standard which Rust's f64
    // implements.
    // For example, a value of 100 Pgas is rounded to steps of 8 gas.
    let left_part = left as f64 * (1.0 - ratio);
    let right_part = right as f64 * ratio;
    // Accumulated error is doubled again, up to 16 gas for 100 Pgas.
    let total = left_part + right_part;

    // Conversion is save because left and right were both u64 and the result is
    // between the two. Even with precision errors, we cannot breach the
    // boundaries.
    return total.round() as u64;
}

impl ShardAcceptsTransactions {
    pub fn is_yes(&self) -> bool {
        matches!(self, ShardAcceptsTransactions::Yes)
    }

    pub fn is_no(&self) -> bool {
        !self.is_yes()
    }
}
