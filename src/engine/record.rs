use serde::Deserialize;

use crate::engine::{Transaction, TransactionType};

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

impl InputRecord {
    pub fn to_transaction(&self) -> Transaction {
        match self.typ {
            RecordType::Deposit => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: self.amount.unwrap(),
                typ: TransactionType::Deposit,
                is_disputed: false,
            },
            RecordType::Withdrawal => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: self.amount.unwrap(),
                typ: TransactionType::Withdrawal,
                is_disputed: false,
            },
            RecordType::Dispute => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: 0.0,
                typ: TransactionType::Dispute,
                is_disputed: false,
            },
            RecordType::Resolve => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: 0.0,
                typ: TransactionType::Resolve,
                is_disputed: false,
            },
            RecordType::Chargeback => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: 0.0,
                typ: TransactionType::Chargeback,
                is_disputed: false,
            },
        }
    }
}
