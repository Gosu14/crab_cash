use crate::engine::amount::{Amount, AmountError};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone)]
enum AccountTxType {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone)]
struct AccountTx {
    amount: Amount,
    typ: AccountTxType,
    is_disputed: bool,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub id: u16, // Unique
    pub amount_available: Amount,
    pub amount_held: Amount,
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

    #[error("Invalid Amount operation (tx id {0})")]
    InvalidAmountOperation(#[from] AmountError),
}

impl Account {
    pub fn new(client_id: u16) -> Self {
        Account {
            id: client_id,
            amount_available: Amount::new(),
            amount_held: Amount::new(),
            is_locked: false,
            tx: HashMap::new(),
        }
    }

    pub fn deposit(&mut self, tx_id: u32, tx_amount: Amount) -> Result<(), AccountOperationError> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }

        if !self.tx.contains_key(&tx_id) {
            self.amount_available = self.amount_available.add(&tx_amount)?;

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

    pub fn withdraw(&mut self, tx_id: u32, tx_amount: Amount) -> Result<(), AccountOperationError> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }
        if !self.tx.contains_key(&tx_id) {
            if self.amount_available >= tx_amount {
                self.amount_available = self.amount_available.sub(&tx_amount)?;

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

    pub fn dispute(&mut self, tx_id: u32) -> Result<(), AccountOperationError> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
            if tx.is_disputed {
                Err(AccountOperationError::TxAlreadyDisputed(tx_id))? // Already disputed -> ignored
            }

            match tx.typ {
                AccountTxType::Deposit => {
                    let new_available = self.amount_available.sub(&tx.amount)?;

                    let new_held = self.amount_held.add(&tx.amount)?;

                    // Hold the funds and keep the same total
                    self.amount_available = new_available;
                    self.amount_held = new_held;
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

    pub fn resolve(&mut self, tx_id: u32) -> Result<(), AccountOperationError> {
        if self.is_locked {
            Err(AccountOperationError::AccountLocked(tx_id))?
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
            if !tx.is_disputed {
                Err(AccountOperationError::TxNotDisputed(tx_id))?
            }

            match tx.typ {
                AccountTxType::Deposit => {
                    let new_held = self.amount_held.sub(&tx.amount)?;

                    let new_available = self.amount_available.add(&tx.amount)?;

                    // Release held funds back to available
                    self.amount_held = new_held;
                    self.amount_available = new_available;
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

    pub fn chargeback(&mut self, tx_id: u32) -> Result<(), AccountOperationError> {
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
                    self.amount_held = self.amount_held.sub(&tx.amount)?;
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
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_that_a_bigger_amount_than_what_is_available_cannot_be_withdrawn() {
        let mut account = Account::new(0);

        // Make Deposit
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());

        // Withdraw all
        let _ = account.withdraw(1, Amount::from_str("100.0").unwrap());

        // Verify client 1: deposit 100.0 + withdrawal 100.0 = 0.0
        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);

        // Try to withdraw more and check that is ignored
        let err = account.withdraw(2, Amount::from_str("50.0").unwrap());
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(
            err,
            AccountOperationError::WithdrawalLimitExceeded(_)
        ));

        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_dispute_can_be_resolved() {
        let mut account = Account::new(0);

        // Make a deposit
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());

        // Dispute the deposit
        let _ = account.dispute(0);

        // Verify that the deposit is under dispute
        let deposit = account.tx.get(&0).unwrap();
        assert!(deposit.is_disputed);
        assert_eq!(account.amount_held, Amount::from_str("100.0").unwrap());
        assert_eq!(account.amount_available, Amount::new());

        // Then resolve
        let _ = account.resolve(0);

        // Verify that now the account is not locked and amount back to 100.0
        assert!(!account.is_locked);
        assert_eq!(account.amount_held, Amount::new());
        assert_eq!(account.amount_available, Amount::from_str("100.0").unwrap());

        // Try adding another deposit
        let _ = account.deposit(1, Amount::from_str("200.0").unwrap());

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        assert_eq!(account.amount_available, Amount::from_str("300.0").unwrap());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_account_is_locked_after_chargeback() {
        let mut account = Account::new(0);

        // First make a deposit
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());

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
        let err = account.deposit(1, Amount::from_str("200.0").unwrap());
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::AccountLocked(_)));

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::new());
        assert!(account.is_locked);
    }

    #[test]
    fn test_that_dispute_on_withdrawhal_are_ignored() {
        let mut account = Account::new(0);

        // Make a deposit
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());

        // Withdraw
        let _ = account.withdraw(1, Amount::from_str("50.0").unwrap());

        // Try dispute the withdrawal
        let err = account.dispute(1);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(
            err,
            AccountOperationError::InvalidWithdrawalDispute(_)
        ));

        // Verify that the deposit is under dispute
        let withdrawal = account.tx.get(&1).unwrap();
        assert!(!withdrawal.is_disputed);
        assert_eq!(account.amount_held, Amount::new());
        assert_eq!(account.amount_available, Amount::from_str("50.0").unwrap());

        // Verify client 1: deposit 100.0 + withdrawal 50.0 + try dispute the withdrawal = 50.0
        assert_eq!(account.amount_available, Amount::from_str("50.0").unwrap());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_deposit_with_same_tx_id_is_rejected() {
        let mut account = Account::new(0);

        // First deposit
        let res = account.deposit(0, Amount::from_str("100.0").unwrap());
        assert!(res.is_ok());

        // Second deposit with same tx id
        let err = account.deposit(0, Amount::from_str("50.0").unwrap());
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxAlreadyExist(0)));

        // Verify that only first deposit is applied
        assert_eq!(account.amount_available, Amount::from_str("100.0").unwrap());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_withdraw_with_same_tx_id_is_rejected() {
        let mut account = Account::new(0);

        // Deposit then withdraw
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());
        let res = account.withdraw(1, Amount::from_str("50.0").unwrap());
        assert!(res.is_ok());

        // Second withdraw with same tx id
        let err = account.withdraw(1, Amount::from_str("10.0").unwrap());
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxAlreadyExist(1)));

        // Verify that only first withdraw is applied
        assert_eq!(account.amount_available, Amount::from_str("50.0").unwrap());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_dispute_on_unknown_tx_is_rejected() {
        let mut account = Account::new(0);

        // No tx with id 42
        let err = account.dispute(42);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxUnknown(42)));

        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_dispute_cannot_be_raised_twice() {
        let mut account = Account::new(0);

        // Make a deposit and dispute it
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());
        let _ = account.dispute(0);

        let disputed_tx = account.tx.get(&0).unwrap();
        assert!(disputed_tx.is_disputed);
        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::from_str("100.0").unwrap());

        // Disputing again should fail
        let err = account.dispute(0);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxAlreadyDisputed(0)));

        // State unchanged
        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::from_str("100.0").unwrap());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_resolve_on_unknown_tx_is_rejected() {
        let mut account = Account::new(0);

        // No tx with id 42
        let err = account.resolve(42);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxUnknown(42)));

        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_resolve_on_not_disputed_tx_is_rejected() {
        let mut account = Account::new(0);

        // Deposit but do not dispute
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());

        let err = account.resolve(0);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxNotDisputed(0)));

        // State unchanged
        assert_eq!(account.amount_available, Amount::from_str("100.0").unwrap());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_resolve_on_withdrawal_is_rejected() {
        let mut account = Account::new(0);

        // Deposit then withdraw
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());
        let _ = account.withdraw(1, Amount::from_str("50.0").unwrap());
        let _ = account.dispute(1);

        let err = account.resolve(1);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxNotDisputed(1)));

        // State unchanged
        assert_eq!(account.amount_available, Amount::from_str("50.0").unwrap());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_chargeback_on_unknown_tx_is_rejected() {
        let mut account = Account::new(0);

        let err = account.chargeback(42);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxUnknown(42)));

        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_chargeback_on_not_disputed_tx_is_rejected() {
        let mut account = Account::new(0);

        // Deposit but do not dispute
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());

        let err = account.chargeback(0);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxNotDisputed(0)));

        // State unchanged and account not locked
        assert_eq!(account.amount_available, Amount::from_str("100.0").unwrap());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_chargeback_on_withdrawal_is_rejected() {
        let mut account = Account::new(0);

        // Deposit then withdraw
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());
        let _ = account.withdraw(1, Amount::from_str("50.0").unwrap());
        let _ = account.dispute(1);

        let err = account.chargeback(1);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, AccountOperationError::TxNotDisputed(1)));

        // State unchanged and account not locked
        assert_eq!(account.amount_available, Amount::from_str("50.0").unwrap());
        assert_eq!(account.amount_held, Amount::new());
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_operations_on_locked_account_are_rejected() {
        let mut account = Account::new(0);

        // Setup: deposit, dispute, then chargeback to lock account
        let _ = account.deposit(0, Amount::from_str("100.0").unwrap());
        let _ = account.dispute(0);
        let _ = account.chargeback(0);
        assert!(account.is_locked);

        // All further operations should be rejected with AccountLocked
        let err = account.deposit(1, Amount::from_str("10.0").unwrap());
        assert!(matches!(
            err.unwrap_err(),
            AccountOperationError::AccountLocked(1)
        ));

        let err = account.withdraw(2, Amount::from_str("10.0").unwrap());
        assert!(matches!(
            err.unwrap_err(),
            AccountOperationError::AccountLocked(2)
        ));

        let err = account.dispute(0);
        assert!(matches!(
            err.unwrap_err(),
            AccountOperationError::AccountLocked(0)
        ));

        let err = account.resolve(0);
        assert!(matches!(
            err.unwrap_err(),
            AccountOperationError::AccountLocked(0)
        ));

        let err = account.chargeback(0);
        assert!(matches!(
            err.unwrap_err(),
            AccountOperationError::AccountLocked(0)
        ));

        // Balances remain what they were after the first chargeback
        assert_eq!(account.amount_available, Amount::new());
        assert_eq!(account.amount_held, Amount::new());
        assert!(account.is_locked);
    }
}
