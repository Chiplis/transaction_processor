use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub(crate) struct AccountId(pub u16);

#[derive(Default, Debug)]
pub(crate) struct Account {
    available: Decimal,
    held: Decimal,
    locked: bool,
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

    pub fn withdraw(&mut self, withdrawed: Decimal) {
        self.available -= withdrawed;
    }

    pub fn dispute(&mut self, disputed: Decimal) {
        self.available -= disputed;
        self.held += disputed;
    }

    pub fn resolve(&mut self, resolved: Decimal) {
        self.available += resolved;
        self.held -= resolved;
    }

    pub fn chargeback(&mut self, charged_back: Decimal) {
        self.held -= charged_back;
        self.locked = true;
    }

    // Getters to deal with the required decimal precision when generating the output file

    pub fn total(&self) -> Decimal {
        (self.available + self.held).round_dp(4).normalize()
    }

    pub fn held(&self) -> Decimal {
        self.held.round_dp(4).normalize()
    }

    pub fn available(&self) -> Decimal {
        self.available.round_dp(4).normalize()
    }

    pub fn locked(&self) -> bool {
        self.locked
    }
}
