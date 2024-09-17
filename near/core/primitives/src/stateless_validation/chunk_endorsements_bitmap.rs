use borsh::{BorshDeserialize, BorshSerialize};
use near_schema_checker_lib::ProtocolSchema;

/// Represents a collection of bitmaps, one per shard, to store whether the endorsements from the chunk validators has been received.
///
/// For each shard, the endorsements are encoded as a sequence of bits: 1 means endorsement received and 0 means not received.
/// While the number of chunk validator seats is fixed, the number of chunk-validator assignments may be smaller and may change,
/// since the seats are assigned to validators weighted by their stake. Thus, we represent the bits as a vector of bytes.
/// The number of assignments may be less or equal to the number of total bytes. This representation allows increasing
/// the chunk validator seats in the future (which will be represented by a vector of greater length).
#[derive(
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Default,
    ProtocolSchema,
)]
pub struct ChunkEndorsementsBitmap {
    inner: Vec<Vec<u8>>,
}

impl ChunkEndorsementsBitmap {
    /// Creates an empty endorsement bitmap for each shard.
    pub fn new(num_shards: usize) -> Self {
        Self {
            inner: vec![Default::default(); num_shards],
        }
    }

    /// Returns the bitmap specifically for the genesis block.
    /// This matches the chunk endorsement signatures in the genesis block body,
    /// which is an empty vector (no shards).
    pub fn genesis() -> Self {
        Self::new(0)
    }
}
