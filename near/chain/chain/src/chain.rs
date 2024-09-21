use near_chain_primitives::Error;
use near_epoch_manager::EpochManagerAdapter;
use near_primitives::block::{genesis_chunks, Block};
use near_primitives::hash::CryptoHash;
use near_primitives::sharding::ShardChunk;

/// Facade to the blockchain block processing and storage.
/// Provides current view on the state according to the chain state.
pub struct Chain {}

impl Chain {
    /// Builds genesis block and chunks from the current configuration obtained through the arguments.
    pub fn make_genesis_block(
        epoch_manager: &dyn EpochManagerAdapter,
        runtime_adapter: &dyn RuntimeAdapter,
        chain_genesis: &ChainGenesis,
        state_roots: Vec<CryptoHash>,
    ) -> Result<(Block, Vec<ShardChunk>), Error> {
        let congestion_infos =
            get_genesis_congestion_infos(epoch_manager, runtime_adapter, &state_roots)?;
        let genesis_chunks = genesis_chunks(
            state_roots,
            congestion_infos,
            &epoch_manager.shard_ids(&EpochId::default())?,
            chain_genesis.gas_limit,
            chain_genesis.height,
            chain_genesis.protocol_version,
        );
        let genesis_block = Block::genesis(
            chain_genesis.protocol_version,
            genesis_chunks
                .iter()
                .map(|chunk| chunk.cloned_header())
                .collect(),
            chain_genesis.time,
            chain_genesis.height,
            chain_genesis.min_gas_price,
            chain_genesis.total_supply,
            Chain::compute_bp_hash(
                epoch_manager,
                EpochId::default(),
                EpochId::default(),
                &CryptoHash::default(),
            )?,
        );
        Ok((genesis_block, genesis_chunks))
    }
}
