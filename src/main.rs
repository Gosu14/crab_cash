mod engine;

use csv::Trim;
use engine::{InputRecord, Ledger};
use simple_logger::SimpleLogger;
use std::path::PathBuf;
use std::{env, error::Error, ffi::OsString, fs::File};

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().env().init()?;

    log::debug!("Application started");

    log::debug!("Transactions processing: Starting");
    let ledger = process_transactions()?;
    log::debug!("Transactions processing: Done");

    log::debug!("Exporting account snapshots to stdout: Started");
    write_to_std_out(&ledger)?;
    log::debug!("Exporting account snapshots to stdout: Done");

    log::debug!("Application finished");

    Ok(())
}

fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn process_transactions() -> Result<Ledger, Box<dyn Error>> {
    let file_path = get_first_arg()?;
    let path = PathBuf::from(file_path);
    log::debug!("Extracted filepath fom args: {path:?}");

    process_transactions_from_filepath(&path)
}

fn process_transactions_from_filepath(filepath: &PathBuf) -> Result<Ledger, Box<dyn Error>> {
    let file: File = File::open(filepath)?;

    let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(file);

    let mut ledger = Ledger::new();

    log::debug!("Started deserialising records");
    for result in rdr.deserialize::<InputRecord>() {
        log::debug!("Deserialising record into InputRecord: {result:?}");
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Error deserializing record:{e}");
                continue;
            }
        };
        log::debug!("Converting InputRecord into Transaction: {record:?}");
        let transaction = record.to_transaction();
        log::debug!("Processing transaction in ledger: {transaction:?}");
        if let Err(e) = ledger.process_transaction(&transaction) {
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

    log::debug!("Starting account snapshot serialisation");
    for acc in ledger.account_snapshots() {
        log::debug!("Serialising account snapshot: {acc:?}");
        wtr.serialize(acc)?;
    }

    log::debug!("Account snapshot serialisation done -> Flushing to stdout");
    wtr.flush()?;

    Ok(())
}
