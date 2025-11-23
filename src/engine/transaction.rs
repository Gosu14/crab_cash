pub struct Transaction {
    pub id: u32,
    pub account_id: u16,
    pub amount: Option<String>,
    pub typ: TransactionType,
}

pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
