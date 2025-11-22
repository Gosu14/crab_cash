use crate::Account;
use crate::{Transaction, TransactionType};
use std::collections::HashMap;

pub struct Ledger {
    accounts: HashMap<u16, Account>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
    }

    pub fn process_transaction(&mut self, tx: Transaction) {
        let account = self
            .accounts
            .entry(tx.account_id)
            .or_insert_with(|| Account::new(tx.account_id));

        match tx.typ {
            TransactionType::Deposit => {
                if let Err(_e) = account.deposit(tx.id, tx.amount) { /* To be logged */ }
            }
            TransactionType::Withdrawal => {
                if let Err(_e) = account.withdraw(tx.id, tx.amount) { /* To be logged */ }
            }
            TransactionType::Dispute => {
                if let Err(_e) = account.dispute(tx.id) { /* To be logged */ }
            }
            TransactionType::Resolve => {
                if let Err(_e) = account.resolve(tx.id) { /* To be logged */ }
            }
            TransactionType::Chargeback => {
                if let Err(_e) = account.chargeback(tx.id) { /* To be logged */ }
            }
        }
    }

    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    pub fn get_account(&self, id: u16) -> &Account {
        self.accounts.get(&id).expect("Missing account")
    }
}
