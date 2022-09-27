use crate::transaction::TransactionType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug, Serialize, Deserialize)]
#[serde(try_from = "TransactionRow")]
pub(crate) struct Transaction {
    pub(crate) transaction_type: TransactionType,
    pub account_id: u16,
    pub transaction_id: u32,
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
    client_id: u16,
    #[serde(rename = "tx")]
    transaction_id: u32,
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

struct RowParsingError(String);
impl fmt::Display for RowParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<TransactionRow> for Transaction {
    type Error = RowParsingError;

    fn try_from(row: TransactionRow) -> Result<Self, Self::Error> {
        let TransactionRow {
            transaction_type,
            client_id,
            transaction_id,
            amount,
        } = row;

        let amount_missing = format!(
            "'{:?}' transactions require a defined amount",
            transaction_type
        );
        let unknown_type = format!("'{:?}' is an unknown transaction type", transaction_type);

        let details = match transaction_type.as_str() {
            "deposit" => {
                let amount = amount.ok_or(RowParsingError(amount_missing))?;
                Deposit(amount)
            }
            "withdrawal" => {
                let amount = amount.ok_or(RowParsingError(amount_missing))?;
                Withdrawal(amount)
            }
            "dispute" => Dispute,
            "resolve" => Resolve,
            "chargeback" => Chargeback,
            _ => return Err(RowParsingError(unknown_type)),
        };
        Ok(Transaction {
            transaction_type: details,
            transaction_id,
            account_id: client_id,
        })
    }
}
