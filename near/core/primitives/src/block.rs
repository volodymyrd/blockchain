use crate::block_body::BlockBody;
pub use crate::block_header::*;
use crate::challenge::Challenges;
use crate::merkle::{merklize, verify_path, MerklePath};
use crate::sharding::{ChunkHashHeight, ShardChunkHeader};
use near_primitives_core::hash::CryptoHash;
use near_primitives_core::types::{Balance, BlockHeight, ProtocolVersion};
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

pub fn genesis_chunks() -> Vec<crate::sharding::ShardChunk> {
    let mut chunks = vec![];
    chunks
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
