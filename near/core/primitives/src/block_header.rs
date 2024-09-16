use crate::challenge::ChallengesResult;
use crate::merkle::combine_hash;
use crate::stateless_validation::chunk_endorsements_bitmap::ChunkEndorsementsBitmap;
use crate::types::validator_stake::ValidatorStake;
use crate::types::EpochId;
use borsh::{BorshDeserialize, BorshSerialize};
use near_crypto::Signature;
use near_primitives_core::hash::{hash, CryptoHash};
use near_primitives_core::types::{Balance, BlockHeight, MerkleHash, NumBlocks, ProtocolVersion};
use near_schema_checker_lib::ProtocolSchema;
use std::sync::Arc;

#[derive(
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Default,
    ProtocolSchema,
)]
pub struct BlockHeaderInnerLite {
    /// Height of this block.
    pub height: BlockHeight,
    /// Epoch start hash of this block's epoch.
    /// Used for retrieving validator information
    pub epoch_id: EpochId,
    pub next_epoch_id: EpochId,
    /// Root hash of the state at the previous block.
    pub prev_state_root: MerkleHash,
    /// Root of the outcomes of transactions and receipts from the previous chunks.
    pub prev_outcome_root: MerkleHash,
    /// Timestamp at which the block was built (number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC).
    pub timestamp: u64,
    /// Hash of the next epoch block producers set
    pub next_bp_hash: CryptoHash,
    /// Merkle root of block hashes up to the current block.
    pub block_merkle_root: CryptoHash,
}

/// Add `chunk_endorsements`
#[derive(
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Default,
    ProtocolSchema,
)]
pub struct BlockHeaderInnerRestV5 {
    /// Hash of block body
    pub block_body_hash: CryptoHash,
    /// Root hash of the previous chunks' outgoing receipts in the given block.
    pub prev_chunk_outgoing_receipts_root: MerkleHash,
    /// Root hash of the chunk headers in the given block.
    pub chunk_headers_root: MerkleHash,
    /// Root hash of the chunk transactions in the given block.
    pub chunk_tx_root: MerkleHash,
    /// Root hash of the challenges in the given block.
    pub challenges_root: MerkleHash,
    /// The output of the randomness beacon
    pub random_value: CryptoHash,
    /// Validator proposals from the previous chunks.
    pub prev_validator_proposals: Vec<ValidatorStake>,
    /// Mask for new chunks included in the block
    pub chunk_mask: Vec<bool>,
    /// Gas price for chunks in the next block.
    pub next_gas_price: Balance,
    /// Total supply of tokens in the system
    pub total_supply: Balance,
    /// List of challenges result from previous block.
    pub challenges_result: ChallengesResult,

    /// Last block that has full BFT finality
    pub last_final_block: CryptoHash,
    /// Last block that has doomslug finality
    pub last_ds_final_block: CryptoHash,

    /// The ordinal of the Block on the Canonical Chain
    pub block_ordinal: NumBlocks,

    pub prev_height: BlockHeight,

    pub epoch_sync_data_hash: Option<CryptoHash>,

    /// All the approvals included in this block
    pub approvals: Vec<Option<Box<Signature>>>,

    /// Latest protocol version that this block producer has.
    pub latest_protocol_version: ProtocolVersion,

    pub chunk_endorsements: ChunkEndorsementsBitmap,
}

#[derive(
    BorshSerialize, BorshDeserialize, serde::Serialize, Debug, Clone, Eq, PartialEq, Default,
)]
#[borsh(init=init)]
pub struct BlockHeaderV5 {
    pub prev_hash: CryptoHash,

    /// Inner part of the block header that gets hashed, split into two parts, one that is sent
    ///    to light clients, and the rest
    pub inner_lite: BlockHeaderInnerLite,
    pub inner_rest: BlockHeaderInnerRestV5,

    /// Signature of the block producer.
    pub signature: Signature,

    /// Cached value of hash for this block.
    #[borsh(skip)]
    pub hash: CryptoHash,
}

impl BlockHeaderV5 {
    pub fn init(&mut self) {
        self.hash = BlockHeader::compute_hash(
            self.prev_hash,
            &borsh::to_vec(&self.inner_lite).expect("Failed to serialize"),
            &borsh::to_vec(&self.inner_rest).expect("Failed to serialize"),
        );
    }
}

/// Versioned BlockHeader data structure.
/// For each next version, document what are the changes between versions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BlockHeader {
    BlockHeaderV5(Arc<BlockHeaderV5>),
}

impl BlockHeader {
    pub fn compute_inner_hash(inner_lite: &[u8], inner_rest: &[u8]) -> CryptoHash {
        let hash_lite = hash(inner_lite);
        let hash_rest = hash(inner_rest);
        combine_hash(&hash_lite, &hash_rest)
    }

    pub fn compute_hash(prev_hash: CryptoHash, inner_lite: &[u8], inner_rest: &[u8]) -> CryptoHash {
        let hash_inner = BlockHeader::compute_inner_hash(inner_lite, inner_rest);

        combine_hash(&hash_inner, &prev_hash)
    }
}
