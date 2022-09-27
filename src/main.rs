mod account;
mod transaction;
mod ledger;

use crate::account::Account;
use crate::ledger::Ledger;
use crate::transaction::Transaction;
use csv::{ReaderBuilder, Trim};
use std::collections::HashMap;

fn main() {
    let data = "
type,client,tx,amount
deposit,1,1,1.0001
deposit, 2, 2, 2.1000
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0,
dispute, 2, 5";

    let mut csv = ReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All)
        .flexible(true)
        .from_reader(data.as_bytes());

    let mut ledger = Ledger::new();
    let accounts: &mut HashMap<u16, Account> = &mut HashMap::new();

    for row in csv.deserialize::<Transaction>() {
        let transaction = row.unwrap();
        let _ = ledger.process_transaction(accounts, transaction);
    }

    println!("client,available,held,total,locked");
    accounts.iter().for_each(|(client, account)| {
        println!("{client},{},{},{},{}", account.available(), account.held(), account.total(), account.locked);
    });
}
