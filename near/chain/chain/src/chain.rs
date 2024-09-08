use near_primitives::block::{genesis_chunks, Block};
use near_primitives::sharding::ShardChunk;

/// Facade to the blockchain block processing and storage.
/// Provides current view on the state according to the chain state.
pub struct Chain {}

impl Chain {
    /// Builds genesis block and chunks from the current configuration obtained through the arguments.
    pub fn make_genesis_block() -> Result<(Block, Vec<ShardChunk>), Error> {
        let genesis_chunks = genesis_chunks();

        let genesis_block = Block::genesis();

        Ok((genesis_block, genesis_chunks))
    }
}
