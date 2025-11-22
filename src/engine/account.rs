use std::collections::HashMap;

enum AccountTxType {
    Deposit,
    Withdrawal,
}

struct AccountTx {
    amount: f64,
    typ: AccountTxType,
    is_disputed: bool,
}

// Client Account
pub struct Account {
    pub id: u16, // Unique
    pub amount_available: f64,
    pub amount_held: f64,
    pub is_locked: bool,
    tx: HashMap<u32, AccountTx>,
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

    pub fn deposit(&mut self, tx_id: u32, tx_amount: f64) {
        if self.is_locked {
            return;
        }
        self.amount_available += tx_amount;
        self.tx.insert(
            tx_id,
            AccountTx {
                amount: tx_amount,
                typ: AccountTxType::Deposit,
                is_disputed: false,
            },
        );
    }

    pub fn withdraw(&mut self, tx_id: u32, tx_amount: f64) {
        if self.is_locked {
            return;
        }
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
        }
    }

    pub fn dispute(&mut self, tx_id: u32) {
        if self.is_locked {
            return;
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
            if tx.is_disputed {
                return; // Already disputed -> ignored
            }

            match tx.typ {
                AccountTxType::Deposit => {
                    // Hold the funds and keep the same total
                    self.amount_available -= tx.amount;
                    self.amount_held += tx.amount;
                }
                AccountTxType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
            }

            tx.is_disputed = true;
        }
    }

    pub fn resolve(&mut self, tx_id: u32) {
        if self.is_locked {
            return;
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
            if !tx.is_disputed {
                return;
            }

            match tx.typ {
                AccountTxType::Deposit => {
                    // Release held funds back to available
                    self.amount_held -= tx.amount;
                    self.amount_available += tx.amount;
                }
                AccountTxType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
            }

            tx.is_disputed = false;
        }
    }

    pub fn chargeback(&mut self, tx_id: u32) {
        if self.is_locked {
            return;
        }
        if let Some(tx) = self.tx.get_mut(&tx_id) {
            if !tx.is_disputed {
                return; // Not under dispute
            }

            match tx.typ {
                AccountTxType::Deposit => {
                    // Remove held funds
                    self.amount_held -= tx.amount;
                }
                AccountTxType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
            }

            self.is_locked = true;
            tx.is_disputed = false; // Dispute resolved via chargeback
        }
    }
}

mod tests {
    use crate::engine::Account;

    #[test]
    fn test_that_a_bigger_amount_than_what_is_available_cannot_be_withdrawn() {
        let mut account = Account::new(0);

        // Make Deposit
        account.deposit(0, 100.0);

        // Withdraw all
        account.withdraw(1, 100.0);

        // Verify client 1: deposit 100.0 + withdrawal 100.0 = 0.0
        assert_eq!(account.amount_available, 0.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);

        // Try to withdraw more and check that is ignored
        account.withdraw(2, 50.0);

        assert_eq!(account.amount_available, 0.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_dispute_can_be_resolved() {
        let mut account = Account::new(0);

        // Make a deposit
        account.deposit(0, 100.0);

        // Dispute the deposit
        account.dispute(0);

        // Verify that the deposit is under dispute
        let deposit = account.tx.get(&0).unwrap();
        assert!(deposit.is_disputed);
        assert_eq!(account.amount_held, 100.0);
        assert_eq!(account.amount_available, 0.0);

        // Then resolve
        account.resolve(0);

        // Verify that now the account is not locked and amount back to 100.0
        assert!(!account.is_locked);
        assert_eq!(account.amount_held, 0.0);
        assert_eq!(account.amount_available, 100.0);

        // Try adding another deposit
        account.deposit(1, 200.0);

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        assert_eq!(account.amount_available, 300.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(!account.is_locked);
    }

    #[test]
    fn test_that_account_is_locked_after_chargeback() {
        let mut account = Account::new(0);

        // First make a deposit
        account.deposit(0, 100.0);

        // Then dispute the deposit
        account.dispute(0);

        // Verify that the deposit is under dispute
        let disputed_tx = account.tx.get(&0).unwrap();
        assert!(disputed_tx.is_disputed);

        // Then chargeback
        account.chargeback(0);

        // Verify that now the account is locked
        assert!(account.is_locked);

        // Try adding another deposit
        account.deposit(1, 200.0);

        // Verify client 1: deposit 100.0 + dispute + chargeback + deposit 200.0 = 0.0
        assert_eq!(account.amount_available, 0.0);
        assert_eq!(account.amount_held, 0.0);
        assert!(account.is_locked);
    }

    #[test]
    fn test_that_dispute_on_withdrawhal_are_ignored() {
        let mut account = Account::new(0);

        // Make a deposit
        account.deposit(0, 100.0);

        // Withdraw
        account.withdraw(1, 50.0);

        // Try dispute the withdrawal
        account.dispute(1);

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
