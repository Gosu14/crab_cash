use csv::Trim;
use serde::Deserialize;
use std::{collections::HashMap, env, error::Error, ffi::OsString, fs::File};

#[derive(Deserialize, Debug, Clone)]
struct InputRecord {
    #[serde(rename = "type")]
    typ: RecordType,
    client: u16,
    tx: u32,
    amount: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
enum RecordType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

// Client Account
struct Account {
    id: u16, // Unique
    amount_available: f64,
    amount_held: f64,
    is_locked: bool,
    tx: HashMap<u32, Transaction>,
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
        if self.is_locked {
            return;
        }
        self.amount_available += transaction.amount;
        self.tx.insert(transaction.id, transaction);
    }

    fn withdraw(&mut self, transaction: Transaction) {
        if self.is_locked {
            return;
        }
        if self.amount_available >= transaction.amount {
            self.amount_available -= transaction.amount;
            self.tx.insert(transaction.id, transaction);
        }
    }

    fn dispute(&mut self, tx_id: u32) {
        if self.is_locked {
            return;
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
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
        if self.is_locked {
            return;
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
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
        if self.is_locked {
            return;
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
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
    let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(file);

    // Create ledger hashmap
    let mut ledger: HashMap<u16, Account> = HashMap::new();

    // Looping over the record
    for result in rdr.deserialize::<InputRecord>() {
        let record = result.expect("a CSV record");

        let tx_type = record.typ;
        let client_id: u16 = record.client;
        let tx_id: u32 = record.tx;

        match tx_type {
            RecordType::Deposit => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                let tx_amount: f64 = record.amount.expect("Invalid Tx Amount");
                let transaction = Transaction {
                    typ: TransactionType::Deposit,
                    id: tx_id,
                    amount: tx_amount,
                    is_disputed: false,
                };

                account.deposit(transaction);
            }
            RecordType::Withdrawal => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                let tx_amount: f64 = record.amount.expect("Invalid Tx Amount");
                let transaction = Transaction {
                    typ: TransactionType::Withdrawal,
                    id: tx_id,
                    amount: tx_amount,
                    is_disputed: false,
                };

                account.withdraw(transaction);
            }
            RecordType::Dispute => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                account.dispute(tx_id);
            }
            RecordType::Resolve => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                account.resolve(tx_id);
            }
            RecordType::Chargeback => {
                let account = ledger
                    .entry(client_id)
                    .or_insert_with(|| Account::new(client_id));

                account.chargeback(tx_id);
            }
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

    #[test]
    fn test_max_withdrawal() {
        let mut ledger: HashMap<u16, Account> = HashMap::new();

        let account: &mut Account = ledger.entry(0).or_insert_with(|| Account::new(0));

        let deposit = Transaction {
            typ: TransactionType::Deposit,
            id: 0,
            amount: 100.0,
            is_disputed: false,
        };

        account.deposit(deposit);

        let withdrawal = Transaction {
            typ: TransactionType::Withdrawal,
            id: 1,
            amount: 100.0,
            is_disputed: false,
        };

        account.withdraw(withdrawal);

        // Verify client 1: deposit 100.0 + withdrawal 100.0 = 0.0
        let account1 = ledger.get(&0).expect("Client 1 should exist");
        assert_eq!(account1.amount_available, 0.0);
        assert_eq!(account1.amount_held, 0.0);
        assert!(!account1.is_locked);
    }

    #[test]
    fn test_that_account_is_locked_after_chargeback() {
        let mut ledger: HashMap<u16, Account> = HashMap::new();

        let account: &mut Account = ledger.entry(0).or_insert_with(|| Account::new(0));

        let deposit = Transaction {
            typ: TransactionType::Deposit,
            id: 0,
            amount: 100.0,
            is_disputed: false,
        };

        account.deposit(deposit);

        // First dispute the deposit
        account.dispute(0);

        // Verify that the deposit is under dispute
        let deposit = account.tx.get(&0).unwrap();
        assert!(deposit.is_disputed);

        // Then chargeback
        account.chargeback(0);

        // Verify that now the account is locked
        assert!(account.is_locked);

        // Try adding another deposit
        let deposit = Transaction {
            typ: TransactionType::Deposit,
            id: 1,
            amount: 200.0,
            is_disputed: false,
        };

        account.deposit(deposit);

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        let account1 = ledger.get(&0).expect("Client 1 should exist");
        assert_eq!(account1.amount_available, 0.0);
        assert_eq!(account1.amount_held, 0.0);
        assert!(account1.is_locked);
    }

    #[test]
    fn test_that_dispute_can_be_resolved() {
        let mut ledger: HashMap<u16, Account> = HashMap::new();

        let account: &mut Account = ledger.entry(0).or_insert_with(|| Account::new(0));

        let deposit = Transaction {
            typ: TransactionType::Deposit,
            id: 0,
            amount: 100.0,
            is_disputed: false,
        };

        account.deposit(deposit);

        // First dispute the deposit
        account.dispute(0);

        // Verify that the deposit is under dispute
        let deposit = account.tx.get(&0).unwrap();
        assert!(deposit.is_disputed);
        assert_eq!(account.amount_held, 100.0);
        assert_eq!(account.amount_available, 0.0);

        // Then resolve
        account.resolve(0);

        // Verify that now the account is locked
        assert!(!account.is_locked);
        assert_eq!(account.amount_held, 0.0);
        assert_eq!(account.amount_available, 100.0);

        // Try adding another deposit
        let deposit = Transaction {
            typ: TransactionType::Deposit,
            id: 1,
            amount: 200.0,
            is_disputed: false,
        };

        account.deposit(deposit);

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        let account1 = ledger.get(&0).expect("Client 1 should exist");
        assert_eq!(account1.amount_available, 300.0);
        assert_eq!(account1.amount_held, 0.0);
        assert!(!account1.is_locked);
    }

    #[test]
    fn test_that_dispute_on_withdrawhal_are_ignored() {
        let mut ledger: HashMap<u16, Account> = HashMap::new();

        let account: &mut Account = ledger.entry(0).or_insert_with(|| Account::new(0));

        let deposit = Transaction {
            typ: TransactionType::Deposit,
            id: 0,
            amount: 100.0,
            is_disputed: false,
        };

        account.deposit(deposit);

        let withdrawal = Transaction {
            typ: TransactionType::Withdrawal,
            id: 1,
            amount: 50.0,
            is_disputed: false,
        };

        account.withdraw(withdrawal);

        // First dispute the withdrawal
        account.dispute(1);

        // Verify that the deposit is under dispute
        let withdrawal = account.tx.get(&1).unwrap();
        assert!(!withdrawal.is_disputed);
        assert_eq!(account.amount_held, 0.0);
        assert_eq!(account.amount_available, 50.0);

        // Verify client 1: deposit 100.0 + withdrawal 50.0 + try dispute the withdrawal = 50.0
        let account1 = ledger.get(&0).expect("Client 1 should exist");
        assert_eq!(account1.amount_available, 50.0);
        assert_eq!(account1.amount_held, 0.0);
        assert!(!account1.is_locked);
    }
}
