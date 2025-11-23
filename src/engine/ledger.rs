use crate::engine::account::{Account, AccountOperationError};
use crate::engine::account_snapshot::AccountSnapshot;
use crate::engine::amount::{Amount, AmountError};
use crate::engine::{Transaction, TransactionType};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LedgerError {
    #[error("Account operation failed (tx id {0})")]
    Account(#[from] AccountOperationError),

    #[error("Duplicate transaction id (tx id {0})")]
    DuplicateTxId(u32),

    #[error("Missing Amount id (tx id {0})")]
    MissingAmount(u32),

    #[error("Amount parsing failed (tx id {0})")]
    Amount(#[from] AmountError),
}

pub struct Ledger {
    tx_processed: HashSet<u32>,
    accounts: HashMap<u16, Account>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
            tx_processed: HashSet::new(),
        }
    }

    pub fn process_transaction(&mut self, tx: Transaction) -> Result<()> {
        let account = self
            .accounts
            .entry(tx.account_id)
            .or_insert_with(|| Account::new(tx.account_id));

        match tx.typ {
            TransactionType::Deposit => {
                if self.tx_processed.contains(&tx.id) {
                    Err(LedgerError::DuplicateTxId(tx.id))?
                }

                let amount_str = tx.amount.ok_or(LedgerError::MissingAmount(tx.id))?;
                let amount = Amount::from_str(&amount_str)?;
                account.deposit(tx.id, amount)?;
                self.tx_processed.insert(tx.id);
            }
            TransactionType::Withdrawal => {
                if self.tx_processed.contains(&tx.id) {
                    Err(LedgerError::DuplicateTxId(tx.id))?
                }
                let amount_str = tx.amount.ok_or(LedgerError::MissingAmount(tx.id))?;
                let amount = Amount::from_str(&amount_str)?;
                account.withdraw(tx.id, amount)?;
                self.tx_processed.insert(tx.id);
            }
            TransactionType::Dispute => account.dispute(tx.id)?,
            TransactionType::Resolve => account.resolve(tx.id)?,
            TransactionType::Chargeback => account.chargeback(tx.id)?,
        }

        Ok(())
    }

    // WARNING: Overflow error when computing total - will be swallowed and logged
    pub fn account_snapshots(&self) -> impl Iterator<Item = AccountSnapshot> {
        self.accounts.values().filter_map(|acc| {
            match acc.amount_available.add(&acc.amount_held) {
                Ok(total) => Some(AccountSnapshot {
                    client: acc.id.to_string(),
                    available: acc.amount_available.to_string(),
                    held: acc.amount_held.to_string(),
                    total: total.to_string(),
                    locked: acc.is_locked,
                }),
                Err(_) => {
                    // Total is overflowing => silently ignore but log
                    log::warn!("Ledger::account_snapshots error: Overflow when computing Total. This is ignored silently.");
                    None
                }
            }
        })
    }

    pub fn get_account(&self, id: u16) -> &Account {
        self.accounts.get(&id).expect("Missing account")
    }
}
