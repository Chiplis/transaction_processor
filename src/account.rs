use crate::transaction::TransactionFailure::{
    FinalizedDispute, InsufficientFunds, RedisputedTransaction, UndisputedTransaction,
};
use crate::transaction::{TransactionId, TransactionResult};
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Formatter;
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub(crate) struct AccountId(pub u16);
impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Default, Debug)]
pub(crate) struct Account {
    available: Decimal,
    held: Decimal,
    past_disputes: HashSet<TransactionId>,
    finalized_disputes: HashSet<TransactionId>,
    pub(crate) locked: bool,
}

impl Account {
    pub(crate) fn new() -> Self {
        Account {
            ..Default::default()
        }
    }

    pub fn deposit(&mut self, deposit: Decimal) {
        self.available += deposit;
    }

    pub fn withdraw(&mut self, withdraw: Decimal) -> TransactionResult {
        if self.available() < withdraw {
            return Err(InsufficientFunds);
        }
        Ok(self.available -= withdraw)
    }

    pub fn dispute(&mut self, tx_id: TransactionId, disputed: Decimal) -> TransactionResult {
        if self.past_disputes.contains(&tx_id) {
            return Err(RedisputedTransaction);
        }
        self.available -= disputed;
        self.held += disputed;
        self.past_disputes.insert(tx_id);
        Ok(())
    }

    pub fn resolve(&mut self, tx_id: TransactionId, resolved: Decimal) -> TransactionResult {
        if self.finalized_disputes.contains(&tx_id) {
            return Err(FinalizedDispute);
        } else if !self.past_disputes.contains(&tx_id) {
            return Err(UndisputedTransaction);
        }
        self.available += resolved;
        self.held -= resolved;
        self.finalized_disputes.insert(tx_id);
        Ok(())
    }

    pub fn chargeback(&mut self, tx_id: TransactionId, chargeback: Decimal) -> TransactionResult {
        if self.finalized_disputes.contains(&tx_id) {
            return Err(FinalizedDispute);
        } else if !self.past_disputes.contains(&tx_id) {
            return Err(UndisputedTransaction);
        }
        self.held -= chargeback;
        self.locked = true;
        self.finalized_disputes.insert(tx_id);
        Ok(())
    }

    pub fn total(&self) -> Decimal {
        (self.available + self.held).round_dp(4).normalize()
    }

    pub fn held(&self) -> Decimal {
        self.held.round_dp(4).normalize()
    }

    pub fn available(&self) -> Decimal {
        self.available.round_dp(4).normalize()
    }
}
