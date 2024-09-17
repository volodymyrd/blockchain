pub mod shard_chunk_header_inner;

use borsh::{BorshDeserialize, BorshSerialize};
use near_crypto::Signature;
pub use shard_chunk_header_inner::ShardChunkHeaderInner;

use crate::congestion_info::CongestionInfo;
use crate::merkle::combine_hash;
use crate::receipt::Receipt;
use crate::sharding::shard_chunk_header_inner::ShardChunkHeaderInnerV3;
use crate::transaction::SignedTransaction;
use crate::types::validator_stake::ValidatorStake;
use crate::types::StateRoot;
use crate::validator_signer::ValidatorSigner;
use near_primitives_core::hash::{hash, CryptoHash};
use near_primitives_core::types::{Balance, BlockHeight, Gas, ProtocolVersion, ShardId};
use near_schema_checker_lib::ProtocolSchema;

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Hash,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
    ProtocolSchema,
)]
pub struct ChunkHash(pub CryptoHash);

impl AsRef<[u8]> for ChunkHash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Debug, ProtocolSchema)]
#[borsh(init=init)]
pub struct ShardChunkHeaderV3 {
    pub inner: ShardChunkHeaderInner,

    pub height_included: BlockHeight,

    /// Signature of the chunk producer.
    pub signature: Signature,

    #[borsh(skip)]
    pub hash: ChunkHash,
}

impl ShardChunkHeaderV3 {
    pub fn init(&mut self) {
        self.hash = Self::compute_hash(&self.inner);
    }

    pub fn compute_hash(inner: &ShardChunkHeaderInner) -> ChunkHash {
        let inner_bytes = borsh::to_vec(&inner).expect("Failed to serialize");
        let inner_hash = hash(&inner_bytes);

        ChunkHash(combine_hash(&inner_hash, inner.encoded_merkle_root()))
    }

    pub fn new(
        protocol_version: ProtocolVersion,
        prev_block_hash: CryptoHash,
        prev_state_root: StateRoot,
        prev_outcome_root: CryptoHash,
        encoded_merkle_root: CryptoHash,
        encoded_length: u64,
        height: BlockHeight,
        shard_id: ShardId,
        prev_gas_used: Gas,
        gas_limit: Gas,
        prev_balance_burnt: Balance,
        prev_outgoing_receipts_root: CryptoHash,
        tx_root: CryptoHash,
        prev_validator_proposals: Vec<ValidatorStake>,
        congestion_info: Option<CongestionInfo>,
        signer: &ValidatorSigner,
    ) -> Self {
        let inner = ShardChunkHeaderInner::V3(ShardChunkHeaderInnerV3 {
            prev_block_hash,
            prev_state_root,
            prev_outcome_root,
            encoded_merkle_root,
            encoded_length,
            height_created: height,
            shard_id,
            prev_gas_used,
            gas_limit,
            prev_balance_burnt,
            prev_outgoing_receipts_root,
            tx_root,
            prev_validator_proposals,
            congestion_info: congestion_info.unwrap(),
        });
        Self::from_inner(inner, signer)
    }

    pub fn from_inner(inner: ShardChunkHeaderInner, signer: &ValidatorSigner) -> Self {
        let hash = Self::compute_hash(&inner);
        let signature = signer.sign_chunk_hash(&hash);
        Self {
            inner,
            height_included: 0,
            signature,
            hash,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq, ProtocolSchema)]
pub struct ShardChunkV2 {
    pub chunk_hash: ChunkHash,
    pub header: ShardChunkHeader,
    pub transactions: Vec<SignedTransaction>,
    pub prev_outgoing_receipts: Vec<Receipt>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq, ProtocolSchema)]
pub enum ShardChunk {
    V2(ShardChunkV2),
}
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Debug, ProtocolSchema)]
pub enum ShardChunkHeader {
    V3(ShardChunkHeaderV3),
}

impl ShardChunkHeader {
    #[inline]
    pub fn height_included(&self) -> BlockHeight {
        match self {
            Self::V3(header) => header.height_included,
        }
    }

    #[inline]
    pub fn prev_state_root(&self) -> StateRoot {
        match self {
            Self::V3(header) => *header.inner.prev_state_root(),
        }
    }

    #[inline]
    pub fn prev_outgoing_receipts_root(&self) -> CryptoHash {
        match &self {
            ShardChunkHeader::V3(header) => *header.inner.prev_outgoing_receipts_root(),
        }
    }

    #[inline]
    pub fn tx_root(&self) -> CryptoHash {
        match &self {
            ShardChunkHeader::V3(header) => *header.inner.tx_root(),
        }
    }

    #[inline]
    pub fn chunk_hash(&self) -> ChunkHash {
        match &self {
            ShardChunkHeader::V3(header) => header.hash.clone(),
        }
    }
}
#[derive(
    Default, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, ProtocolSchema,
)]
pub struct EncodedShardChunkBody {
    pub parts: Vec<Option<Box<[u8]>>>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, ProtocolSchema)]
pub struct EncodedShardChunkV2 {
    pub header: ShardChunkHeader,
    pub content: EncodedShardChunkBody,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, ProtocolSchema)]
pub enum EncodedShardChunk {
    V2(EncodedShardChunkV2),
}

#[derive(
    BorshSerialize, BorshDeserialize, Hash, Eq, PartialEq, Clone, Debug, Default, ProtocolSchema,
)]
pub struct ChunkHashHeight(pub ChunkHash, pub BlockHeight);
