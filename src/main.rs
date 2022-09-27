mod account;
mod ledger;
mod transaction;

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
        Err(format!("Expected 1 argument for CSV input, got {}", args.len() - 1))?
    }

    let mut csv = ReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All)
        .flexible(true)
        .from_path(Path::new(&args[1]))?;

    let mut ledger = Ledger::new();
    let accounts: &mut HashMap<AccountId, Account> = &mut HashMap::new();

    for row in csv.deserialize::<Transaction>() {
        let transaction = row.unwrap();
        let _ = ledger.process_transaction(accounts, transaction);
    }

    println!("client,available,held,total,locked");
    accounts.iter().for_each(|(client, account)| {
        println!(
            "{client},{},{},{},{}",
            account.available(),
            account.held(),
            account.total(),
            account.locked()
        );
    });
    Ok(())
}
