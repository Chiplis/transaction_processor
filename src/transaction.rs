use crate::account::AccountId;
use crate::transaction::TransactionType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use DepositState::Deposited;
use RowParsingError::{NegativeAmount, UndefinedAmount, UnknownTransactionType};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub(crate) struct TransactionId(u32);

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
    Deposit(Decimal, DepositState),
    Withdrawal(Decimal),
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum DepositState {
    Deposited,
    Disputed,
    Resolved,
    ChargedBack,
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
pub(crate) enum TransactionFailure {
    #[error("{1:?} for {0:?} can't withdraw ${2} due to insufficient funds")]
    InsufficientFunds(AccountId, TransactionId, Decimal),
    #[error("{0:?} not found")]
    NonExistentTransaction(TransactionId),
    #[error("{0:?} not found")]
    NonExistentAccount(AccountId),
    #[error("No previously disputed {0:?} found")]
    UndisputedTransaction(TransactionId),
    #[error("{0:?} cannot transition from {1:?} to {2:?}")]
    InvalidDepositTransition(TransactionId, DepositState, DepositState),
    // An invalid transaction reference happens if you attempt to dispute/resolve/chargeback a non-deposit transaction
    #[error("{1:?} cannot reference {0:?} which is a {2:?}")]
    InvalidTransactionReference(TransactionId, TransactionType, TransactionType),
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
    #[error("Transaction requires a positive amount but was {0}")]
    NegativeAmount(Decimal),
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
                Err(NegativeAmount(value))
            } else {
                Ok(value)
            }
        });

        let transaction_type = match transaction_type.as_str() {
            "deposit" => Deposit(sane_amount?, Deposited),
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
