use std::collections::HashMap;
use std::collections::hash_map::Entry;

use super::amount::Amount;
use super::event::{ClientId, EventKind, TxId};
use super::transaction::{Transaction, TxKind};
use crate::error::EngineError;

/// A client's single asset account plus the transactions it owns.
#[derive(Debug, Clone)]
pub struct Account {
    client: ClientId,
    available: Amount,
    held: Amount,
    locked: bool,
    transactions: HashMap<TxId, Transaction>,
}

impl Account {
    pub fn new(client: ClientId) -> Account {
        Account {
            client,
            available: Amount::ZERO,
            held: Amount::ZERO,
            locked: false,
            transactions: HashMap::new(),
        }
    }

    pub fn client(&self) -> ClientId {
        self.client
    }

    pub fn available(&self) -> Amount {
        self.available
    }

    pub fn held(&self) -> Amount {
        self.held
    }

    pub fn total(&self) -> Amount {
        self.available
            .checked_add(self.held)
            .expect("account total must fit in Amount")
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Applies an event atomically: on error the account is left unchanged.
    pub fn apply(&mut self, tx: TxId, kind: EventKind) -> Result<(), EngineError> {
        if self.locked {
            return Err(EngineError::AccountLocked);
        }
        match kind {
            EventKind::Deposit(amount) => self.deposit(tx, amount),
            EventKind::Withdrawal(amount) => self.withdraw(tx, amount),
            EventKind::Dispute => self.dispute(tx),
            EventKind::Resolve => self.resolve(tx),
            EventKind::Chargeback => self.chargeback(tx),
        }
    }

    fn deposit(&mut self, tx: TxId, amount: Amount) -> Result<(), EngineError> {
        ensure_positive(amount)?;
        let available = add(self.available, amount)?;
        add(available, self.held)?;
        self.record(tx, TxKind::Deposit, amount)?;
        self.available = available;
        Ok(())
    }

    fn withdraw(&mut self, tx: TxId, amount: Amount) -> Result<(), EngineError> {
        ensure_positive(amount)?;
        if self.available < amount {
            return Err(EngineError::InsufficientFunds);
        }
        let available = add(self.available, -amount)?;
        self.record(tx, TxKind::Withdrawal, amount)?;
        self.available = available;
        Ok(())
    }

    /// Holds the disputed funds; `available` may go negative if they were already spent.
    fn dispute(&mut self, tx: TxId) -> Result<(), EngineError> {
        let transaction = self
            .transactions
            .get_mut(&tx)
            .ok_or(EngineError::UnknownTx)?;
        let amount = transaction.amount();
        let available = add(self.available, -amount)?;
        let held = add(self.held, amount)?;
        transaction.dispute()?;
        self.available = available;
        self.held = held;
        Ok(())
    }

    fn resolve(&mut self, tx: TxId) -> Result<(), EngineError> {
        let transaction = self
            .transactions
            .get_mut(&tx)
            .ok_or(EngineError::UnknownTx)?;
        let amount = transaction.amount();
        let available = add(self.available, amount)?;
        let held = add(self.held, -amount)?;
        transaction.resolve()?;
        self.available = available;
        self.held = held;
        Ok(())
    }

    fn chargeback(&mut self, tx: TxId) -> Result<(), EngineError> {
        let transaction = self
            .transactions
            .get_mut(&tx)
            .ok_or(EngineError::UnknownTx)?;
        let held = add(self.held, -transaction.amount())?;
        transaction.chargeback()?;
        self.held = held;
        self.locked = true;
        Ok(())
    }

    fn record(&mut self, tx: TxId, kind: TxKind, amount: Amount) -> Result<(), EngineError> {
        match self.transactions.entry(tx) {
            Entry::Occupied(_) => Err(EngineError::DuplicateTx),
            Entry::Vacant(slot) => {
                slot.insert(Transaction::new(kind, amount));
                Ok(())
            }
        }
    }
}

fn ensure_positive(amount: Amount) -> Result<(), EngineError> {
    if amount.is_positive() {
        Ok(())
    } else {
        Err(EngineError::NonPositiveAmount)
    }
}

fn add(a: Amount, b: Amount) -> Result<Amount, EngineError> {
    a.checked_add(b).ok_or(EngineError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::TxState;

    fn amt(s: &str) -> Amount {
        s.parse().unwrap()
    }

    fn deposit(s: &str) -> EventKind {
        EventKind::Deposit(amt(s))
    }

    fn withdrawal(s: &str) -> EventKind {
        EventKind::Withdrawal(amt(s))
    }

    fn assert_balances(account: &Account, available: &str, held: &str) {
        assert_eq!(account.available(), amt(available), "available");
        assert_eq!(account.held(), amt(held), "held");
        assert_eq!(
            account.total(),
            amt(available).checked_add(amt(held)).unwrap(),
            "total"
        );
    }

    fn funded(s: &str) -> Account {
        let mut account = Account::new(1);
        account.apply(1, deposit(s)).unwrap();
        account
    }

    #[test]
    fn deposit_increases_available_and_total() {
        let account = funded("1.5");
        assert_balances(&account, "1.5", "0");
    }

    #[test]
    fn withdrawal_decreases_available_and_total() {
        let mut account = funded("2");
        account.apply(2, withdrawal("1.5")).unwrap();
        assert_balances(&account, "0.5", "0");
    }

    #[test]
    fn withdrawal_with_insufficient_funds_fails_without_changes() {
        let mut account = funded("1");
        assert_eq!(
            account.apply(2, withdrawal("1.0001")),
            Err(EngineError::InsufficientFunds)
        );
        assert_balances(&account, "1", "0");
    }

    #[test]
    fn withdrawing_exact_balance_is_allowed() {
        let mut account = funded("1");
        account.apply(2, withdrawal("1")).unwrap();
        assert_balances(&account, "0", "0");
    }

    #[test]
    fn non_positive_amounts_are_rejected() {
        let mut account = Account::new(1);
        assert_eq!(
            account.apply(1, deposit("0")),
            Err(EngineError::NonPositiveAmount)
        );
        assert_eq!(
            account.apply(2, deposit("-1")),
            Err(EngineError::NonPositiveAmount)
        );
        assert_eq!(
            account.apply(3, withdrawal("0")),
            Err(EngineError::NonPositiveAmount)
        );
    }

    #[test]
    fn duplicate_transaction_id_is_rejected() {
        let mut account = funded("1");
        assert_eq!(
            account.apply(1, deposit("5")),
            Err(EngineError::DuplicateTx)
        );
        assert_balances(&account, "1", "0");
    }

    #[test]
    fn dispute_moves_funds_to_held() {
        let mut account = funded("3");
        account.apply(1, EventKind::Dispute).unwrap();
        assert_balances(&account, "0", "3");
    }

    #[test]
    fn dispute_of_spent_funds_makes_available_negative() {
        let mut account = funded("3");
        account.apply(2, withdrawal("2")).unwrap();
        account.apply(1, EventKind::Dispute).unwrap();
        assert_balances(&account, "-2", "3");
    }

    #[test]
    fn resolve_releases_held_funds() {
        let mut account = funded("3");
        account.apply(1, EventKind::Dispute).unwrap();
        account.apply(1, EventKind::Resolve).unwrap();
        assert_balances(&account, "3", "0");
        assert!(!account.is_locked());
    }

    #[test]
    fn chargeback_removes_held_funds_and_locks() {
        let mut account = funded("3");
        account.apply(2, deposit("1")).unwrap();
        account.apply(1, EventKind::Dispute).unwrap();
        account.apply(1, EventKind::Chargeback).unwrap();
        assert_balances(&account, "1", "0");
        assert!(account.is_locked());
        assert_eq!(account.transactions[&1].state(), TxState::ChargedBack);
    }

    #[test]
    fn locked_account_rejects_everything() {
        let mut account = funded("3");
        account.apply(1, EventKind::Dispute).unwrap();
        account.apply(1, EventKind::Chargeback).unwrap();
        assert_eq!(
            account.apply(2, deposit("1")),
            Err(EngineError::AccountLocked)
        );
        assert_eq!(
            account.apply(3, withdrawal("1")),
            Err(EngineError::AccountLocked)
        );
        assert_balances(&account, "0", "0");
    }

    #[test]
    fn references_to_unknown_transactions_are_rejected() {
        let mut account = funded("1");
        for kind in [
            EventKind::Dispute,
            EventKind::Resolve,
            EventKind::Chargeback,
        ] {
            assert_eq!(account.apply(99, kind), Err(EngineError::UnknownTx));
        }
        assert_balances(&account, "1", "0");
    }

    #[test]
    fn resolve_or_chargeback_without_dispute_is_rejected() {
        let mut account = funded("1");
        assert!(account.apply(1, EventKind::Resolve).is_err());
        assert!(account.apply(1, EventKind::Chargeback).is_err());
        assert_balances(&account, "1", "0");
        assert!(!account.is_locked());
    }

    #[test]
    fn withdrawal_dispute_holds_its_amount() {
        let mut account = funded("2");
        account.apply(2, withdrawal("1")).unwrap();
        account.apply(2, EventKind::Dispute).unwrap();
        assert_balances(&account, "0", "1");
    }

    #[test]
    fn invalid_state_transitions_leave_balances_unchanged() {
        let mut account = funded("2");
        account.apply(1, EventKind::Dispute).unwrap();
        assert_eq!(
            account.apply(1, EventKind::Dispute),
            Err(EngineError::AlreadyDisputed)
        );
        assert_balances(&account, "0", "2");
    }
}
