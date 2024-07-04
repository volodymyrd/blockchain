//! Abstractions and implementations for transaction signers.

use {
    crate::{
        derivation_path::DerivationPath,
        signature::{PresignerError, Signature},
        transaction::TransactionError,
    },
    std::{
        error,
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        path::Path,
    },
    thiserror::Error,
};
use my_solana_program::pubkey::Pubkey;

pub mod keypair;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SignerError {
    #[error("keypair-pubkey mismatch")]
    KeypairPubkeyMismatch,

    #[error("not enough signers")]
    NotEnoughSigners,

    #[error("transaction error")]
    TransactionError(#[from] TransactionError),

    #[error("custom error: {0}")]
    Custom(String),

    // Presigner-specific Errors
    #[error("presigner error")]
    PresignerError(#[from] PresignerError),

    // Remote Keypair-specific Errors
    #[error("connection error: {0}")]
    Connection(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("no device found")]
    NoDeviceFound,

    #[error("{0}")]
    Protocol(String),

    #[error("{0}")]
    UserCancel(String),

    #[error("too many signers")]
    TooManySigners,
}

/// The `Signer` trait declares operations that all digital signature providers
/// must support. It is the primary interface by which signers are specified in
/// `Transaction` signing interfaces
pub trait Signer {
    /// Infallibly gets the implementor's public key. Returns the all-zeros
    /// `Pubkey` if the implementor has none.
    fn pubkey(&self) -> Pubkey {
        self.try_pubkey().unwrap_or_default()
    }
    /// Fallibly gets the implementor's public key
    fn try_pubkey(&self) -> Result<Pubkey, SignerError>;
    /// Infallibly produces an Ed25519 signature over the provided `message`
    /// bytes. Returns the all-zeros `Signature` if signing is not possible.
    fn sign_message(&self, message: &[u8]) -> Signature {
        self.try_sign_message(message).unwrap_or_default()
    }
    /// Fallibly produces an Ed25519 signature over the provided `message` bytes.
    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError>;
    /// Whether the implementation requires user interaction to sign
    fn is_interactive(&self) -> bool;
}

/// The `EncodableKey` trait defines the interface by which cryptographic keys/keypairs are read,
/// written, and derived from sources.
pub trait EncodableKey: Sized {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Box<dyn error::Error>>;
    fn read_from_file<F: AsRef<Path>>(path: F) -> Result<Self, Box<dyn error::Error>> {
        let mut file = File::open(path.as_ref())?;
        Self::read(&mut file)
    }
    fn write<W: Write>(&self, writer: &mut W) -> Result<String, Box<dyn error::Error>>;
    fn write_to_file<F: AsRef<Path>>(&self, outfile: F) -> Result<String, Box<dyn error::Error>> {
        let outfile = outfile.as_ref();

        if let Some(outdir) = outfile.parent() {
            fs::create_dir_all(outdir)?;
        }

        let mut f = {
            #[cfg(not(unix))]
            {
                OpenOptions::new()
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                OpenOptions::new().mode(0o600)
            }
        }
        .write(true)
        .truncate(true)
        .create(true)
        .open(outfile)?;

        self.write(&mut f)
    }
}

/// The `SeedDerivable` trait defines the interface by which cryptographic keys/keypairs are
/// derived from byte seeds, derivation paths, and passphrases.
pub trait SeedDerivable: Sized {
    fn from_seed(seed: &[u8]) -> Result<Self, Box<dyn error::Error>>;
    fn from_seed_and_derivation_path(
        seed: &[u8],
        derivation_path: Option<DerivationPath>,
    ) -> Result<Self, Box<dyn error::Error>>;
    fn from_seed_phrase_and_passphrase(
        seed_phrase: &str,
        passphrase: &str,
    ) -> Result<Self, Box<dyn error::Error>>;
}

/// The `EncodableKeypair` trait extends `EncodableKey` for asymmetric keypairs, i.e. have
/// associated public keys.
pub trait EncodableKeypair: EncodableKey {
    type Pubkey: ToString;

    /// Returns an encodable representation of the associated public key.
    fn encodable_pubkey(&self) -> Self::Pubkey;
}
