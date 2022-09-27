mod account;
mod ledger;
mod transaction;

use std::borrow::Borrow;
use crate::account::{Account, AccountId};
use crate::ledger::Ledger;
use crate::transaction::Transaction;
use csv::{ReaderBuilder, Trim};
use std::collections::HashMap;
use std::env;
use std::error::Error;
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
    let mut accounts = HashMap::new();
    let accounts = match process_csv(path, &mut accounts) {
        Ok(accounts) => Ok(accounts),
        Err(cause) => Err(format!("Unexpected error while attempting to parse CSV: {}", cause))
    }?;

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
    Ok(())
}

fn process_csv<'a>(path: &'a String, accounts: &'a mut HashMap<AccountId, Account>) -> Result<&'a HashMap<AccountId, Account>, Box<dyn Error>> {
    let mut csv = ReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All) // Supports arbitrary blank spaces between columns
        .flexible(true) // Allows parsing of differently sized rows
        .from_path(Path::new(&path))?;

    let mut ledger = Ledger::new();
    let mut rows: usize = 2; // Start counting from the 2nd line since we skip the headers
    for row in csv.deserialize::<Transaction>() {
        match row {
            Err(error) => eprintln!("Skipping row due to parsing error: {}", error),
            Ok(transaction) => match ledger.process_transaction(accounts, transaction) {
                Err(error) => eprintln!("Row #{} not processed: {:?}", rows, error),
                _ => (),
            }
        }
        rows += 1;
    }
    Ok(accounts)
}
