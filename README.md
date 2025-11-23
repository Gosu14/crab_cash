# 🦀💸🦀 Crab Cash

**Crab Cash** is a small payment engine written in Rust that manages transactions (deposit, withdrawal, dispute, resolve, chargeback) for multiples clients.

## 📑 Table of Contents

- 🧭 [Usage](#usage)
- 🧩 [Business rules and constraints](#business-rules-and-constraints)
- ⚖️ [Assumption and trade-offs](#assumption-and-trade-offs)
- 🔧 [How it works](#how-it-works)
- 🤖 [Use of AI](#use-of-ai)
- 🚀 [Potential improvements](#potential-improvements)
- 🎬 [That's all folks](#thats-all-folks)

## 🧭 Usage

1. To build and run :

```
cargo run -- transactions.csv
```

2. The engine reads the input CSV, processes each transaction, and writes account snapshots to stdout as CSV:

```
client,available,held,total,locked
1,1.5000,0.0000,1.5000,false
2,2.0000,0.0000,2.0000,false
```
3. Run tests (unit + integration):

```
cargo test
```

4. To write to a file:

```
cargo run -- transactions.csv > accounts.csv
```

## 🧩 Business rules and constraints

I have implemented the following business rules in the payment system:

- No overdrafts.
- No operations on locked accounts.
- Fixed 4-decimal precision.
- Operation on unknown transaction are ignored.
- Transaction IDs are globally unique and if reused, transaction will be ignored.
- Chargeback and resolve on unknown or undisputed transaction are ignored

## ⚖️ Assumption and trade-offs

In order to implement those business rules I had to make the following assumptions:

- **Withdrawals can't be disputed** : Money comes in only with deposit and no money can be held in a withdrawal dispute. In reality, most of dispute are on payment someone should not have received (i.e. deposit). Dispute/Resolve/Chargeback on withdrawals will be ignored with a warning.

- **Transaction amount can't be negative**: Wether for a withdrawal or a deposit, the transaction amount should always be positive. If a transaction amount is negative therefore this transaction will be rejected with a warning.

- **Transaction IDs are globally unique**: If a transaction ID is reused for another transaction wether the for the same client or not, it will be ignored with a warning. Failed, rejected and ignored transactions are not considered processed - Only the successful transactions will have the IDs "seen" - therefore would potentially be re-playable.

- **Malformed transaction are ignored**: When reading the inputs and parsing the transactions, if a record is malformed it will be ignored with a warning.

- **Amount limitations**: To represent the amount of an asset held, I have build the struct Amount. However, in order to avoid any floating point rounding error and to be mindful of the memory footprint (compared to BigNumber crates), it is represented with an `i64`. To enable a 4-decimal precision: `std::i64::MIN` / 10_000 < Amount < `std::i64::MAX` / 10_000. Overflow and Underflow are checked and will return errors.

When using Crab Cash, keep in mind the following limitations and risks:

- **Single Threaded**: Crab cash is single threaded and therefore most of the data structure are not thread-safe.

- **In memory-processing**: As any transactions can be disputed, until the stdout flush at the end, all data structure are held in memory, therefore, for large CSV the system can run out of memory.

- **No checkpointing**: There are no state checkpointing in external data storage during processing. If the application panics, then you will have to re-process the whole file again.

## 🔧 How it works

At a high level, the engine:

1. **Reads and parses the CSV**
   - [`src/main.rs`](./src/main.rs) reads the CSV file path from the command line.
   - It uses `csv::Reader` to read each row and deserialize it into:
     - [`InputRecord`](./src/engine/record.rs), which matches the CSV schema (`type, client, tx, amount`).
   - `InputRecord::to_transaction` converts each record into a typed:
     - [`Transaction`](./src/engine/transaction.rs) with:
       - `account_id` (client)
       - `id` (tx)
       - `amount` (optional string)
       - `typ` (`Deposit`, `Withdrawal`, `Dispute`, `Resolve`, `Chargeback`)

2. **Converts amounts safely**
   - Monetary values use the [`Amount`](./src/engine/amount.rs) type instead of floats.
   - `Amount` is an `i64` scaled by `10_000` (4 decimal places).
   - `Amount::add` and `Amount::sub` check for overflow/underflow and return errors if they occur.
   - `FromStr for Amount` parses strings like `5`, `5.1`, `5.1234`, `.05` and rejects invalid formats.

3. **Applies account-level logic**
   - Each client is represented by an [`Account`](./src/engine/account.rs) with:
     - `amount_available`
     - `amount_held`
     - `is_locked`
   - The account stores a small history of its own transactions to support disputes.
   - The main methods:
     - `deposit(tx_id, amount)`
     - `withdraw(tx_id, amount)`
     - `dispute(tx_id)`
     - `resolve(tx_id)`
     - `chargeback(tx_id)`
   - These methods enforce the business rules and return `AccountOperationError` when something is invalid (e.g. overdraft, unknown tx, double dispute, operations on a locked account).

4. **Coordinates everything in the ledger**
   - [`Ledger`](./src/engine/ledger.rs) keeps:
     - A map of client id → `Account`.
     - A global set of processed transaction ids to enforce uniqueness.
   - `Ledger::process_transaction`:
     - Ensures an account exists for the client.
     - Validates the transaction (amount presence, parsing, non-negative, no duplicate id).
     - Delegates to the right `Account` method.
   - `Ledger::account_snapshots` produces [`AccountSnapshot`](./src/engine/account_snapshot.rs) values:
     - `client`
     - `available`
     - `held`
     - `total` (available + held, checked for overflow)
     - `locked`

5. **Writes snapshots as CSV**
   - `write_to_std_out` in [`src/main.rs`](./src/main.rs):
     - Iterates over `ledger.account_snapshots()`.
     - Serializes each [`AccountSnapshot`](./src/engine/account_snapshot.rs) with `csv::Writer` to `stdout`.
     - You can redirect this output to a file:  
       `cargo run -- transactions.csv > accounts.csv`

## 🤖 Use of AI

My friend Claude was used for:
- Rubber ducking - Discussing assumption and trade-off.
- Tests coverage - Check coverage and missing edge-cases.
- Help with documentation - Docs and readme.
- Suggest small refactoring - On Rust idioms.

## 🚀 Potential improvements

- **Improve encapsulation**: Push the encapsulation further with a struct for Client IDs (u16 is a bit small) and Transaction IDs, to future proof the API.

- **Improve logging and observability**: Add structured error types for CSV parsing errors and provide clearer CLI error messages.

- **Improve performance and robustness**: Support streaming from multiple sources with a multi-threaded MPSC streaming pipeline. Add the ability to leverage large caching system (i.e. Redis) to avoid keeping the whole state in memory. Add a checkpointing mechanism to be able to recover from crash.

## 🎬 That's all folks
I hope you enjoy playing with Crab Cash and feel free to leave some feedback ;)