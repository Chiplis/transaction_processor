**Transaction Processor**

This is a relatively simple Rust program which can analyze a CSV file containing different types of transactions and
outputs a list of accounts along with their funds' status.

The code takes advantage of Rust's powerful type system to prevent potential parsing errors at compile time.
The CSV is processed using a best effort strategy, meaning anomalies such as invalid/incorrect data do not cause a crash 
but instead get collected into a vector of errors which can later be analyzed and debugged.

**Run instructions**

`cargo run --release -- {CSV_PATH}`

**Testing**

`cargo test`

All tests work through the entire flow of the application by using mocked CSVs with different input data to make sure
both "happy path" and edge cases are handled correctly. A conscious decision was made to focus more on
these type of integration tests instead of simply unit testing each individual file. This is mainly due to the simplicity
of the system, having relatively few moving parts (with most of the logic concentrated in the `ledger.rs` file) means
integration testing allows us to test the whole logic of the program in a quick and easy way.