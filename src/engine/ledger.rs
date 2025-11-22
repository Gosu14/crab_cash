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
            TransactionType::Deposit => account.deposit(tx.id, tx.amount),
            TransactionType::Withdrawal => account.withdraw(tx.id, tx.amount),
            TransactionType::Dispute => account.dispute(tx.id),
            TransactionType::Resolve => account.resolve(tx.id),
            TransactionType::Chargeback => account.chargeback(tx.id),
        }
    }

    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    pub fn get_account(&self, id: u16) -> &Account {
        self.accounts.get(&id).expect("Missing account")
    }
}
