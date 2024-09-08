use near_account_id::AccountId;
use near_primitives_core::hash::CryptoHash;

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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Receipt {
    V1(ReceiptV1),
}
