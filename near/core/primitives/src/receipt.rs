use crate::action::Action;
use crate::hash::CryptoHash;
use crate::serialize::dec_format;
use crate::types::{AccountId, Balance};
use borsh::{BorshDeserialize, BorshSerialize};
use near_crypto::PublicKey;
use near_fmt::AbbrBytes;
use near_schema_checker_lib::ProtocolSchema;
use serde_with::base64::Base64;
use serde_with::serde_as;
use std::io::Read;
use std::io::{Error, ErrorKind};
use std::{fmt, io};

/// The outgoing (egress) data which will be transformed
/// to a `DataReceipt` to be sent to a `receipt.receiver`
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Hash,
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct DataReceiver {
    pub data_id: CryptoHash,
    pub receiver_id: AccountId,
}

/// ActionReceipt is derived from an Action from `Transaction or from Receipt`
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ActionReceipt {
    /// A signer of the original transaction
    pub signer_id: AccountId,
    /// An access key which was used to sign the original transaction
    pub signer_public_key: PublicKey,
    /// A gas_price which has been used to buy gas in the original transaction
    #[serde(with = "dec_format")]
    pub gas_price: Balance,
    /// If present, where to route the output data
    pub output_data_receivers: Vec<DataReceiver>,
    /// A list of the input data dependencies for this Receipt to process.
    /// If all `input_data_ids` for this receipt are delivered to the account
    /// that means we have all the `ReceivedData` input which will be than converted to a
    /// `PromiseResult::Successful(value)` or `PromiseResult::Failed`
    /// depending on `ReceivedData` is `Some(_)` or `None`
    pub input_data_ids: Vec<CryptoHash>,
    /// A list of actions to process when all input_data_ids are filled
    pub actions: Vec<Action>,
}

/// An incoming (ingress) `DataReceipt` which is going to a Receipt's `receiver` input_data_ids
/// Which will be converted to `PromiseResult::Successful(value)` or `PromiseResult::Failed`
#[serde_as]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Hash,
    PartialEq,
    Eq,
    Clone,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct DataReceipt {
    pub data_id: CryptoHash,
    #[serde_as(as = "Option<Base64>")]
    pub data: Option<Vec<u8>>,
}

impl fmt::Debug for DataReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataReceipt")
            .field("data_id", &self.data_id)
            .field("data", &format_args!("{}", AbbrBytes(self.data.as_deref())))
            .finish()
    }
}

/// Receipt could be either ActionReceipt or DataReceipt
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ReceiptEnum {
    Action(ActionReceipt),
    Data(DataReceipt),
    PromiseYield(ActionReceipt),
    PromiseResume(DataReceipt),
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    ProtocolSchema,
)]
pub struct ReceiptV1 {
    /// An issuer account_id of a particular receipt.
    /// `predecessor_id` could be either `Transaction` `signer_id` or intermediate contract's `account_id`.
    pub predecessor_id: AccountId,
    /// `receiver_id` is a receipt destination.
    pub receiver_id: AccountId,
    /// An unique id for the receipt
    pub receipt_id: CryptoHash,
    /// A receipt type
    pub receipt: ReceiptEnum,
    /// Priority of a receipt
    pub priority: u64,
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize, ProtocolSchema)]
#[serde(untagged)]
pub enum Receipt {
    V1(ReceiptV1),
}

impl BorshSerialize for Receipt {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            Receipt::V1(receipt) => {
                BorshSerialize::serialize(&1_u8, writer)?;
                receipt.serialize(writer)
            }
        }
    }
}

impl BorshDeserialize for Receipt {
    /// Deserialize based on the first and second bytes of the stream. For V0, we do backward compatible deserialization by deserializing
    /// the entire stream into V0. For V1, we consume the first byte and then deserialize the rest.
    fn deserialize_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let u2 = u8::deserialize_reader(reader)?;
        let u3 = u8::deserialize_reader(reader)?;
        let u4 = u8::deserialize_reader(reader)?;
        // This is a ridiculous hackery: because the first field in `ReceiptV0` is an `AccountId`
        // and an account id is at most 64 bytes, for all valid `ReceiptV0` the second byte must be 0
        // because of the littel endian encoding of the length of the account id.
        // On the other hand, for `ReceiptV0`, since the first byte is 1 and an account id must have nonzero
        // length, so the second byte must not be zero. Therefore, we can distinguish between the two versions
        // by looking at the second byte.

        let read_predecessor_id = |buf: [u8; 4], reader: &mut R| -> io::Result<AccountId> {
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
        let signer_id = read_predecessor_id([u2, u3, u4, u5], reader)?;
        let receiver_id = AccountId::deserialize_reader(reader)?;
        let receipt_id = CryptoHash::deserialize_reader(reader)?;
        let receipt = ReceiptEnum::deserialize_reader(reader)?;
        let priority = u64::deserialize_reader(reader)?;
        Ok(Receipt::V1(ReceiptV1 {
            predecessor_id: signer_id,
            receiver_id,
            receipt_id,
            receipt,
            priority,
        }))
    }
}
