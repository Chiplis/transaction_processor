use crate::account::AccountId;
use crate::transaction::TransactionFailure::{
    InvalidTransactionReference, NonExistentAccount, NonExistentTransaction,
};
use crate::transaction::TransactionType::{Deposit, Withdrawal};
use crate::transaction::{TransactionId, TransactionResult, TransactionType};
use crate::{Account, Transaction};
use std::collections::HashMap;
use TransactionType::{Chargeback, Dispute, Resolve};

#[derive(Default)]
pub(crate) struct Ledger {
    regular_transactions: HashMap<TransactionId, TransactionType>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            ..Default::default()
        }
    }

    pub fn process_transaction(
        &mut self,
        accounts: &mut HashMap<AccountId, Account>,
        transaction: Transaction,
    ) -> TransactionResult {
        let Transaction {
            transaction_type,
            account_id,
            transaction_id,
        } = &transaction;

        // An account is "created" when a deposit is made into a new AccountId
        // Any other transaction appearing before the account creation should be considered invalid
        let account = match transaction_type {
            // Get the existing account or create a new one
            Deposit(_) => accounts.entry(*account_id).or_insert_with(Account::new),
            // Get the existing account or fail immediately
            Withdrawal(_) | Dispute | Resolve | Chargeback => accounts
                .get_mut(account_id)
                .ok_or(NonExistentAccount(*account_id))?,
        };

        let disputed_amount = self
            .regular_transactions
            .get(transaction_id) // Get the transaction type referenced by the dispute/resolve/chargeback
            .ok_or(NonExistentTransaction(*transaction_id))
            .and_then(|transaction| {
                if let Deposit(amount) = transaction {
                    Ok(amount)
                } else {
                    Err(InvalidTransactionReference(*transaction_type, *transaction))
                }
            });

        // Any errors that happen during the deposit/withdrawal halt the method execution
        // before we insert the processed transaction.
        match transaction_type {
            Deposit(deposit) => {
                account.deposit(*deposit)?;
                self.regular_transactions
                    .insert(*transaction_id, *transaction_type);
            }
            Withdrawal(withdrawal) => {
                account.withdraw(*account_id, *transaction_id, *withdrawal)?;
                self.regular_transactions
                    .insert(*transaction_id, *transaction_type);
            }
            Dispute => {
                account.dispute(*transaction_id, *disputed_amount?)?;
            }
            Resolve => {
                account.resolve(*transaction_id, *disputed_amount?)?;
            }
            Chargeback => {
                account.chargeback(*transaction_id, *disputed_amount?)?;
            }
        }

        Ok(())
    }
}
