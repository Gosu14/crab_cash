pub struct Transaction {
    pub id: u32,
    pub account_id: u16,
    pub amount: f64,
    pub typ: TransactionType,
    pub is_disputed: bool,
}

pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
