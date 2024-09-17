use crate::block::{Approval, ApprovalInner, BlockHeader};
use crate::sharding::ChunkHash;
use near_crypto::{Signature, Signer};
use std::fmt::Debug;
use std::sync::Arc;

use crate::types::{AccountId, BlockHeight, EpochId};

/// Enum for validator signer, that holds validator id and key used for signing data.
#[derive(Clone, Debug, PartialEq)]
pub enum ValidatorSigner {
    /// Dummy validator signer, does not hold a key. Use for tests only!
    Empty(EmptyValidatorSigner),
    /// Default validator signer that holds data in memory.
    InMemory(InMemoryValidatorSigner),
}

/// Signer that keeps secret key in memory and signs locally.
#[derive(Clone, Debug, PartialEq)]
pub struct InMemoryValidatorSigner {
    account_id: AccountId,
    signer: Arc<Signer>,
}

impl InMemoryValidatorSigner {
    pub fn validator_id(&self) -> &AccountId {
        &self.account_id
    }

    fn sign_chunk_hash(&self, chunk_hash: &ChunkHash) -> Signature {
        self.signer.sign(chunk_hash.as_ref())
    }

    fn sign_approval(&self, inner: &ApprovalInner, target_height: BlockHeight) -> Signature {
        self.signer
            .sign(&Approval::get_data_for_sig(inner, target_height))
    }
}
/// Test-only signer that "signs" everything with 0s.
/// Don't use in any production or code that requires signature verification.
#[derive(smart_default::SmartDefault, Clone, Debug, PartialEq)]
pub struct EmptyValidatorSigner {
    #[default("test".parse().unwrap())]
    account_id: AccountId,
}

impl EmptyValidatorSigner {
    pub fn new(account_id: AccountId) -> ValidatorSigner {
        ValidatorSigner::Empty(Self { account_id })
    }

    fn validator_id(&self) -> &AccountId {
        &self.account_id
    }
    fn sign_chunk_hash(&self, _chunk_hash: &ChunkHash) -> Signature {
        Signature::default()
    }

    fn sign_approval(&self, _inner: &ApprovalInner, _target_height: BlockHeight) -> Signature {
        Signature::default()
    }
}

/// Validator signer that is used to sign blocks and approvals.
impl ValidatorSigner {
    /// Account id of the given validator.
    pub fn validator_id(&self) -> &AccountId {
        match self {
            ValidatorSigner::Empty(signer) => signer.validator_id(),
            ValidatorSigner::InMemory(signer) => signer.validator_id(),
        }
    }

    /// Signs given inner of the chunk header.
    pub fn sign_chunk_hash(&self, chunk_hash: &ChunkHash) -> Signature {
        match self {
            ValidatorSigner::Empty(signer) => signer.sign_chunk_hash(chunk_hash),
            ValidatorSigner::InMemory(signer) => signer.sign_chunk_hash(chunk_hash),
        }
    }

    /// Signs approval of given parent hash and reference hash.
    pub fn sign_approval(&self, inner: &ApprovalInner, target_height: BlockHeight) -> Signature {
        match self {
            ValidatorSigner::Empty(signer) => signer.sign_approval(inner, target_height),
            ValidatorSigner::InMemory(signer) => signer.sign_approval(inner, target_height),
        }
    }
}
