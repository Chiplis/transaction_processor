mod account;
mod ledger;
mod transaction;

use crate::account::{Account, AccountId};
use crate::ledger::Ledger;
use crate::transaction::{Transaction};
use csv::{Reader, ReaderBuilder, Trim};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io::Read;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let args = &env::args().collect::<Vec<String>>();

    if args.len() != 2 {
        Err(format!(
            "Expected 1 argument for CSV input, got {}",
            args.len() - 1
        ))?
    }

    let path = &args[1];
    let csv = ReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All) // Supports arbitrary blank spaces between columns
        .flexible(true) // Allows parsing of differently sized rows
        .from_path(Path::new(path))?;

    let mut accounts = HashMap::new();
    let (accounts, errors) = process_csv(csv, &mut accounts);

    println!("client,available,held,total,locked");
    accounts.iter().for_each(|(account_id, account)| {
        println!(
            "{account_id},{},{},{},{}",
            account.available(),
            account.held(),
            account.total(),
            account.locked()
        );
    });
    for error in errors {
        eprintln!("{}", error);
    }
    Ok(())
}

// Traverses the specified CSV reader rows and returns the accounts HashMap modified according to all valid transactions
// Also returns an array containing all the errors (parsing and logical) found during the traversal
fn process_csv(
    mut csv: Reader<impl Read>,
    accounts: &mut HashMap<AccountId, Account>,
) -> (&HashMap<AccountId, Account>, Vec<Box<dyn Error>>) {
    let mut ledger = Ledger::new();
    let mut errors: Vec<Box<dyn Error>> = vec![];
    for row in csv.deserialize::<Transaction>() {
        match row {
            Err(error) => errors.push(Box::new(error)),
            Ok(transaction) => match ledger.process_transaction(accounts, transaction) {
                Ok(()) => (),
                Err(error) => errors.push(Box::new(error)),
            },
        }
    }
    (accounts, errors)
}

#[cfg(test)]
mod tests {
    use crate::{process_csv, AccountId};
    use csv::{ReaderBuilder, Trim};
    use std::collections::HashMap;
    use std::error::Error;
    use std::path::Path;
    use rust_decimal::Decimal;

    #[test]
    fn process_csv_parses_string_correctly() -> Result<(), Box<dyn Error>> {
        let csv = "type,client,tx,amount
                        deposit,1,1,1.0001
                        deposit, 2, 2, 2.1000
                        deposit, 1, 3, 2.0
                        withdrawal, 1, 4, 1.5
                        withdrawal, 2, 5, 3.0,
                        dispute, 2, 5,
                        dispute, 1, 1,
                        chargeback, 1, 1";
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(csv.as_bytes());

        let accounts = &mut HashMap::new();
        let (accounts, errors) = process_csv(csv, accounts);
        let (first_account, second_account) = (
            accounts.get(&AccountId(1)).unwrap(),
            accounts.get(&AccountId(2)).unwrap()
        );
        assert_eq!(first_account.total(), Decimal::from_str_exact("0.5")?);
        assert_eq!(second_account.total(), Decimal::from_str_exact("2.1")?);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].to_string(), "Transaction #5 for account #2 can't withdraw $3 due to insufficient funds");
        assert_eq!(errors[1].to_string(), "Transaction #5 not found");
        Ok(())
    }

    #[test]
    fn process_csv_parses_file_correctly() -> Result<(), Box<dyn Error>> {
        let csv = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .flexible(true)
            .from_path(Path::new("tests/basic.csv"))?;
        let accounts = &mut HashMap::new();
        let (accounts, errors) = process_csv(csv, accounts);
        let (first_account, second_account) = (
            accounts.get(&AccountId(1)).unwrap(),
            accounts.get(&AccountId(2)).unwrap()
        );
        assert_eq!(first_account.total(), Decimal::from_str_exact("1.5001")?);
        assert_eq!(second_account.total(), Decimal::from_str_exact("2.1")?);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].to_string(), "Transaction #5 for account #2 can't withdraw $3 due to insufficient funds");
        assert_eq!(errors[1].to_string(), "Transaction #5 not found");
        Ok(())
    }
}
