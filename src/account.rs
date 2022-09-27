use rust_decimal::Decimal;
use crate::transaction::{TransactionResult, TransactionType};
use crate::transaction::TransactionFailure::InsufficientFunds;

#[derive(Default, Debug)]
pub(crate) struct Account {
    available: Decimal,
    held: Decimal,
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

    pub fn dispute(&mut self, disputed: Decimal) {
        self.available -= disputed;
        self.held += disputed;
    }

    pub fn resolve(&mut self, resolved: Decimal) {
        self.available += resolved;
        self.held -= resolved;
    }

    pub fn chargeback(&mut self, chargeback: Decimal) {
        self.held -= chargeback;
        self.locked = true;
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
