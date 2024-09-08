pub mod shard_chunk_header_inner;
use near_crypto::Signature;
pub use shard_chunk_header_inner::ShardChunkHeaderInner;

use crate::receipt::Receipt;
use crate::transaction::SignedTransaction;
use near_primitives_core::hash::CryptoHash;
use near_primitives_core::types::BlockHeight;

pub struct ChunkHash(pub CryptoHash);

pub struct ShardChunkHeaderV3 {
    pub inner: ShardChunkHeaderInner,

    pub height_included: BlockHeight,

    /// Signature of the chunk producer.
    pub signature: Signature,

    pub hash: ChunkHash,
}

pub struct ShardChunkV2 {
    pub chunk_hash: ChunkHash,
    pub header: ShardChunkHeader,
    pub transactions: Vec<SignedTransaction>,
    pub prev_outgoing_receipts: Vec<Receipt>,
}

pub enum ShardChunk {
    V2(ShardChunkV2),
}
pub enum ShardChunkHeader {
    V3(ShardChunkHeaderV3),
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct EncodedShardChunkBody {
    pub parts: Vec<Option<Box<[u8]>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedShardChunkV2 {
    pub header: ShardChunkHeader,
    pub content: EncodedShardChunkBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodedShardChunk {
    V2(EncodedShardChunkV2),
}
