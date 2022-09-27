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
    transaction_types: HashMap<TransactionId, TransactionType>,
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
            Deposit(_) => accounts.entry(*account_id).or_insert(Account::new()),
            // Get the existing account or fail immediately
            Withdrawal(_) | Dispute | Resolve | Chargeback => {
                accounts.get_mut(account_id).ok_or(NonExistentAccount)?
            }
        };

        // If we're dealing with a deposit or a withdrawal, we don't need to reference a
        // previous transaction, so we can return early from the method call.
        // Any errors that happen during the deposit/withdrawal halt the method execution
        // before we insert the processed transaction.
        if let Deposit(deposit) = transaction_type {
            account.deposit(*deposit)?;
            self.transaction_types
                .insert(*transaction_id, *transaction_type);
            return Ok(());
        } else if let Withdrawal(withdrawal) = transaction_type {
            account.withdraw(*withdrawal)?;
            self.transaction_types
                .insert(*transaction_id, *transaction_type);
            return Ok(());
        }

        // By this point we know the transaction type we're processing is either a dispute, resolve or chargeback
        let referenced_deposit = self
            .transaction_types
            .get(transaction_id) // Get the transaction type referenced by the dispute/resolve/chargeback
            .map(|referenced_type| {
                if let Deposit(_) = referenced_type {
                    // The referenced transaction type can only be a deposit
                    Ok(referenced_type)
                } else {
                    Err(InvalidTransactionReference(
                        *transaction_type, // Dispute/Resolve/Chargeback
                        *referenced_type,  // A non-deposit transaction type
                    ))
                }
            })
            .unwrap_or(Err(NonExistentTransaction))?;

        self.handle_referential_transaction(
            transaction_type,
            *referenced_deposit,
            transaction_id,
            account,
        )?;

        self.transaction_types
            .insert(*transaction_id, *transaction_type);
        Ok(())
    }

    fn handle_referential_transaction(
        &self,
        referential_transaction_type: &TransactionType, // Dispute/Resolve/Chargeback
        referenced_deposit: TransactionType,            // Deposit
        transaction_id: &TransactionId,
        account: &mut Account,
    ) -> TransactionResult {
        // The referenced transaction should always be a deposit,
        // while the way the account state is modified depends on the referential transaction
        match (referential_transaction_type, referenced_deposit) {
            (Dispute, Deposit(disputed_amount)) => {
                account.dispute(*transaction_id, disputed_amount)
            }

            (Resolve, Deposit(resolved_amount)) => {
                account.resolve(*transaction_id, resolved_amount)
            }

            (Chargeback, Deposit(chargeback_amount)) => {
                account.chargeback(*transaction_id, chargeback_amount)
            }

            // Any invalid combination of referential/reference transactions
            // should've been dealt with before the method call
            _ => unreachable!("Any invalid combination of referential/reference transactions should've been dealt with before calling this method"),
        }
    }
}
