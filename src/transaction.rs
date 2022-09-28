use crate::account::AccountId;
use crate::transaction::TransactionType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};
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

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
pub(crate) enum TransactionFailure {
    #[error("Transaction #{1} for account #{0} can't withdraw ${2} due to insufficient funds")]
    InsufficientFunds(AccountId, TransactionId, Decimal),
    #[error("Transaction #{0} not found")]
    NonExistentTransaction(TransactionId),
    #[error("Account #{0} not found")]
    NonExistentAccount(AccountId),
    #[error("No previously disputed transaction #{0} found")]
    UndisputedTransaction(TransactionId),
    #[error("Transaction #{0} has already been disputed")]
    RedisputedTransaction(TransactionId),
    #[error("Transaction #{0} dispute has already ended")]
    FinalizedDispute(TransactionId),
    // An invalid transaction reference happens if you attempt to dispute/resolve/chargeback a non-deposit transaction
    #[error("{0:?} transaction cannot reference {1:?}")]
    InvalidTransactionReference(TransactionType, TransactionType),
}

// The result of a transaction is either an empty type, meaning the transaction completed successfully,
// or a particular transaction failure enum
pub(crate) type TransactionResult = Result<(), TransactionFailure>;

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
enum RowParsingError {
    #[error("{0} is an unknown type")]
    UnknownTransactionType(String),
    #[error("Transaction requires a defined amount")]
    UndefinedAmount,
    #[error("Transaction requires a positive amount")]
    NegativeAmount,
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

        let sane_amount = amount.ok_or(UndefinedAmount).and_then(|value| {
            if value.is_sign_negative() {
                Err(NegativeAmount)
            } else {
                Ok(value)
            }
        });

        let transaction_type = match transaction_type.as_str() {
            "deposit" => Deposit(sane_amount?),
            "withdrawal" => Withdrawal(sane_amount?),
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
