mod engine;

use csv::Trim;
use engine::{Amount, InputRecord, Ledger};
use simple_logger::SimpleLogger;
use std::path::PathBuf;
use std::{env, error::Error, ffi::OsString, fs::File};

fn main() -> Result<(), Box<dyn Error>> {
    // Setup logger
    SimpleLogger::new().env().init()?;

    log::debug!("Application started");

    // Process transactions
    log::debug!("Transactions processing: Starting");
    let ledger = process_transactions()?;
    log::debug!("Transactions processing: Done");

    // Write in std::out
    log::debug!("Exporting account snapshots to stdout: Started");
    write_to_std_out(&ledger)?;
    log::debug!("Exporting account snapshots to stdout: Done");

    log::debug!("Application finished");

    Ok(())
}

// Returns the first positional argument sent to this process. If there are no
// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn process_transactions() -> Result<Ledger, Box<dyn Error>> {
    // Get filename as 1st argument
    let file_path = get_first_arg()?;
    let path = PathBuf::from(file_path);
    log::debug!("Extracted filepath fom args: {path:?}");

    process_transactions_from_filepath(&path)
}

fn process_transactions_from_filepath(filepath: &PathBuf) -> Result<Ledger, Box<dyn Error>> {
    // Open file
    let file: File = File::open(filepath)?;

    // Create a CSV parser that reads from the file
    let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(file);

    // Create ledger
    let mut ledger = Ledger::new();

    // Looping over the record
    log::debug!("Started deserialising records");
    for result in rdr.deserialize::<InputRecord>() {
        log::debug!("Deserialising record into InputRecord: {result:?}");
        let record = result?;
        log::debug!("Converting InputRecord into Transaction: {record:?}");
        let transaction = record.to_transaction();
        log::debug!("Processing transaction in ledger: {transaction:?}");
        if let Err(e) = ledger.process_transaction(transaction) {
            log::warn!(
                "Error processing transaction id={} client={}: {}",
                record.tx,
                record.client,
                e
            );
        }
    }
    Ok(ledger)
}

pub fn write_to_std_out(ledger: &Ledger) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());

    // Write header manually
    wtr.write_record(["client", "available", "held", "total", "locked"])?;

    log::debug!("Starting account snapshot serialisation");
    for acc in ledger.account_snapshots() {
        log::debug!("Serialising account snapshot: {acc:?}");
        wtr.serialize(acc)?;
    }

    log::debug!("Account snapshot serialisation done -> Flushing to stdout");
    wtr.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_process_transactions_csv() {
        let ledger =
            process_transactions_from_filepath(&PathBuf::from("transactions.csv")).unwrap();

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
