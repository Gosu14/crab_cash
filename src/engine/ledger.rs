use crate::Account;
use crate::engine::account::AccountOperationError;
use crate::{Transaction, TransactionType};
use anyhow::{Ok, Result};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LedgerError {
    #[error("Account operation failed: {0}")]
    Account(#[from] AccountOperationError),
}

pub struct Ledger {
    accounts: HashMap<u16, Account>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
    }

    pub fn process_transaction(&mut self, tx: Transaction) -> Result<()> {
        let account = self
            .accounts
            .entry(tx.account_id)
            .or_insert_with(|| Account::new(tx.account_id));

        match tx.typ {
            TransactionType::Deposit => account.deposit(tx.id, tx.amount)?,
            TransactionType::Withdrawal => account.withdraw(tx.id, tx.amount)?,
            TransactionType::Dispute => account.dispute(tx.id)?,
            TransactionType::Resolve => account.resolve(tx.id)?,
            TransactionType::Chargeback => account.chargeback(tx.id)?,
        }

        Ok(())
    }

    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    pub fn get_account(&self, id: u16) -> &Account {
        self.accounts.get(&id).expect("Missing account")
    }
}
