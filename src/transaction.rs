use crate::account::AccountId;
use crate::transaction::TransactionType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use RowParsingError::{UndefinedAmount, UnknownTransactionType};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub(crate) struct TransactionId(u32);
impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(try_from = "TransactionRow")]
pub(crate) struct Transaction {
    pub(crate) transaction_type: TransactionType,
    pub account_id: AccountId,
    pub transaction_id: TransactionId,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub(crate) enum TransactionType {
    Deposit(Decimal),
    Withdrawal(Decimal),
    Dispute,
    Resolve,
    Chargeback,
}

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

pub(crate) enum TransactionFailure {
    InsufficientFunds,
    NonExistentTransaction,
    NonExistentAccount,
    UndisputedTransaction,
    RedisputedTransaction,
    FinalizedDispute,
    InvalidTransactionReference(TransactionType, TransactionType),
}

pub(crate) type TransactionResult = Result<(), TransactionFailure>;

enum RowParsingError {
    UnknownTransactionType(String),
    UndefinedAmount,
}
impl fmt::Display for RowParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UnknownTransactionType(unknown_type) => {
                write!(f, "{} is an unknown type", unknown_type)
            }
            UndefinedAmount => write!(f, "Transaction requires a defined amount"),
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

        let transaction_type = match transaction_type.as_str() {
            "deposit" => Deposit(amount.ok_or(UndefinedAmount)?),
            "withdrawal" => Withdrawal(amount.ok_or(UndefinedAmount)?),
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
