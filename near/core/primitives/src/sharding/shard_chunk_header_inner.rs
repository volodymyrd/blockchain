use crate::congestion_info::CongestionInfo;
use crate::types::validator_stake::ValidatorStake;
use crate::types::StateRoot;
use borsh::{BorshDeserialize, BorshSerialize};
use near_primitives_core::hash::CryptoHash;
use near_primitives_core::types::{Balance, BlockHeight, Gas, ShardId};
use near_schema_checker_lib::ProtocolSchema;

#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Debug, ProtocolSchema)]
pub enum ShardChunkHeaderInner {
    V3(ShardChunkHeaderInnerV3),
}

#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Debug, ProtocolSchema)]
pub struct ShardChunkHeaderInnerV3 {
    /// Previous block hash.
    pub prev_block_hash: CryptoHash,
    pub prev_state_root: StateRoot,
    /// Root of the outcomes from execution transactions and results of the previous chunk.
    pub prev_outcome_root: CryptoHash,
    pub encoded_merkle_root: CryptoHash,
    pub encoded_length: u64,
    pub height_created: BlockHeight,
    /// Shard index.
    pub shard_id: ShardId,
    /// Gas used in the previous chunk.
    pub prev_gas_used: Gas,
    /// Gas limit voted by validators.
    pub gas_limit: Gas,
    /// Total balance burnt in the previous chunk.
    pub prev_balance_burnt: Balance,
    /// Previous chunk's outgoing receipts merkle root.
    pub prev_outgoing_receipts_root: CryptoHash,
    /// Tx merkle root.
    pub tx_root: CryptoHash,
    /// Validator proposals from the previous chunk.
    pub prev_validator_proposals: Vec<ValidatorStake>,
    /// Congestion info about this shard after the previous chunk was applied.
    pub congestion_info: CongestionInfo,
}

impl ShardChunkHeaderInner {
    #[inline]
    pub fn prev_state_root(&self) -> &StateRoot {
        match self {
            Self::V3(inner) => &inner.prev_state_root,
        }
    }
    #[inline]
    pub fn encoded_merkle_root(&self) -> &CryptoHash {
        match self {
            Self::V3(inner) => &inner.encoded_merkle_root,
        }
    }

    #[inline]
    pub fn prev_outgoing_receipts_root(&self) -> &CryptoHash {
        match self {
            Self::V3(inner) => &inner.prev_outgoing_receipts_root,
        }
    }

    #[inline]
    pub fn tx_root(&self) -> &CryptoHash {
        match self {
            Self::V3(inner) => &inner.tx_root,
        }
    }
}
