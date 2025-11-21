pub struct Transaction {
    pub typ: TransactionType,
    pub id: u32,
    pub amount: f64,
    pub is_disputed: bool,
}

pub enum TransactionType {
    Deposit,
    Withdrawal,
}
