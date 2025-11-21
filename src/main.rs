mod engine;

use csv::Trim;
use engine::{Account, InputRecord, Ledger, RecordType, Transaction, TransactionType};
use std::{env, error::Error, ffi::OsString, fs::File};

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

    for acc in ledger.accounts() {
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

fn process_transactions(file: File) -> Ledger {
    // Create a CSV parser that reads from the file
    let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(file);

    // Create ledger hashmap
    let mut ledger = Ledger::new();

    // Looping over the record
    for result in rdr.deserialize::<InputRecord>() {
        let record = result.expect("a CSV record");

        let tx_type: RecordType = record.typ;
        let client_id: u16 = record.client;
        let tx_id: u32 = record.tx;

        match tx_type {
            RecordType::Deposit => {
                let tx_amount: f64 = record.amount.expect("Invalid Tx Amount");
                let transaction = Transaction {
                    account_id: client_id,
                    typ: TransactionType::Deposit,
                    id: tx_id,
                    amount: tx_amount,
                    is_disputed: false,
                };

                ledger.process_transaction(transaction);
            }
            RecordType::Withdrawal => {
                let tx_amount: f64 = record.amount.expect("Invalid Tx Amount");
                let transaction = Transaction {
                    account_id: client_id,
                    typ: TransactionType::Withdrawal,
                    id: tx_id,
                    amount: tx_amount,
                    is_disputed: false,
                };

                ledger.process_transaction(transaction);
            }
            RecordType::Dispute => {
                let transaction = Transaction {
                    account_id: client_id,
                    typ: TransactionType::Dispute,
                    id: tx_id,
                    amount: 0.0,
                    is_disputed: false,
                };

                ledger.process_transaction(transaction);
            }
            RecordType::Resolve => {
                let transaction = Transaction {
                    account_id: client_id,
                    typ: TransactionType::Resolve,
                    id: tx_id,
                    amount: 0.0,
                    is_disputed: false,
                };

                ledger.process_transaction(transaction);
            }
            RecordType::Chargeback => {
                let transaction = Transaction {
                    account_id: client_id,
                    typ: TransactionType::Chargeback,
                    id: tx_id,
                    amount: 0.0,
                    is_disputed: false,
                };

                ledger.process_transaction(transaction);
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
        let account1 = ledger.get_account(1);
        assert_eq!(account1.amount_available, 1.5);
        assert_eq!(account1.amount_held, 0.0);
        assert!(!account1.is_locked);

        // Verify client 2: deposit 2.0 - withdrawal 3.0 (should fail) = 2.0
        let account2 = ledger.get_account(2);
        assert_eq!(account2.amount_available, 2.0);
        assert_eq!(account2.amount_held, 0.0);
        assert!(!account2.is_locked);
    }

    #[test]
    fn test_max_withdrawal() {
        let mut ledger = Ledger::new();

        let deposit = Transaction {
            account_id: 0,
            id: 0,
            amount: 100.0,
            typ: TransactionType::Deposit,
            is_disputed: false,
        };

        ledger.process_transaction(deposit);

        let withdrawal = Transaction {
            account_id: 0,
            id: 1,
            amount: 100.0,
            typ: TransactionType::Withdrawal,
            is_disputed: false,
        };

        ledger.process_transaction(withdrawal);

        // Verify client 1: deposit 100.0 + withdrawal 100.0 = 0.0
        let account1 = ledger.get_account(0);
        assert_eq!(account1.amount_available, 0.0);
        assert_eq!(account1.amount_held, 0.0);
        assert!(!account1.is_locked);
    }

    #[test]
    fn test_that_account_is_locked_after_chargeback() {
        let mut ledger = Ledger::new();

        let deposit = Transaction {
            account_id: 0,
            id: 0,
            amount: 100.0,
            typ: TransactionType::Deposit,
            is_disputed: false,
        };

        ledger.process_transaction(deposit);

        // First dispute the deposit
        let dispute = Transaction {
            account_id: 0,
            id: 0,
            amount: 0.0,
            typ: TransactionType::Dispute,
            is_disputed: false,
        };
        ledger.process_transaction(dispute);

        // Verify that the deposit is under dispute
        {
            let account = ledger.get_account(0);
            let disputed_tx = account.tx.get(&0).unwrap();
            assert!(disputed_tx.is_disputed);
        }

        // Then chargeback
        let chargeback = Transaction {
            account_id: 0,
            id: 0,
            amount: 0.0,
            typ: TransactionType::Chargeback,
            is_disputed: false,
        };
        ledger.process_transaction(chargeback);

        // Verify that now the account is locked
        let account = ledger.get_account(0);
        assert!(account.is_locked);

        // Try adding another deposit
        let deposit = Transaction {
            account_id: 0,
            id: 1,
            amount: 200.0,
            typ: TransactionType::Deposit,
            is_disputed: false,
        };

        ledger.process_transaction(deposit);

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        let account1 = ledger.get_account(0);
        assert_eq!(account1.amount_available, 0.0);
        assert_eq!(account1.amount_held, 0.0);
        assert!(account1.is_locked);
    }

    #[test]
    fn test_that_dispute_can_be_resolved() {
        let mut ledger = Ledger::new();

        let deposit = Transaction {
            account_id: 0,
            id: 0,
            amount: 100.0,
            typ: TransactionType::Deposit,
            is_disputed: false,
        };

        ledger.process_transaction(deposit);

        // First dispute the deposit
        let dispute = Transaction {
            account_id: 0,
            id: 0,
            amount: 0.0,
            typ: TransactionType::Dispute,
            is_disputed: false,
        };

        ledger.process_transaction(dispute);

        // Verify that the deposit is under dispute
        {
            let account = ledger.get_account(0);
            let deposit = account.tx.get(&0).unwrap();
            assert!(deposit.is_disputed);
            assert_eq!(account.amount_held, 100.0);
            assert_eq!(account.amount_available, 0.0);
        }

        // Then resolve
        let resolve = Transaction {
            account_id: 0,
            id: 0,
            amount: 0.0,
            typ: TransactionType::Resolve,
            is_disputed: false,
        };

        ledger.process_transaction(resolve);

        // Verify that now the account is locked
        {
            let account = ledger.get_account(0);
            assert!(!account.is_locked);
            assert_eq!(account.amount_held, 0.0);
            assert_eq!(account.amount_available, 100.0);
        }

        // Try adding another deposit
        let deposit = Transaction {
            account_id: 0,
            id: 1,
            amount: 200.0,
            typ: TransactionType::Deposit,
            is_disputed: false,
        };

        ledger.process_transaction(deposit);

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        let account = ledger.get_account(0);
        assert_eq!(account.amount_available, 300.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_dispute_on_withdrawhal_are_ignored() {
        let mut ledger = Ledger::new();

        let deposit = Transaction {
            account_id: 0,
            id: 0,
            amount: 100.0,
            typ: TransactionType::Deposit,
            is_disputed: false,
        };

        ledger.process_transaction(deposit);

        let withdrawal = Transaction {
            account_id: 0,
            id: 1,
            amount: 50.0,
            typ: TransactionType::Withdrawal,
            is_disputed: false,
        };

        ledger.process_transaction(withdrawal);

        // First dispute the withdrawal
        let dispute = Transaction {
            account_id: 0,
            id: 1,
            amount: 0.0,
            typ: TransactionType::Dispute,
            is_disputed: false,
        };

        ledger.process_transaction(dispute);

        // Verify that the deposit is under dispute
        let account = ledger.get_account(0);
        let withdrawal = account.tx.get(&1).unwrap();
        assert!(!withdrawal.is_disputed);
        assert_eq!(account.amount_held, 0.0);
        assert_eq!(account.amount_available, 50.0);

        // Verify client 1: deposit 100.0 + withdrawal 50.0 + try dispute the withdrawal = 50.0
        assert_eq!(account.amount_available, 50.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);
    }
}
