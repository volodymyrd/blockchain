const SECP256K1_SIGNATURE_LENGTH: usize = 65;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Secp256K1Signature([u8; SECP256K1_SIGNATURE_LENGTH]);

#[derive(Clone, PartialEq, Eq)]
pub enum Signature {
    ED25519(ed25519_dalek::Signature),
    SECP256K1(Secp256K1Signature),
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct ED25519PublicKey(pub [u8; ed25519_dalek::PUBLIC_KEY_LENGTH]);

impl TryFrom<&[u8]> for ED25519PublicKey {
    type Error = crate::errors::ParseKeyError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        data.try_into()
            .map(Self)
            .map_err(|_| Self::Error::InvalidLength {
                expected_length: ed25519_dalek::PUBLIC_KEY_LENGTH,
                received_length: data.len(),
            })
    }
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Secp256K1PublicKey([u8; 64]);

/// Public key container supporting different curves.
#[derive(Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum PublicKey {
    /// 256 bit elliptic curve based public-key.
    ED25519(ED25519PublicKey),
    /// 512 bit elliptic curve based public-key used in Bitcoin's public-key cryptography.
    SECP256K1(Secp256K1PublicKey),
}

#[derive(Clone, Eq)]
// This is actually a keypair, because ed25519_dalek api only has keypair.sign
// From ed25519_dalek doc: The first SECRET_KEY_LENGTH of bytes is the SecretKey
// The last PUBLIC_KEY_LENGTH of bytes is the public key, in total it's KEYPAIR_LENGTH
pub struct ED25519SecretKey(pub [u8; ed25519_dalek::KEYPAIR_LENGTH]);

/// Secret key container supporting different curves.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum SecretKey {
    ED25519(ED25519SecretKey),
    SECP256K1(secp256k1::SecretKey),
}
