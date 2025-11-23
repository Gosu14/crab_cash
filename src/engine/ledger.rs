use crate::engine::account::{Account, AccountOperationError};
use crate::engine::account_snapshot::AccountSnapshot;
use crate::engine::amount::{Amount, AmountError};
use crate::engine::{Transaction, TransactionType};
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

    #[error("Negative Tx amount is not allowed (tx id {0})")]
    NegativeTxAmount(u32),
}

pub struct Ledger {
    tx_processed: HashSet<u32>,
    accounts: HashMap<u16, Account>,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
            tx_processed: HashSet::new(),
        }
    }

    pub fn process_transaction(&mut self, tx: &Transaction) -> Result<(), LedgerError> {
        let account = self
            .accounts
            .entry(tx.account_id)
            .or_insert_with(|| Account::new(tx.account_id));

        match tx.typ {
            TransactionType::Deposit => {
                if self.tx_processed.contains(&tx.id) {
                    Err(LedgerError::DuplicateTxId(tx.id))?
                }

                let amount_str = tx
                    .amount
                    .as_ref()
                    .ok_or(LedgerError::MissingAmount(tx.id))?;
                let amount = Amount::from_str(amount_str)?;
                // Negative transaction amount are forbidden and will return error
                if amount < Amount::new() {
                    Err(LedgerError::NegativeTxAmount(tx.id))?;
                }
                account.deposit(tx.id, amount)?;
                self.tx_processed.insert(tx.id);
            }
            TransactionType::Withdrawal => {
                if self.tx_processed.contains(&tx.id) {
                    Err(LedgerError::DuplicateTxId(tx.id))?
                }
                let amount_str = tx
                    .amount
                    .as_ref()
                    .ok_or(LedgerError::MissingAmount(tx.id))?;
                let amount = Amount::from_str(amount_str)?;
                // Negative transaction amount are forbidden and will return error
                if amount < Amount::new() {
                    Err(LedgerError::NegativeTxAmount(tx.id))?;
                }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Transaction, TransactionType};

    #[test]
    fn test_that_duplicate_tx_id_is_rejected_by_ledger() {
        let mut ledger = Ledger::new();

        // First deposit with tx id 1 on client 1
        let tx1 = Transaction {
            id: 1,
            account_id: 1,
            typ: TransactionType::Deposit,
            amount: Some(String::from("10.0")),
        };
        assert!(ledger.process_transaction(&tx1).is_ok());

        // Second deposit with same tx id 1, even on different client
        let tx2 = Transaction {
            id: 1,
            account_id: 2,
            typ: TransactionType::Deposit,
            amount: Some(String::from("5.0")),
        };
        let err = ledger.process_transaction(&tx2).unwrap_err();
        assert!(matches!(err, LedgerError::DuplicateTxId(1)));
    }

    #[test]
    fn test_that_invalid_amount_string_is_rejected() {
        let mut ledger = Ledger::new();

        let tx = Transaction {
            id: 1,
            account_id: 1,
            typ: TransactionType::Deposit,
            amount: Some(String::from("not_parsable")),
        };
        let err = ledger.process_transaction(&tx).unwrap_err();
        assert!(matches!(err, LedgerError::Amount(_)));
    }

    #[test]
    fn test_that_overflow_in_total_removes_account_from_snapshots() {
        let mut ledger = Ledger::new();

        let acc = ledger.accounts.entry(1).or_insert_with(|| Account::new(1));

        // Force near-overflow values manually
        acc.amount_available = Amount::from_str("922337203685477.5807").unwrap();
        acc.amount_held = Amount::from_str("1.0").unwrap();

        // This should overflow available + held and thus be filtered out
        let snapshots: Vec<_> = ledger.account_snapshots().collect();
        assert!(snapshots.is_empty());
    }

    #[test]
    fn test_that_negative_deposit_amount_is_rejected() {
        let mut ledger = Ledger::new();

        let tx = Transaction {
            id: 1,
            account_id: 1,
            typ: TransactionType::Deposit,
            amount: Some(String::from("-1.0")),
        };
        let err = ledger.process_transaction(&tx).unwrap_err();
        assert!(matches!(err, LedgerError::NegativeTxAmount(1)));
    }

    #[test]
    fn test_that_negative_withdrawal_amount_is_rejected() {
        let mut ledger = Ledger::new();

        let tx = Transaction {
            id: 1,
            account_id: 1,
            typ: TransactionType::Withdrawal,
            amount: Some(String::from("-1.0")),
        };
        let err = ledger.process_transaction(&tx).unwrap_err();
        assert!(matches!(err, LedgerError::NegativeTxAmount(1)));
    }
}
