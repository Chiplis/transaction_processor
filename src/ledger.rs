use std::collections::{HashMap, HashSet};
use TransactionType::{Chargeback, Dispute, Resolve};
use crate::{Account, Transaction};
use crate::transaction::TransactionFailure::{AlreadyDisputedTransaction, InsufficientFunds, InvalidTransactionType, NonExistentAccount, NonExistentTransaction, UndisputedTransaction};
use crate::transaction::{TransactionFailure, TransactionResult, TransactionType};
use crate::transaction::TransactionType::{Deposit, Withdrawal};

#[derive(Default)]
pub(crate) struct Ledger {
    transactions: HashMap<u32, Transaction>,
    disputed: HashSet<u32>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            ..Default::default()
        }
    }

    pub fn process_transaction(
        &mut self,
        accounts: &mut HashMap<u16, Account>,
        transaction: Transaction,
    ) -> TransactionResult {
        let Transaction {
            transaction_type,
            account_id,
            transaction_id,
        } = &transaction;

        let referenced_tx = self.get_referenced_transaction(&transaction)?;

        let account = match transaction_type {
            Deposit(_) => accounts.entry(*account_id).or_insert(Account::new()),
            Withdrawal(_) | Dispute | Resolve | Chargeback => {
                accounts.get_mut(account_id).ok_or(NonExistentAccount)?
            }
        };

        self.apply_transaction_to_account(
            transaction_type,
            referenced_tx.transaction_type,
            transaction_id,
            account,
        )?;

        self.transactions.insert(*transaction_id, transaction);
        Ok(())
    }

    fn apply_transaction_to_account(
        &mut self,
        original_tx: &TransactionType,
        referenced_tx: TransactionType,
        transaction_id: &u32,
        account: &mut Account,
    ) -> TransactionResult {
        match (original_tx, referenced_tx) {
            (Withdrawal(withdrawal), _) => {
                account.withdraw(*withdrawal)
            }

            (Deposit(deposit), _) => Ok(account.deposit(*deposit)),

            (Dispute, Deposit(disputed_amount)) => {
                self.disputed.insert(*transaction_id);
                account.dispute(disputed_amount);
                Ok(())
            }

            (Resolve, Deposit(resolved_amount)) => {
                account.resolve(resolved_amount);
                Ok(())
            }

            (Chargeback, Deposit(chargeback_amount)) => {
                account.chargeback(chargeback_amount);
                Ok(())
            }

            (&original, referenced) => Err(InvalidTransactionType(original, referenced)),
        }
    }

    fn get_referenced_transaction<'a>(
        &'a self,
        original_transaction: &'a Transaction,
    ) -> Result<&Transaction, TransactionFailure> {
        let Transaction {
            transaction_type,
            transaction_id,
            ..
        } = &original_transaction;

        if let Dispute = transaction_type {
            self.transactions
                .get(transaction_id)
                .map(|tx| match &tx.transaction_type {
                    Deposit(_) => {
                        if self.disputed.contains(transaction_id) {
                            Ok(tx)
                        } else {
                            Err(AlreadyDisputedTransaction)
                        }
                    }
                    &invalid => Err(InvalidTransactionType(Dispute, invalid)),
                })
                .unwrap_or(Err(NonExistentTransaction))
        } else if let Chargeback | Resolve = transaction_type {
            self.transactions
                .get(transaction_id)
                .map(|tx| {
                    if self.disputed.contains(transaction_id) {
                        Ok(tx)
                    } else {
                        Err(UndisputedTransaction)
                    }
                })
                .unwrap_or(Err(NonExistentTransaction))
        } else {
            Ok(original_transaction)
        }
    }
}