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
    transactions: HashMap<TransactionId, Transaction>,
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

        let account = match transaction_type {
            Deposit(_) => accounts.entry(*account_id).or_insert(Account::new()),
            Withdrawal(_) | Dispute | Resolve | Chargeback => {
                accounts.get_mut(account_id).ok_or(NonExistentAccount)?
            }
        };

        let referenced_tx = if let Chargeback | Resolve | Dispute = transaction_type {
            self.transactions
                .get(transaction_id)
                .map(|tx| match &tx.transaction_type {
                    Deposit(_) => Ok(tx),
                    &invalid => Err(InvalidTransactionReference(Dispute, invalid)),
                })
                .unwrap_or(Err(NonExistentTransaction))
        } else {
            Ok(&transaction)
        }?;

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
        transaction_id: &TransactionId,
        account: &mut Account,
    ) -> TransactionResult {
        match (original_tx, referenced_tx) {
            (Withdrawal(withdrawal), _) => account.withdraw(*withdrawal),

            (Deposit(deposit), _) => Ok(account.deposit(*deposit)),

            (Dispute, Deposit(disputed_amount)) => {
                account.dispute(*transaction_id, disputed_amount)
            }

            (Resolve, Deposit(resolved_amount)) => {
                account.resolve(*transaction_id, resolved_amount)
            }

            (Chargeback, Deposit(chargeback_amount)) => {
                account.chargeback(*transaction_id, chargeback_amount)
            }

            (&original, referenced) => Err(InvalidTransactionReference(original, referenced)),
        }
    }
}
