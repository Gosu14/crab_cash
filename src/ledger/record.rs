use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct InputRecord {
    #[serde(rename = "type")]
    pub typ: RecordType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum RecordType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
