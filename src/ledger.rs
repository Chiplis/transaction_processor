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

        if let Withdrawal(withdrawal) = transaction_type {
            return account.withdraw(*withdrawal);
        } else if let Deposit(deposit) = transaction_type {
            return account.deposit(*deposit);
        }

        let referenced_transaction_type = self
            .transactions
            .get(transaction_id)
            .map(|ref_tx| {
                if let Deposit(_) = ref_tx.transaction_type {
                    Ok(ref_tx)
                } else {
                    Err(InvalidTransactionReference(
                        *transaction_type,
                        ref_tx.transaction_type,
                    ))
                }
            })
            .unwrap_or(Err(NonExistentTransaction))?
            .transaction_type;

        self.handle_referenced_transaction(
            transaction_type,
            referenced_transaction_type,
            transaction_id,
            account,
        )?;

        self.transactions.insert(*transaction_id, transaction);
        Ok(())
    }

    fn handle_referenced_transaction(
        &mut self,
        original_transaction_type: &TransactionType,
        referenced_transaction_type: TransactionType,
        transaction_id: &TransactionId,
        account: &mut Account,
    ) -> TransactionResult {
        match (original_transaction_type, referenced_transaction_type) {
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
