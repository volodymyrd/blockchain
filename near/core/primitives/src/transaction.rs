pub use crate::action::{Action, CreateAccountAction};
use crate::hash::{hash, CryptoHash};
use crate::types::{AccountId, Nonce};
use borsh::{BorshDeserialize, BorshSerialize};
use near_crypto::{PublicKey, Signature};
use near_schema_checker_lib::ProtocolSchema;
use std::io::{Error, ErrorKind, Read, Write};

#[derive(BorshSerialize, BorshDeserialize, Eq, Debug, Clone, ProtocolSchema)]
#[borsh(init=init)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Signature,
    #[borsh(skip)]
    hash: CryptoHash,
    #[borsh(skip)]
    size: u64,
}

impl SignedTransaction {
    pub fn new(signature: Signature, transaction: Transaction) -> Self {
        let mut signed_tx = Self {
            signature,
            transaction,
            hash: CryptoHash::default(),
            size: u64::default(),
        };
        signed_tx.init();
        signed_tx
    }

    pub fn init(&mut self) {
        let (hash, size) = self.transaction.get_hash_and_size();
        self.hash = hash;
        self.size = size;
    }
}
impl PartialEq for SignedTransaction {
    fn eq(&self, other: &SignedTransaction) -> bool {
        self.hash == other.hash && self.signature == other.signature
    }
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone, ProtocolSchema)]
pub struct TransactionV1 {
    /// An account on which behalf transaction is signed
    pub signer_id: AccountId,
    /// A public key of the access key which was used to sign an account.
    /// Access key holds permissions for calling certain kinds of actions.
    pub public_key: PublicKey,
    /// Nonce is used to determine order of transaction in the pool.
    /// It increments for a combination of `signer_id` and `public_key`
    pub nonce: Nonce,
    /// Receiver account for this transaction
    pub receiver_id: AccountId,
    /// The hash of the block in the blockchain on top of which the given transaction is valid
    pub block_hash: CryptoHash,
    /// A list of actions to be applied
    pub actions: Vec<Action>,
    /// Priority fee. Unit is 10^12 yotcoNEAR
    pub priority_fee: u64,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Transaction {
    V1(TransactionV1),
}

impl Transaction {
    /// Computes a hash of the transaction for signing and size of serialized transaction
    pub fn get_hash_and_size(&self) -> (CryptoHash, u64) {
        let bytes = borsh::to_vec(&self).expect("Failed to deserialize");
        (hash(&bytes), bytes.len() as u64)
    }
}

impl BorshSerialize for Transaction {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            Transaction::V1(tx) => {
                BorshSerialize::serialize(&1_u8, writer)?;
                tx.serialize(writer)?;
            }
        }
        Ok(())
    }
}

impl BorshDeserialize for Transaction {
    /// Deserialize based on the first and second bytes of the stream. For V0, we do backward compatible deserialization by deserializing
    /// the entire stream into V0. For V1, we consume the first byte and then deserialize the rest.
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let u1 = u8::deserialize_reader(reader)?;
        let u2 = u8::deserialize_reader(reader)?;
        let u3 = u8::deserialize_reader(reader)?;
        let u4 = u8::deserialize_reader(reader)?;
        // This is a ridiculous hackery: because the first field in `TransactionV0` is an `AccountId`
        // and an account id is at most 64 bytes, for all valid `TransactionV0` the second byte must be 0
        // because of the littel endian encoding of the length of the account id.
        // On the other hand, for `TransactionV1`, since the first byte is 1 and an account id must have nonzero
        // length, so the second byte must not be zero. Therefore, we can distinguish between the two versions
        // by looking at the second byte.

        let read_signer_id = |buf: [u8; 4], reader: &mut R| -> std::io::Result<AccountId> {
            let str_len = u32::from_le_bytes(buf);
            let mut str_vec = Vec::with_capacity(str_len as usize);
            for _ in 0..str_len {
                str_vec.push(u8::deserialize_reader(reader)?);
            }
            AccountId::try_from(String::from_utf8(str_vec).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    "Failed to parse AccountId from bytes",
                )
            })?)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))
        };

        let u5 = u8::deserialize_reader(reader)?;
        let signer_id = read_signer_id([u2, u3, u4, u5], reader)?;
        let public_key = PublicKey::deserialize_reader(reader)?;
        let nonce = Nonce::deserialize_reader(reader)?;
        let receiver_id = AccountId::deserialize_reader(reader)?;
        let block_hash = CryptoHash::deserialize_reader(reader)?;
        let actions = Vec::<Action>::deserialize_reader(reader)?;
        let priority_fee = u64::deserialize_reader(reader)?;
        Ok(Transaction::V1(TransactionV1 {
            signer_id,
            public_key,
            nonce,
            receiver_id,
            block_hash,
            actions,
            priority_fee,
        }))
    }
}
