use anyhow::{Ok, Result};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone)]
enum AccountTxType {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone)]
struct AccountTx {
    amount: f64,
    typ: AccountTxType,
    is_disputed: bool,
}

#[derive(Debug)]
pub struct Account {
    pub id: u16, // Unique
    pub amount_available: f64,
    pub amount_held: f64,
    pub is_locked: bool,
    tx: HashMap<u32, AccountTx>,
}

#[derive(Error, Debug)]
pub enum AccountOperationError {
    #[error("Account is locked (tx id {0})")]
    AccountLocked(u32),

    #[error("Transaction already exists (tx id {0})")]
    TxAlreadyExist(u32),

    #[error("Unknown transaction (tx id {0})")]
    TxUnknown(u32),

    #[error("Withdrawal limit exceeded (tx id {0})")]
    WithdrawalLimitExceeded(u32),

    #[error("Transaction already disputed (tx id {0})")]
    TxAlreadyDisputed(u32),

    #[error("Transaction not disputed (tx id {0})")]
    TxNotDisputed(u32),

    #[error("Withdrawal transaction cannot be disputed / resolved / charged back (tx id {0})")]
    InvalidWithdrawalDispute(u32),
}

impl Account {
    pub fn new(client_id: u16) -> Self {
        Account {
            id: client_id,
            amount_available: 0.0,
            amount_held: 0.0,
            is_locked: false,
            tx: HashMap::new(),
        }
    }

    pub fn deposit(&mut self, tx_id: u32, tx_amount: f64) -> Result<()> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }

        if !self.tx.contains_key(&tx_id) {
            self.amount_available += tx_amount;
            self.tx.insert(
                tx_id,
                AccountTx {
                    amount: tx_amount,
                    typ: AccountTxType::Deposit,
                    is_disputed: false,
                },
            );
        } else {
            Err(AccountOperationError::TxAlreadyExist(tx_id))?
        }

        Ok(())
    }

    pub fn withdraw(&mut self, tx_id: u32, tx_amount: f64) -> Result<()> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }
        if !self.tx.contains_key(&tx_id) {
            if self.amount_available >= tx_amount {
                self.amount_available -= tx_amount;
                self.tx.insert(
                    tx_id,
                    AccountTx {
                        amount: tx_amount,
                        typ: AccountTxType::Withdrawal,
                        is_disputed: false,
                    },
                );
            } else {
                Err(AccountOperationError::WithdrawalLimitExceeded(tx_id))?
            }
        } else {
            Err(AccountOperationError::TxAlreadyExist(tx_id))?
        }

        Ok(())
    }

    pub fn dispute(&mut self, tx_id: u32) -> Result<()> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
            if tx.is_disputed {
                Err(AccountOperationError::TxAlreadyDisputed(tx_id))? // Already disputed -> ignored
            }

            match tx.typ {
                AccountTxType::Deposit => {
                    // Hold the funds and keep the same total
                    self.amount_available -= tx.amount;
                    self.amount_held += tx.amount;
                }
                AccountTxType::Withdrawal => {
                    Err(AccountOperationError::InvalidWithdrawalDispute(tx_id))? // Withdrawal can't be disputed
                }
            }

            tx.is_disputed = true;
        } else {
            Err(AccountOperationError::TxUnknown(tx_id))? // Unknown Tx
        }
        Ok(())
    }

    pub fn resolve(&mut self, tx_id: u32) -> Result<()> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
            if !tx.is_disputed {
                Err(AccountOperationError::TxNotDisputed(tx_id))?
            }

            match tx.typ {
                AccountTxType::Deposit => {
                    // Release held funds back to available
                    self.amount_held -= tx.amount;
                    self.amount_available += tx.amount;
                }
                AccountTxType::Withdrawal => {
                    Err(AccountOperationError::InvalidWithdrawalDispute(tx_id))?
                }
            }
            tx.is_disputed = false;
        } else {
            Err(AccountOperationError::TxUnknown(tx_id))? // Unknown Tx
        }
        Ok(())
    }

    pub fn chargeback(&mut self, tx_id: u32) -> Result<()> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
            if !tx.is_disputed {
                Err(AccountOperationError::TxNotDisputed(tx_id))? // Not under dispute
            }

            match tx.typ {
                AccountTxType::Deposit => {
                    // Remove held funds
                    self.amount_held -= tx.amount;
                }
                AccountTxType::Withdrawal => {
                    Err(AccountOperationError::InvalidWithdrawalDispute(tx_id))?
                }
            }

            self.is_locked = true;
            tx.is_disputed = false; // Dispute resolved via chargeback
        } else {
            Err(AccountOperationError::TxUnknown(tx_id))? // Unknown Tx
        }
        Ok(())
    }
}

mod tests {
    use crate::engine::{Account, account::AccountOperationError};

    #[test]
    fn test_that_a_bigger_amount_than_what_is_available_cannot_be_withdrawn() {
        let mut account = Account::new(0);

        // Make Deposit
        let _ = account.deposit(0, 100.0);

        // Withdraw all
        let _ = account.withdraw(1, 100.0);

        // Verify client 1: deposit 100.0 + withdrawal 100.0 = 0.0
        assert_eq!(account.amount_available, 0.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);

        // Try to withdraw more and check that is ignored
        let err = account.withdraw(2, 50.0);
        assert!(err.is_err());
        let err = err.unwrap_err();
        let op_err = err.downcast::<AccountOperationError>().unwrap();
        assert!(matches!(
            op_err,
            AccountOperationError::WithdrawalLimitExceeded(_)
        ));

        assert_eq!(account.amount_available, 0.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_dispute_can_be_resolved() {
        let mut account = Account::new(0);

        // Make a deposit
        let _ = account.deposit(0, 100.0);

        // Dispute the deposit
        let _ = account.dispute(0);

        // Verify that the deposit is under dispute
        let deposit = account.tx.get(&0).unwrap();
        assert!(deposit.is_disputed);
        assert_eq!(account.amount_held, 100.0);
        assert_eq!(account.amount_available, 0.0);

        // Then resolve
        let _ = account.resolve(0);

        // Verify that now the account is not locked and amount back to 100.0
        assert!(!account.is_locked);
        assert_eq!(account.amount_held, 0.0);
        assert_eq!(account.amount_available, 100.0);

        // Try adding another deposit
        let _ = account.deposit(1, 200.0);

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        assert_eq!(account.amount_available, 300.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_account_is_locked_after_chargeback() {
        let mut account = Account::new(0);

        // First make a deposit
        let _ = account.deposit(0, 100.0);

        // Then dispute the deposit
        let _ = account.dispute(0);

        // Verify that the deposit is under dispute
        let disputed_tx = account.tx.get(&0).unwrap();
        assert!(disputed_tx.is_disputed);

        // Then chargeback
        let _ = account.chargeback(0);

        // Verify that now the account is locked
        assert!(account.is_locked);

        // Try adding another deposit
        let err = account.deposit(1, 200.0);
        assert!(err.is_err());
        let err = err.unwrap_err();
        let op_err = err.downcast::<AccountOperationError>().unwrap();
        assert!(matches!(op_err, AccountOperationError::AccountLocked(_)));

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        assert_eq!(account.amount_available, 0.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(account.is_locked);
    }

    #[test]
    fn test_that_dispute_on_withdrawhal_are_ignored() {
        let mut account = Account::new(0);

        // Make a deposit
        let _ = account.deposit(0, 100.0);

        // Withdraw
        let _ = account.withdraw(1, 50.0);

        // Try dispute the withdrawal
        let err = account.dispute(1);
        assert!(err.is_err());
        let err = err.unwrap_err();
        let op_err = err.downcast::<AccountOperationError>().unwrap();
        assert!(matches!(
            op_err,
            AccountOperationError::InvalidWithdrawalDispute(_)
        ));

        // Verify that the deposit is under dispute
        let withdrawal = account.tx.get(&1).unwrap();
        assert!(!withdrawal.is_disputed);
        assert_eq!(account.amount_held, 0.0);
        assert_eq!(account.amount_available, 50.0);

        // Verify client 1: deposit 100.0 + withdrawal 50.0 + try dispute the withdrawal = 50.0
        assert_eq!(account.amount_available, 50.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);
    }
}
