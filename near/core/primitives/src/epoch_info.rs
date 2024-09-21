use borsh::{BorshDeserialize, BorshSerialize};
use smart_default::SmartDefault;
use std::collections::{BTreeMap, HashMap};

use crate::rand::WeightedIndex;
use crate::types::validator_stake::{ValidatorStake, ValidatorStakeIter};
use crate::types::{AccountId, ValidatorKickoutReason, ValidatorStakeV1};
use crate::validator_mandates::ValidatorMandates;
use crate::version::PROTOCOL_VERSION;
use near_primitives_core::types::{Balance, EpochHeight, ProtocolVersion, ValidatorId};
use near_primitives_core::version::ProtocolFeature;
use near_primitives_core::{
    checked_feature,
    hash::hash,
    types::{BlockHeight, ShardId},
};
use near_schema_checker_lib::ProtocolSchema;

/// Information per epoch.
#[derive(
    BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, serde::Serialize, ProtocolSchema,
)]
pub enum EpochInfo {
    V4(EpochInfoV4),
}

pub type RngSeed = [u8; 32];

#[derive(
    Default,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    ProtocolSchema,
)]
pub struct ValidatorWeight(ValidatorId, u64);

// V3 -> V4: Add structures and methods for stateless validator assignment.
#[derive(
    SmartDefault,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    ProtocolSchema,
)]
pub struct EpochInfoV4 {
    pub epoch_height: EpochHeight,
    pub validators: Vec<ValidatorStake>,
    pub validator_to_index: HashMap<AccountId, ValidatorId>,
    pub block_producers_settlement: Vec<ValidatorId>,
    pub chunk_producers_settlement: Vec<Vec<ValidatorId>>,
    /// Deprecated.
    pub _hidden_validators_settlement: Vec<ValidatorWeight>,
    /// Deprecated.
    pub _fishermen: Vec<crate::types::validator_stake::ValidatorStake>,
    /// Deprecated.
    pub _fishermen_to_index: HashMap<AccountId, ValidatorId>,
    pub stake_change: BTreeMap<AccountId, Balance>,
    pub validator_reward: HashMap<AccountId, Balance>,
    pub validator_kickout: HashMap<AccountId, ValidatorKickoutReason>,
    pub minted_amount: Balance,
    pub seat_price: Balance,
    #[default(PROTOCOL_VERSION)]
    pub protocol_version: ProtocolVersion,
    // stuff for selecting validators at each height
    rng_seed: RngSeed,
    block_producers_sampler: crate::rand::WeightedIndex,
    chunk_producers_sampler: Vec<crate::rand::WeightedIndex>,
    /// Contains the epoch's validator mandates. Used to sample chunk validators.
    validator_mandates: crate::validator_mandates::ValidatorMandates,
}

impl EpochInfo {
    pub fn new(
        epoch_height: EpochHeight,
        validators: Vec<ValidatorStake>,
        validator_to_index: HashMap<AccountId, ValidatorId>,
        block_producers_settlement: Vec<ValidatorId>,
        chunk_producers_settlement: Vec<Vec<ValidatorId>>,
        stake_change: BTreeMap<AccountId, Balance>,
        validator_reward: HashMap<AccountId, Balance>,
        validator_kickout: HashMap<AccountId, ValidatorKickoutReason>,
        minted_amount: Balance,
        seat_price: Balance,
        protocol_version: ProtocolVersion,
        rng_seed: RngSeed,
        validator_mandates: ValidatorMandates,
    ) -> Self {
        let stake_weights = |ids: &[ValidatorId]| -> WeightedIndex {
            WeightedIndex::new(
                ids.iter()
                    .copied()
                    .map(|validator_id| validators[validator_id as usize].stake())
                    .collect(),
            )
        };
        let block_producers_sampler = stake_weights(&block_producers_settlement);
        let chunk_producers_sampler = chunk_producers_settlement
            .iter()
            .map(|vs| stake_weights(vs))
            .collect();

        Self::V4(EpochInfoV4 {
            epoch_height,
            validators,
            _fishermen: Default::default(),
            validator_to_index,
            block_producers_settlement,
            chunk_producers_settlement,
            _hidden_validators_settlement: Default::default(),
            stake_change,
            validator_reward,
            validator_kickout,
            _fishermen_to_index: Default::default(),
            minted_amount,
            seat_price,
            protocol_version,
            rng_seed,
            block_producers_sampler,
            chunk_producers_sampler,
            validator_mandates,
        })
    }

    #[inline]
    pub fn epoch_height_mut(&mut self) -> &mut EpochHeight {
        match self {
            Self::V4(v4) => &mut v4.epoch_height,
        }
    }

    #[inline]
    pub fn epoch_height(&self) -> EpochHeight {
        match self {
            Self::V4(v4) => v4.epoch_height,
        }
    }

    #[inline]
    pub fn seat_price(&self) -> Balance {
        match self {
            Self::V4(v4) => v4.seat_price,
        }
    }

    #[inline]
    pub fn minted_amount(&self) -> Balance {
        match self {
            Self::V4(v4) => v4.minted_amount,
        }
    }

    #[inline]
    pub fn block_producers_settlement(&self) -> &[ValidatorId] {
        match self {
            Self::V4(v4) => &v4.block_producers_settlement,
        }
    }

    #[inline]
    pub fn chunk_producers_settlement(&self) -> &[Vec<ValidatorId>] {
        match self {
            Self::V4(v4) => &v4.chunk_producers_settlement,
        }
    }

    #[inline]
    pub fn chunk_producers_settlement_mut(&mut self) -> &mut Vec<Vec<ValidatorId>> {
        match self {
            Self::V4(v4) => &mut v4.chunk_producers_settlement,
        }
    }

    #[inline]
    pub fn validator_kickout(&self) -> &HashMap<AccountId, ValidatorKickoutReason> {
        match self {
            Self::V4(v4) => &v4.validator_kickout,
        }
    }

    #[inline]
    pub fn protocol_version(&self) -> ProtocolVersion {
        match self {
            Self::V4(v4) => v4.protocol_version,
        }
    }

    #[inline]
    pub fn stake_change(&self) -> &BTreeMap<AccountId, Balance> {
        match self {
            Self::V4(v4) => &v4.stake_change,
        }
    }

    #[inline]
    pub fn validator_reward(&self) -> &HashMap<AccountId, Balance> {
        match self {
            Self::V4(v4) => &v4.validator_reward,
        }
    }

    #[inline]
    pub fn validators_iter(&self) -> ValidatorStakeIter {
        match self {
            Self::V4(v4) => ValidatorStakeIter::new(&v4.validators),
        }
    }

    #[inline]
    pub fn fishermen_iter(&self) -> ValidatorStakeIter {
        match self {
            Self::V4(v4) => ValidatorStakeIter::new(&v4._fishermen),
        }
    }

    #[inline]
    pub fn validator_stake(&self, validator_id: u64) -> Balance {
        match self {
            Self::V4(v4) => v4.validators[validator_id as usize].stake(),
        }
    }

    #[inline]
    pub fn validator_account_id(&self, validator_id: u64) -> &AccountId {
        match self {
            Self::V4(v4) => v4.validators[validator_id as usize].account_id(),
        }
    }

    #[inline]
    pub fn account_is_validator(&self, account_id: &AccountId) -> bool {
        match self {
            Self::V4(v4) => v4.validator_to_index.contains_key(account_id),
        }
    }

    pub fn get_validator_id(&self, account_id: &AccountId) -> Option<&ValidatorId> {
        match self {
            Self::V4(v4) => v4.validator_to_index.get(account_id),
        }
    }

    pub fn get_validator_by_account(&self, account_id: &AccountId) -> Option<ValidatorStake> {
        match self {
            Self::V4(v4) => v4
                .validator_to_index
                .get(account_id)
                .map(|validator_id| v4.validators[*validator_id as usize].clone()),
        }
    }

    #[inline]
    pub fn get_validator(&self, validator_id: u64) -> ValidatorStake {
        match self {
            Self::V4(v4) => v4.validators[validator_id as usize].clone(),
        }
    }

    #[inline]
    pub fn account_is_fisherman(&self, account_id: &AccountId) -> bool {
        match self {
            Self::V4(v4) => v4._fishermen_to_index.contains_key(account_id),
        }
    }

    pub fn get_fisherman_by_account(&self, account_id: &AccountId) -> Option<ValidatorStake> {
        match self {
            Self::V4(v4) => v4
                ._fishermen_to_index
                .get(account_id)
                .map(|validator_id| v4._fishermen[*validator_id as usize].clone()),
        }
    }

    #[inline]
    pub fn get_fisherman(&self, fisherman_id: u64) -> ValidatorStake {
        match self {
            Self::V4(v4) => v4._fishermen[fisherman_id as usize].clone(),
        }
    }

    #[inline]
    pub fn validators_len(&self) -> usize {
        match self {
            Self::V4(v4) => v4.validators.len(),
        }
    }

    #[inline]
    pub fn rng_seed(&self) -> RngSeed {
        match self {
            Self::V4(v4) => v4.rng_seed,
        }
    }

    #[inline]
    pub fn validator_mandates(&self) -> ValidatorMandates {
        match self {
            Self::V4(v4) => v4.validator_mandates.clone(),
        }
    }

    pub fn sample_block_producer(&self, height: BlockHeight) -> ValidatorId {
        match &self {
            Self::V4(v4) => {
                let seed = Self::block_produce_seed(height, &v4.rng_seed);
                v4.block_producers_settlement[v4.block_producers_sampler.sample(seed)]
            }
        }
    }

    pub fn sample_chunk_producer(
        &self,
        height: BlockHeight,
        shard_id: ShardId,
    ) -> Option<ValidatorId> {
        match &self {
            Self::V4(v4) => {
                let protocol_version = self.protocol_version();
                let seed =
                    Self::chunk_produce_seed(protocol_version, &v4.rng_seed, height, shard_id);
                let shard_id = shard_id as usize;
                let sample = v4.chunk_producers_sampler.get(shard_id)?.sample(seed);
                v4.chunk_producers_settlement
                    .get(shard_id)?
                    .get(sample)
                    .copied()
            }
        }
    }

    /// 32 bytes from epoch_seed, 8 bytes from height
    fn block_produce_seed(height: BlockHeight, seed: &RngSeed) -> [u8; 32] {
        let mut buffer = [0u8; 40];
        buffer[0..32].copy_from_slice(seed);
        buffer[32..40].copy_from_slice(&height.to_le_bytes());
        hash(&buffer).0
    }

    fn chunk_produce_seed(
        protocol_version: ProtocolVersion,
        seed: &RngSeed,
        height: BlockHeight,
        shard_id: ShardId,
    ) -> [u8; 32] {
        if checked_feature!("stable", SynchronizeBlockChunkProduction, protocol_version)
            && !checked_feature!("stable", ChunkOnlyProducers, protocol_version)
        {
            // This is same seed that used for determining block
            // producer. This seed does not contain the shard id
            // so all shards will be produced by the same
            // validator.
            Self::block_produce_seed(height, seed)
        } else {
            // 32 bytes from epoch_seed, 8 bytes from height, 8 bytes from shard_id
            let mut buffer = [0u8; 48];
            buffer[0..32].copy_from_slice(seed);
            buffer[32..40].copy_from_slice(&height.to_le_bytes());
            buffer[40..48].copy_from_slice(&shard_id.to_le_bytes());
            hash(&buffer).0
        }
    }
}
