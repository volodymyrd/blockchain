use crate::hash::CryptoHash;
use borsh::{BorshDeserialize, BorshSerialize};
use near_crypto::PublicKey;
/// Reexport primitive types
pub use near_primitives_core::types::*;
use near_schema_checker_lib::ProtocolSchema;

/// Hash used by to store state root.
pub type StateRoot = CryptoHash;

/// Epoch identifier -- wrapped hash, to make it easier to distinguish.
/// EpochId of epoch T is the hash of last block in T-2
/// EpochId of first two epochs is 0
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Hash,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    derive_more::AsRef,
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
    arbitrary::Arbitrary,
)]
#[as_ref(forward)]
pub struct EpochId(pub CryptoHash);

pub mod validator_stake {
    pub use super::ValidatorStakeV1;
    use borsh::{BorshDeserialize, BorshSerialize};
    use serde::Serialize;

    /// Stores validator and its stake.
    #[derive(BorshSerialize, BorshDeserialize, Serialize, Debug, Clone, PartialEq, Eq)]
    #[serde(tag = "validator_stake_struct_version")]
    pub enum ValidatorStake {
        V1(ValidatorStakeV1),
    }
}

/// Stores validator and its stake.
#[derive(
    BorshSerialize, BorshDeserialize, serde::Serialize, Debug, Clone, PartialEq, Eq, ProtocolSchema,
)]
pub struct ValidatorStakeV1 {
    /// Account that stakes money.
    pub account_id: AccountId,
    /// Public key of the proposed validator.
    pub public_key: PublicKey,
    /// Stake / weight of the validator.
    pub stake: Balance,
}
