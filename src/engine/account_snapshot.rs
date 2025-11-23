use serde::{Deserialize, Serialize};

/// A Snapshot of an Account to easily view the content
/// It is used for decoupling ledger output from Account and easy serialisation
#[derive(Serialize, Deserialize, Debug)]
pub struct AccountSnapshot {
    pub client: String,
    pub available: String,
    pub held: String,
    pub total: String,
    pub locked: bool,
}
