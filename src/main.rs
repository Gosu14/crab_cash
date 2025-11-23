mod engine;

use csv::Trim;
use engine::{Amount, InputRecord, Ledger};
use simple_logger::SimpleLogger;
use std::{env, error::Error, ffi::OsString, fs::File};

fn main() {
    // Setup logger
    SimpleLogger::new().env().init().unwrap();

    log::debug!("Application started - Extracting filepath from args");

    // Get filename as 1st argument
    let filepath = get_first_arg().expect("Unable to get first arg");

    // Open file
    let file = File::open(filepath).expect("Unable to open the File");

    log::debug!("Transactions processing Starting");
    // Process transactions
    let ledger = process_transactions(file);

    log::debug!("Exporting account snapshots to stdout");
    // Write in std::out
    let mut wtr = csv::Writer::from_writer(std::io::stdout());

    // Write header manually
    wtr.write_record(["client", "available", "held", "total", "locked"])
        .expect("Error writing headers");

    for acc in ledger.account_snapshots() {
        wtr.serialize(acc).expect("Error writing record");
    }

    wtr.flush().expect("Flushing Meadows");

    log::debug!("Application finished - All transactions processed");
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
        let transaction = record.to_transaction();
        if let Err(e) = ledger.process_transaction(transaction) {
            log::warn!(
                "Error processing transaction id={} client={}: {}",
                record.tx,
                record.client,
                e
            );
        }
    }
    ledger
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::Path, str::FromStr};

    #[test]
    fn test_process_transactions_csv() {
        let file = File::open(Path::new("transactions.csv")).expect("Unable to open the File");

        let ledger = process_transactions(file);

        // Verify client 1: deposit 1.0 + deposit 2.0 - withdrawal 1.5 = 1.5
        let account1 = ledger.get_account(1);
        assert_eq!(account1.amount_available, Amount::from_str("1.5").unwrap());
        assert_eq!(account1.amount_held, Amount::new());
        assert!(!account1.is_locked);

        // Verify client 2: deposit 2.0 - withdrawal 3.0 (should fail) = 2.0
        let account2 = ledger.get_account(2);
        assert_eq!(account2.amount_available, Amount::from_str("2.0").unwrap());
        assert_eq!(account2.amount_held, Amount::new());
        assert!(!account2.is_locked);
    }
}
