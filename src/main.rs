mod account;
mod ledger;
mod transaction;

use crate::account::{Account, AccountId};
use crate::ledger::Ledger;
use crate::transaction::Transaction;
use anyhow::{bail, Error};
use csv::{Reader, ReaderBuilder, Trim};
use std::collections::HashMap;
use std::env;
use std::io::Read;
use std::path::Path;

fn main() -> Result<(), Error> {
    let args = &env::args().collect::<Vec<String>>();

    if args.len() != 2 {
        bail!("Expected 1 argument for CSV input, got {}", args.len() - 1);
    }

    let path = &args[1];
    let csv = ReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All) // Supports arbitrary blank spaces between columns
        .flexible(true) // Allows parsing of differently sized rows
        .from_path(Path::new(path))?;

    let (accounts, _) = process_csv(csv);

    println!("client,available,held,total,locked");
    accounts.iter().for_each(|(account_id, account)| {
        println!(
            "{},{},{},{},{}",
            account_id.0,
            account.available(),
            account.held(),
            account.total(),
            account.locked()
        );
    });
    Ok(())
}

// Traverses the specified CSV reader rows and returns the accounts HashMap modified according to all valid transactions
// Also returns an array containing all the errors (parsing and logical) found during the traversal
fn process_csv(mut csv: Reader<impl Read>) -> (HashMap<AccountId, Account>, Vec<Error>) {
    let mut ledger = Ledger::new();
    let mut errors: Vec<Error> = vec![];
    let mut accounts = HashMap::new();
    let mut process_row = |row| Ok(ledger.process_transaction(&mut accounts, row?)?);

    for row in csv.deserialize::<Transaction>() {
        if let Err(e) = process_row(row) {
            errors.push(e);
        }
    }
    (accounts, errors)
}

#[cfg(test)]
mod tests {
    use crate::{process_csv, AccountId};
    use csv::{ReaderBuilder, Trim};
    use rust_decimal::Decimal;

    use std::path::Path;

    #[test]
    fn processes_regular_transactions_correctly() {
        let csv = "type,client,tx,amount
                        deposit, 1, 1, 1
                        deposit, 1, 2, 1
                        withdrawal, 1, 3, 0.5";
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(csv.as_bytes());

        let (accounts, errors) = process_csv(csv);
        let first_account = accounts.get(&AccountId(1)).unwrap();
        assert_eq!(
            first_account.available(),
            Decimal::from_str_exact("1.5").unwrap()
        );
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn processes_dispute_correctly() {
        let csv = "type,client,tx,amount
                        deposit,1,1,1.0001
                        dispute, 1, 1";
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(csv.as_bytes());

        let (accounts, errors) = process_csv(csv);
        let first_account = accounts.get(&AccountId(1)).unwrap();
        assert_eq!(
            first_account.available(),
            Decimal::from_str_exact("0").unwrap()
        );
        assert_eq!(
            first_account.held(),
            Decimal::from_str_exact("1.0001").unwrap()
        );
        assert_eq!(first_account.held(), first_account.total());
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn processes_resolve_correctly() {
        let csv = "type,client,tx,amount
                        deposit,1,1,1.0001
                        dispute, 1, 1,
                        resolve, 1, 1";
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(csv.as_bytes());

        let (accounts, errors) = process_csv(csv);
        let first_account = accounts.get(&AccountId(1)).unwrap();
        assert_eq!(
            first_account.available(),
            Decimal::from_str_exact("1.0001").unwrap()
        );
        assert_eq!(first_account.held(), Decimal::from_str_exact("0").unwrap());
        assert_eq!(first_account.available(), first_account.total());
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn processes_chargeback_correctly() {
        let csv = "type,client,tx,amount
                        deposit,1,1,1.0001
                        dispute, 1, 1,
                        chargeback, 1, 1";
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(csv.as_bytes());

        let (accounts, errors) = process_csv(csv);
        let first_account = accounts.get(&AccountId(1)).unwrap();
        assert_eq!(
            first_account.available(),
            Decimal::from_str_exact("0").unwrap()
        );
        assert_eq!(first_account.held(), Decimal::from_str_exact("0").unwrap());
        assert!(first_account.locked());
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn process_csv_parses_file_correctly() {
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_path(Path::new("tests/basic.csv"))
            .unwrap();
        let (accounts, errors) = process_csv(csv);
        let (first_account, second_account) = (
            accounts.get(&AccountId(1)).unwrap(),
            accounts.get(&AccountId(2)).unwrap(),
        );
        assert_eq!(
            first_account.total(),
            Decimal::from_str_exact("1.5").unwrap()
        );
        assert_eq!(
            second_account.total(),
            Decimal::from_str_exact("2").unwrap()
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].to_string(),
            "TransactionId(5) for AccountId(2) can't withdraw $3 due to insufficient funds"
        );
    }

    #[test]
    fn parses_csv_with_logic_errors_correctly() {
        let csv = "type,client,tx,amount
                        deposit,1,1,1.0001
                        deposit,2,2,2.1000
                        deposit,1,3,2.0
                        withdrawal,1,4,1.5
                        withdrawal,2,5,3.0,
                        withdrawal,2,6,1.1,
                        withdrawal,3,7,1.1,
                        dispute,2,5,
                        dispute,2,6,
                        dispute,1,1,
                        resolve,1,1,
                        chargeback,1,1,
                        dispute,2,2";
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(csv.as_bytes());

        let (accounts, errors) = process_csv(csv);
        let (first_account, second_account) = (
            accounts.get(&AccountId(1)).unwrap(),
            accounts.get(&AccountId(2)).unwrap(),
        );
        assert_eq!(
            first_account.available(),
            Decimal::from_str_exact("1.5001").unwrap()
        );
        assert_eq!(
            second_account.held(),
            Decimal::from_str_exact("2.1").unwrap()
        );
        assert_eq!(
            second_account.available(),
            // At this point the user should have a negative available balance because they withdrew
            // some funds from the account and then reverted the original funding deposit
            Decimal::from_str_exact("-1.1").unwrap()
        );
        assert_eq!(errors.len(), 5);
        assert_eq!(
            errors[0].to_string(),
            "TransactionId(5) for AccountId(2) can't withdraw $3 due to insufficient funds"
        );
        assert_eq!(errors[1].to_string(), "AccountId(3) not found");
        assert_eq!(errors[2].to_string(), "TransactionId(5) not found");
        assert_eq!(
            errors[3].to_string(),
            "Dispute cannot reference TransactionId(6) which is a Withdrawal(1.1)"
        );
        assert_eq!(
            errors[4].to_string(),
            "TransactionId(1) cannot transition from Resolved to ChargedBack"
        );
    }

    #[test]
    fn parses_csv_with_parsing_errors_correctly() {
        let csv = "type,client,tx,amount
                        invalid,0
                        unknown,1,1
                        deposit,1,1,-1.001
                        deposit,1,1,
                        deposit,1,1,1.0001
                        deposit, 2, 2, 3.3";
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(csv.as_bytes());

        let (accounts, errors) = process_csv(csv);
        let (first_account, second_account) = (
            accounts.get(&AccountId(1)).unwrap(),
            accounts.get(&AccountId(2)).unwrap(),
        );
        assert_eq!(
            first_account.total(),
            Decimal::from_str_exact("1.0001").unwrap()
        );
        assert_eq!(
            second_account.total(),
            Decimal::from_str_exact("3.3").unwrap()
        );
        assert_eq!(errors.len(), 4);
        assert_eq!(
            errors[0].to_string(),
            "CSV deserialize error: record 1 (line: 2, byte: 22): expected field, but got end of row"
        );
        assert_eq!(
            errors[1].to_string(),
            "CSV deserialize error: record 2 (line: 3, byte: 56): unknown is an unknown type"
        );
        assert_eq!(
            errors[2].to_string(),
            "CSV deserialize error: record 3 (line: 4, byte: 92): Transaction requires a positive amount but was -1.001"
        );
        assert_eq!(
            errors[3].to_string(),
            "CSV deserialize error: record 4 (line: 5, byte: 135): Transaction requires a defined amount"
        );
    }
}
