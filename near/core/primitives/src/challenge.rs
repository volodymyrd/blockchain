use crate::merkle::MerklePath;
use crate::sharding::{EncodedShardChunk, ShardChunk, ShardChunkHeader};
use borsh::{BorshDeserialize, BorshSerialize};
use crate::types::AccountId;
use near_crypto::Signature;
use near_primitives_core::hash::CryptoHash;
use near_schema_checker_lib::ProtocolSchema;
use std::fmt::{Debug, Formatter};

/// Double signed block.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct BlockDoubleSign {
    pub left_block_header: Vec<u8>,
    pub right_block_header: Vec<u8>,
}

/// Either `EncodedShardChunk` or `ShardChunk`. Used for `ChunkProofs`.
/// `Decoded` is used to avoid re-encoding an already decoded chunk to construct a challenge.
/// `Encoded` is still needed in case a challenge challenges an invalid encoded chunk that can't be
/// decoded.
#[allow(clippy::large_enum_variant)] // both variants are large
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MaybeEncodedShardChunk {
    Encoded(EncodedShardChunk),
    Decoded(ShardChunk),
}

/// Invalid chunk (body of the chunk doesn't match proofs or invalid encoding).
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ChunkProofs {
    /// Encoded block header that contains invalid chunk.
    pub block_header: Vec<u8>,
    /// Merkle proof of inclusion of this chunk.
    pub merkle_proof: MerklePath,
    /// Invalid chunk in an encoded form or in a decoded form.
    pub chunk: Box<MaybeEncodedShardChunk>,
}

/// Serialized TrieNodeWithSize or state value.
pub type TrieValue = std::sync::Arc<[u8]>;

#[derive(BorshSerialize, BorshDeserialize, Clone, Eq, PartialEq, ProtocolSchema)]
/// TODO (#8984): consider supporting format containing trie values only for
/// state part boundaries and storing state items for state part range.
pub enum PartialState {
    /// State represented by the set of unique trie values (`RawTrieNodeWithSize`s and state values).
    TrieValues(Vec<TrieValue>),
}

impl Default for PartialState {
    fn default() -> Self {
        PartialState::TrieValues(vec![])
    }
}

// When debug-printing, don't dump the entire partial state; that is very unlikely to be useful,
// and wastes a lot of screen space.
impl Debug for PartialState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PartialState::TrieValues(values) => {
                f.write_str(&format!("{} trie values", values.len()))
            }
        }
    }
}

/// Doesn't match post-{state root, outgoing receipts, gas used, etc} results after applying previous chunk.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ChunkState {
    /// Encoded prev block header.
    pub prev_block_header: Vec<u8>,
    /// Encoded block header that contains invalid chunnk.
    pub block_header: Vec<u8>,
    /// Merkle proof in inclusion of prev chunk.
    pub prev_merkle_proof: MerklePath,
    /// Previous chunk that contains transactions.
    pub prev_chunk: ShardChunk,
    /// Merkle proof of inclusion of this chunk.
    pub merkle_proof: MerklePath,
    /// Invalid chunk header.
    pub chunk_header: ShardChunkHeader,
    /// Partial state that was affected by transactions of given chunk.
    pub partial_state: PartialState,
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ChallengeBody {
    BlockDoubleSign(BlockDoubleSign),
    ChunkProofs(ChunkProofs),
    ChunkState(ChunkState),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Challenge {
    pub body: ChallengeBody,
    pub account_id: AccountId,
    pub signature: Signature,

    pub hash: CryptoHash,
}

pub type Challenges = Vec<Challenge>;

#[derive(
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    ProtocolSchema,
)]
pub struct SlashedValidator {
    pub account_id: AccountId,
    pub is_double_sign: bool,
}

/// Result of checking challenge, contains which accounts to slash.
/// If challenge is invalid this is sender, otherwise author of chunk (and possibly other participants that signed invalid blocks).
pub type ChallengesResult = Vec<SlashedValidator>;
