use near_account_id::AccountId;
use near_crypto::PublicKey;
use near_primitives_core::hash::CryptoHash;
use near_primitives_core::types::Balance;

/// Epoch identifier -- wrapped hash, to make it easier to distinguish.
/// EpochId of epoch T is the hash of last block in T-2
/// EpochId of first two epochs is 0
#[derive(Debug, Clone, Copy, Default, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[as_ref(forward)]
pub struct EpochId(pub CryptoHash);

pub mod validator_stake {

    pub use super::ValidatorStakeV1;
    /// Stores validator and its stake.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[serde(tag = "validator_stake_struct_version")]
    pub enum ValidatorStake {
        V1(ValidatorStakeV1),
    }
}

/// Stores validator and its stake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorStakeV1 {
    /// Account that stakes money.
    pub account_id: AccountId,
    /// Public key of the proposed validator.
    pub public_key: PublicKey,
    /// Stake / weight of the validator.
    pub stake: Balance,
}
