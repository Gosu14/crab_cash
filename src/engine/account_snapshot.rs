use serde::{Deserialize, Serialize};

/// A Snapshot of an Account to easily view the content
/// It is used for decoupling ledger output from Account and easy serialisation
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountSnapshot {
    pub client: String,
    pub available: String,
    pub held: String,
    pub total: String,
    pub locked: bool,
}
