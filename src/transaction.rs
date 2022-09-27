use std::error::Error;
use crate::account::AccountId;
use crate::transaction::TransactionType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use RowParsingError::{NegativeAmount, UndefinedAmount, UnknownTransactionType};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub(crate) struct TransactionId(u32);
impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// This struct defines all the fields we can find in the parsed CSV
#[derive(Debug, Serialize, Deserialize)]
struct TransactionRow {
    #[serde(rename = "type")]
    transaction_type: String,
    #[serde(rename = "client")]
    account_id: AccountId,
    #[serde(rename = "tx")]
    transaction_id: TransactionId,
    amount: Option<Decimal>,
}

// TransactionRow is converted into Transaction, which only contains fields available in every transaction type
#[derive(Debug, Serialize, Deserialize)]
#[serde(try_from = "TransactionRow")]
pub(crate) struct Transaction {
    pub(crate) transaction_type: TransactionType,
    pub account_id: AccountId,
    pub transaction_id: TransactionId,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub(crate) enum TransactionType {
    Deposit(Decimal),
    Withdrawal(Decimal),
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub(crate) enum TransactionFailure {
    InsufficientFunds(AccountId, TransactionId, Decimal),
    NonExistentTransaction(TransactionId),
    NonExistentAccount(AccountId),
    UndisputedTransaction(TransactionId),
    RedisputedTransaction(TransactionId),
    FinalizedDispute(TransactionId),
    // An invalid transaction reference happens if you attempt to dispute/resolve/chargeback a non-deposit transaction
    InvalidTransactionReference(TransactionType, TransactionType),
}

impl Display for TransactionFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TransactionFailure::InsufficientFunds(account_id, tx_id, amount) =>
                write!(f, "Transaction #{} for account #{} can't withdraw ${} due to insufficient funds", tx_id, account_id, amount),
            TransactionFailure::NonExistentTransaction(tx_id) =>
                write!(f, "Transaction #{} not found", tx_id),
            TransactionFailure::NonExistentAccount(account_id) =>
                write!(f, "Account #{} not found", account_id),
            TransactionFailure::UndisputedTransaction(tx_id) =>
                write!(f, "No previously disputed transaction #{} found", tx_id),
            TransactionFailure::RedisputedTransaction(tx_id) =>
                write!(f, "Transaction #{} has already been disputed", tx_id),
            TransactionFailure::FinalizedDispute(tx_id) =>
                write!(f, "Transaction #{} dispute has already ended", tx_id),
            TransactionFailure::InvalidTransactionReference(a, b) =>
                write!(f, "{:?} transaction cannot reference {:?}", a, b),
        }
    }
}

impl Error for TransactionFailure {}

// The result of a transaction is either an empty type, meaning the transaction completed successfully,
// or a particular transaction failure enum
pub(crate) type TransactionResult = Result<(), TransactionFailure>;

enum RowParsingError {
    UnknownTransactionType(String),
    UndefinedAmount,
    NegativeAmount,
}
impl fmt::Display for RowParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UnknownTransactionType(unknown_type) => {
                write!(f, "{} is an unknown type", unknown_type)
            }
            UndefinedAmount => write!(f, "Transaction requires a defined amount"),
            NegativeAmount => write!(f, "Transaction requires a positive amount"),
        }
    }
}

impl TryFrom<TransactionRow> for Transaction {
    type Error = RowParsingError;

    fn try_from(row: TransactionRow) -> Result<Self, Self::Error> {
        let TransactionRow {
            transaction_type,
            account_id,
            transaction_id,
            amount,
        } = row;

        let amount = match amount {
            Some(value) => {
                if value.is_sign_negative() {
                    Err(NegativeAmount)
                } else {
                    Ok(value)
                }
            }
            None => Err(UndefinedAmount),
        };

        let transaction_type = match transaction_type.as_str() {
            "deposit" => Deposit(amount?),
            "withdrawal" => Withdrawal(amount?),
            "dispute" => Dispute,
            "resolve" => Resolve,
            "chargeback" => Chargeback,
            unknown_type => return Err(UnknownTransactionType(unknown_type.to_string())),
        };
        Ok(Transaction {
            transaction_type,
            transaction_id,
            account_id,
        })
    }
}
