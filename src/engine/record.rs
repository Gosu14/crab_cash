use serde::Deserialize;

use crate::engine::{Transaction, TransactionType};

#[derive(Deserialize, Debug, Clone)]
pub struct InputRecord {
    #[serde(rename = "type")]
    pub typ: RecordType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<String>,
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
                amount: self.amount.clone(),
                typ: TransactionType::Deposit,
            },
            RecordType::Withdrawal => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: self.amount.clone(),
                typ: TransactionType::Withdrawal,
            },
            RecordType::Dispute => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: None,
                typ: TransactionType::Dispute,
            },
            RecordType::Resolve => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: None,
                typ: TransactionType::Resolve,
            },
            RecordType::Chargeback => Transaction {
                account_id: self.client,
                id: self.tx,
                amount: None,
                typ: TransactionType::Chargeback,
            },
        }
    }
}
