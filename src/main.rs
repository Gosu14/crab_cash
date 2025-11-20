use std::cell::RefCell;
use std::{collections::HashMap, env, error::Error, ffi::OsString, fs::File};

// Client Account
struct Account {
    id: u16, // Unique
    amount_available: f64,
    amount_held: f64,
    is_locked: bool,
    tx: HashMap<u32, RefCell<Transaction>>,
}

impl Account {
    fn new(client_id: u16) -> Self {
        Account {
            id: client_id,
            amount_available: 0.0,
            amount_held: 0.0,
            is_locked: false,
            tx: HashMap::new(),
        }
    }

    fn deposit(&mut self, transaction: Transaction) {
        self.amount_available += transaction.amount;
        self.tx.insert(transaction.id, RefCell::new(transaction));
    }

    fn withdraw(&mut self, transaction: Transaction) {
        if self.amount_available > transaction.amount {
            self.amount_available -= transaction.amount;
            self.tx.insert(transaction.id, RefCell::new(transaction));
        }
    }

    fn dispute(&mut self, tx_id: u32) {
        if let Some(tx_cell) = self.tx.get(&tx_id) {
            let mut tx = tx_cell.borrow_mut();

            if tx.is_disputed {
                return; // Already disputed -> ignored
            }

            match tx.typ {
                TransactionType::Deposit => {
                    // Hold the funds and keep the same total
                    self.amount_available -= tx.amount;
                    self.amount_held += tx.amount;
                }
                TransactionType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
            }

            tx.is_disputed = true;
        }
    }

    fn resolve(&mut self, tx_id: u32) {
        if let Some(tx_cell) = self.tx.get(&tx_id) {
            let mut tx = tx_cell.borrow_mut();

            if !tx.is_disputed {
                return;
            }

            match tx.typ {
                TransactionType::Deposit => {
                    // Release held funds back to available
                    self.amount_held -= tx.amount;
                    self.amount_available += tx.amount;
                }
                TransactionType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
            }

            tx.is_disputed = false;
        }
    }

    fn chargeback(&mut self, tx_id: u32) {
        if let Some(tx_cell) = self.tx.get(&tx_id) {
            let mut tx = tx_cell.borrow_mut();

            if !tx.is_disputed {
                return; // Not under dispute
            }

            match tx.typ {
                TransactionType::Deposit => {
                    // Remove held funds
                    self.amount_held -= tx.amount;
                }
                TransactionType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
            }

            self.is_locked = true;
            tx.is_disputed = false; // Dispute resolved via chargeback
        }
    }
}

struct Transaction {
    typ: TransactionType,
    id: u32,
    amount: f64,
    is_disputed: bool,
}

enum TransactionType {
    Deposit,
    Withdrawal,
}

fn main() {
    // Get filename as 1st argument
    let filepath = get_first_arg().expect("Unable to get first arg");

    // Open file
    let file = File::open(filepath).expect("Unable to open the File");

    // Process transactions
    let ledger = process_transactions(file);

    // Write in std::out
    let mut wtr = csv::Writer::from_writer(std::io::stdout());

    // Write header manually
    wtr.write_record(["client", "available", "held", "total", "locked"])
        .expect("Error writing headers");

    for acc in ledger.values() {
        wtr.write_record([
            acc.id.to_string(),
            acc.amount_available.to_string(),
            acc.amount_held.to_string(),
            (acc.amount_available + acc.amount_held).to_string(),
            acc.is_locked.to_string(),
        ])
        .expect("Error writing record");
    }

    wtr.flush().expect("Flushing Meadows");
}

// Returns the first positional argument sent to this process. If there are no
// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn process_transactions(file: File) -> HashMap<u16, Account> {
    // Create a CSV parser that reads from the file
    let mut rdr = csv::Reader::from_reader(file);

    // Create ledger hashmap
    let mut ledger: HashMap<u16, Account> = HashMap::new();

    // Looping over the record
    for result in rdr.records() {
        let record = result.expect("a CSV record");

        let tx_type = &record[0];
        let client_id: u16 = record[1].trim().parse().expect("Invalid client ID");
        let tx_id: u32 = record[2].trim().parse().expect("Invalid Tx ID");

        match tx_type {
            "deposit" => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                let tx_amount: f64 = record[3].trim().parse().expect("Invalid Tx Amount");
                let transaction = Transaction {
                    typ: TransactionType::Deposit,
                    id: tx_id,
                    amount: tx_amount,
                    is_disputed: false,
                };

                account.deposit(transaction);
            }
            "withdrawal" => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                let tx_amount: f64 = record[3].trim().parse().expect("Invalid Tx Amount");
                let transaction = Transaction {
                    typ: TransactionType::Withdrawal,
                    id: tx_id,
                    amount: tx_amount,
                    is_disputed: false,
                };

                account.withdraw(transaction);
            }
            "dispute" => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                account.dispute(tx_id);
            }
            "resolve" => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                account.resolve(tx_id);
            }
            "chargeback" => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                account.chargeback(tx_id);
            }
            _ => (),
        }
    }
    ledger
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_process_transactions_csv() {
        let file = File::open(Path::new("transactions.csv")).expect("Unable to open the File");

        let ledger = process_transactions(file);

        // Verify client 1: deposit 1.0 + deposit 2.0 - withdrawal 1.5 = 1.5
        let account1 = ledger.get(&1).expect("Client 1 should exist");
        assert_eq!(account1.amount_available, 1.5);
        assert_eq!(account1.amount_held, 0.0);
        assert!(!account1.is_locked);

        // Verify client 2: deposit 2.0 - withdrawal 3.0 (should fail) = 2.0
        let account2 = ledger.get(&2).expect("Client 2 should exist");
        assert_eq!(account2.amount_available, 2.0);
        assert_eq!(account2.amount_held, 0.0);
        assert!(!account2.is_locked);
    }
}
