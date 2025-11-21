use crate::{Transaction, TransactionType};
use std::collections::HashMap;

// Client Account
pub struct Account {
    pub id: u16, // Unique
    pub amount_available: f64,
    pub amount_held: f64,
    pub is_locked: bool,
    pub tx: HashMap<u32, Transaction>,
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

    pub fn deposit(&mut self, transaction: Transaction) {
        if self.is_locked {
            return;
        }
        self.amount_available += transaction.amount;
        self.tx.insert(transaction.id, transaction);
    }

    pub fn withdraw(&mut self, transaction: Transaction) {
        if self.is_locked {
            return;
        }
        if self.amount_available >= transaction.amount {
            self.amount_available -= transaction.amount;
            self.tx.insert(transaction.id, transaction);
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
                TransactionType::Deposit => {
                    // Hold the funds and keep the same total
                    self.amount_available -= tx.amount;
                    self.amount_held += tx.amount;
                }
                TransactionType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
                _ => return,
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
                TransactionType::Deposit => {
                    // Release held funds back to available
                    self.amount_held -= tx.amount;
                    self.amount_available += tx.amount;
                }
                TransactionType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
                _ => return,
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
                TransactionType::Deposit => {
                    // Remove held funds
                    self.amount_held -= tx.amount;
                }
                TransactionType::Withdrawal => {
                    return; // Withdrawal can't be disputed
                }
                _ => return,
            }

            self.is_locked = true;
            tx.is_disputed = false; // Dispute resolved via chargeback
        }
    }
}
