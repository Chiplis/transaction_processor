use crate::account::AccountId;
use crate::transaction::DepositState::{ChargedBack, Resolved};
use crate::transaction::TransactionFailure::{
    InsufficientFunds, InvalidDepositTransition, InvalidTransactionReference, NonExistentAccount,
    NonExistentTransaction,
};
use crate::transaction::TransactionType::{Deposit, Withdrawal};
use crate::transaction::{DepositState, TransactionId, TransactionResult, TransactionType};
use crate::{Account, Transaction};
use rust_decimal::Decimal;
use std::collections::HashMap;
use DepositState::{Deposited, Disputed};
use TransactionType::{Chargeback, Dispute, Resolve};

#[derive(Default)]
pub(crate) struct Ledger {
    transactions: HashMap<TransactionId, TransactionType>,
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
            Deposit(..) => accounts.entry(*account_id).or_insert_with(Account::new),
            // Get the existing account or fail immediately
            Withdrawal(_) | Dispute | Resolve | Chargeback => accounts
                .get_mut(account_id)
                .ok_or(NonExistentAccount(*account_id))?,
        };

        let mut handle_dispute =
            |expected: DepositState, new: DepositState, operation: fn(&mut Account, Decimal)| {
                return self.handle_referential_transaction(
                    account,
                    *transaction_id,
                    *transaction_type,
                    expected,
                    new,
                    operation,
                );
            };

        match transaction_type {
            // No need to check the state of the deposit since it comes from the CSV
            Deposit(deposit, _) => {
                account.deposit(*deposit);
                self.transactions.insert(*transaction_id, *transaction_type);
                Ok(())
            }
            // A withdrawal can fail if the user tries to withdraw more funds than they have available
            Withdrawal(withdrawal) => {
                if account.available() < *withdrawal {
                    return Err(InsufficientFunds(*account_id, *transaction_id, *withdrawal));
                }
                account.withdraw(*withdrawal);
                self.transactions.insert(*transaction_id, *transaction_type);
                Ok(())
            }
            // Disputes can only be triggered once
            Dispute => handle_dispute(Deposited, Disputed, Account::dispute),
            // Resolves/Chargebacks can only be triggered on non-finalized transactions, and require a previous dispute to exist
            Resolve => handle_dispute(Disputed, Resolved, Account::resolve),
            Chargeback => handle_dispute(Disputed, ChargedBack, Account::chargeback),
        }
    }

    // When dealing with disputes, resolves and chargebacks verify the referenced transaction is
    // a deposit and in a valid state before adding it to the ledger
    fn handle_referential_transaction(
        &mut self,
        account: &mut Account,
        transaction_id: TransactionId,
        transaction_type: TransactionType,
        expected_state: DepositState,
        new_state: DepositState,
        operation: fn(&mut Account, Decimal),
    ) -> TransactionResult {
        let reference = self.transactions.get(&transaction_id);
        match reference {
            Some(Deposit(amount, state)) if *state == expected_state => {
                operation(account, *amount);
                self.transactions
                    .insert(transaction_id, Deposit(*amount, new_state));
                Ok(())
            }
            Some(Deposit(_, previous_state)) => Err(InvalidDepositTransition(
                transaction_id,
                *previous_state,
                new_state,
            )),
            Some(invalid_reference) => Err(InvalidTransactionReference(
                transaction_id,
                transaction_type,
                *invalid_reference,
            )),
            None => Err(NonExistentTransaction(transaction_id)),
        }
    }
}
