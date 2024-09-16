use crate::challenge::Challenges;
use crate::sharding::ShardChunkHeader;
use near_crypto::Signature;
use near_crypto::vrf::{Proof, Value};
use near_primitives_core::types::ProtocolVersion;

pub type ChunkEndorsementSignatures = Vec<Option<Box<Signature>>>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BlockBodyV2 {
    pub chunks: Vec<ShardChunkHeader>,
    pub challenges: Challenges,

    // Data to confirm the correctness of randomness beacon output
    pub vrf_value: Value,
    pub vrf_proof: Proof,

    // Chunk endorsements
    // These are structured as a vector of Signatures from all ordered chunk_validators
    // for each shard got from fn get_ordered_chunk_validators
    // chunk_endorsements[shard_id][chunk_validator_index] is the signature (if present).
    // If the chunk_validator did not endorse the chunk, the signature is None.
    // For cases of missing chunk, we include the chunk endorsements from the previous
    // block just like we do for chunks.
    pub chunk_endorsements: Vec<ChunkEndorsementSignatures>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BlockBody {
    V2(BlockBodyV2),
}

impl BlockBody {
    pub fn new(
        protocol_version: ProtocolVersion,
        chunks: Vec<ShardChunkHeader>,
        challenges: Challenges,
        vrf_value: Value,
        vrf_proof: Proof,
        chunk_endorsements: Vec<ChunkEndorsementSignatures>,
    ) -> Self {
        BlockBody::V2(BlockBodyV2 {
            chunks,
            challenges,
            vrf_value,
            vrf_proof,
            chunk_endorsements,
        })
    }
}
