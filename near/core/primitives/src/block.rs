use crate::block_body::BlockBody;
pub use crate::block_header::*;
use crate::challenge::Challenges;
use crate::hash::CryptoHash;
use crate::merkle::{merklize, MerklePath};
use crate::sharding::{ChunkHashHeight, ShardChunkHeader};
use crate::types::{Balance, BlockHeight, EpochId, Gas};
use crate::version::{ProtocolVersion, SHARD_CHUNK_HEADER_UPGRADE_VERSION};
use near_time::Utc;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BlockV4 {
    pub header: BlockHeader,
    pub body: BlockBody,
}

/// Versioned Block data structure.
/// For each next version, document what are the changes between versions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Block {
    BlockV4(Arc<BlockV4>),
}

type ShardChunkReedSolomon = reed_solomon_erasure::galois_8::ReedSolomon;

pub fn genesis_chunks(
    state_roots: Vec<crate::types::StateRoot>,
    congestion_infos: Vec<Option<crate::congestion_info::CongestionInfo>>,
    shard_ids: &[crate::types::ShardId],
    initial_gas_limit: Gas,
    genesis_height: BlockHeight,
    genesis_protocol_version: ProtocolVersion,
) -> Vec<crate::sharding::ShardChunk> {
    let rs = ShardChunkReedSolomon::new(1, 2).unwrap();
    let state_roots = if state_roots.len() == shard_ids.len() {
        state_roots
    } else {
        assert_eq!(state_roots.len(), 1);
        std::iter::repeat(state_roots[0])
            .take(shard_ids.len())
            .collect()
    };

    let mut chunks = vec![];

    let num = shard_ids.len();
    assert_eq!(state_roots.len(), num);

    for shard_id in 0..num {
        let state_root = state_roots[shard_id];
        let congestion_info = congestion_infos[shard_id];
        let shard_id = shard_id as crate::types::ShardId;

        let encoded_chunk = genesis_chunk(
            &rs,
            genesis_protocol_version,
            genesis_height,
            initial_gas_limit,
            shard_id,
            state_root,
            congestion_info,
        );
        let mut chunk = encoded_chunk
            .decode_chunk(1)
            .expect("Failed to decode genesis chunk");
        chunk.set_height_included(genesis_height);
        chunks.push(chunk);
    }

    chunks
}

fn genesis_chunk(
    rs: &ShardChunkReedSolomon,
    genesis_protocol_version: u32,
    genesis_height: u64,
    initial_gas_limit: u64,
    shard_id: u64,
    state_root: CryptoHash,
    congestion_info: Option<crate::congestion_info::CongestionInfo>,
) -> crate::sharding::EncodedShardChunk {
    let (encoded_chunk, _) = crate::sharding::EncodedShardChunk::new(
        CryptoHash::default(),
        state_root,
        CryptoHash::default(),
        genesis_height,
        shard_id,
        rs,
        0,
        initial_gas_limit,
        0,
        CryptoHash::default(),
        vec![],
        vec![],
        &[],
        CryptoHash::default(),
        congestion_info,
        &crate::validator_signer::EmptyValidatorSigner::default().into(),
        genesis_protocol_version,
    )
    .expect("Failed to decode genesis chunk");
    encoded_chunk
}

impl Block {
    fn block_from_protocol_version(header: BlockHeader, body: BlockBody) -> Block {
        Block::BlockV4(Arc::new(BlockV4 { header, body }))
    }
    /// Returns genesis block for given genesis date and state root.
    pub fn genesis(
        genesis_protocol_version: ProtocolVersion,
        chunks: Vec<ShardChunkHeader>,
        timestamp: Utc,
        height: BlockHeight,
        initial_gas_price: Balance,
        initial_total_supply: Balance,
        next_bp_hash: CryptoHash,
    ) -> Self {
        let challenges = vec![];
        let chunk_endorsements = vec![];
        for chunk in &chunks {
            assert_eq!(chunk.height_included(), height);
        }
        let vrf_value = near_crypto::vrf::Value([0; 32]);
        let vrf_proof = near_crypto::vrf::Proof([0; 64]);
        let body = BlockBody::new(
            genesis_protocol_version,
            chunks,
            challenges,
            vrf_value,
            vrf_proof,
            chunk_endorsements,
        );
        let header = BlockHeader::genesis(
            genesis_protocol_version,
            height,
            Block::compute_state_root(body.chunks()),
            body.compute_hash(),
            Block::compute_chunk_prev_outgoing_receipts_root(body.chunks()),
            Block::compute_chunk_headers_root(body.chunks()).0,
            Block::compute_chunk_tx_root(body.chunks()),
            body.chunks().len() as u64,
            Block::compute_challenges_root(body.challenges()),
            timestamp,
            initial_gas_price,
            initial_total_supply,
            next_bp_hash,
        );

        Self::block_from_protocol_version(header, body)
    }

    pub fn compute_state_root<'a, T: IntoIterator<Item = &'a ShardChunkHeader>>(
        chunks: T,
    ) -> CryptoHash {
        merklize(
            &chunks
                .into_iter()
                .map(|chunk| chunk.prev_state_root())
                .collect::<Vec<CryptoHash>>(),
        )
        .0
    }

    pub fn compute_chunk_prev_outgoing_receipts_root<
        'a,
        T: IntoIterator<Item = &'a ShardChunkHeader>,
    >(
        chunks: T,
    ) -> CryptoHash {
        merklize(
            &chunks
                .into_iter()
                .map(|chunk| chunk.prev_outgoing_receipts_root())
                .collect::<Vec<CryptoHash>>(),
        )
        .0
    }

    pub fn compute_chunk_headers_root<'a, T: IntoIterator<Item = &'a ShardChunkHeader>>(
        chunks: T,
    ) -> (CryptoHash, Vec<MerklePath>) {
        merklize(
            &chunks
                .into_iter()
                .map(|chunk| ChunkHashHeight(chunk.chunk_hash(), chunk.height_included()))
                .collect::<Vec<ChunkHashHeight>>(),
        )
    }

    pub fn compute_chunk_tx_root<'a, T: IntoIterator<Item = &'a ShardChunkHeader>>(
        chunks: T,
    ) -> CryptoHash {
        merklize(
            &chunks
                .into_iter()
                .map(|chunk| chunk.tx_root())
                .collect::<Vec<CryptoHash>>(),
        )
        .0
    }

    pub fn compute_challenges_root(challenges: &Challenges) -> CryptoHash {
        merklize(
            &challenges
                .iter()
                .map(|challenge| challenge.hash)
                .collect::<Vec<CryptoHash>>(),
        )
        .0
    }
}
