use super::amount::Amount;
use crate::error::EngineError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxKind {
    Deposit,
    Withdrawal,
}

/// Dispute lifecycle: `Settled -> Disputed -> Settled | ChargedBack`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxState {
    Settled,
    Disputed,
    ChargedBack,
}

/// Compact summary of every event applied to a transaction id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    kind: TxKind,
    amount: Amount,
    state: TxState,
}

impl Transaction {
    pub fn new(kind: TxKind, amount: Amount) -> Transaction {
        Transaction {
            kind,
            amount,
            state: TxState::Settled,
        }
    }

    pub fn kind(&self) -> TxKind {
        self.kind
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }

    pub fn state(&self) -> TxState {
        self.state
    }

    pub fn dispute(&mut self) -> Result<(), EngineError> {
        match self.state {
            TxState::Settled => {
                self.state = TxState::Disputed;
                Ok(())
            }
            TxState::Disputed => Err(EngineError::AlreadyDisputed),
            TxState::ChargedBack => Err(EngineError::AlreadyChargedBack),
        }
    }

    pub fn resolve(&mut self) -> Result<(), EngineError> {
        match self.state {
            TxState::Disputed => {
                self.state = TxState::Settled;
                Ok(())
            }
            TxState::Settled => Err(EngineError::NotDisputed),
            TxState::ChargedBack => Err(EngineError::AlreadyChargedBack),
        }
    }

    pub fn chargeback(&mut self) -> Result<(), EngineError> {
        match self.state {
            TxState::Disputed => {
                self.state = TxState::ChargedBack;
                Ok(())
            }
            TxState::Settled => Err(EngineError::NotDisputed),
            TxState::ChargedBack => Err(EngineError::AlreadyChargedBack),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deposit() -> Transaction {
        Transaction::new(TxKind::Deposit, "1".parse().unwrap())
    }

    fn withdrawal() -> Transaction {
        Transaction::new(TxKind::Withdrawal, "1".parse().unwrap())
    }

    #[test]
    fn dispute_then_resolve_returns_to_settled() {
        for mut tx in [deposit(), withdrawal()] {
            tx.dispute().unwrap();
            assert_eq!(tx.state(), TxState::Disputed);
            tx.resolve().unwrap();
            assert_eq!(tx.state(), TxState::Settled);
        }
    }

    #[test]
    fn resolved_transaction_can_be_disputed_again() {
        let mut tx = deposit();
        tx.dispute().unwrap();
        tx.resolve().unwrap();
        assert!(tx.dispute().is_ok());
    }

    #[test]
    fn chargeback_is_terminal() {
        let mut tx = deposit();
        tx.dispute().unwrap();
        tx.chargeback().unwrap();
        assert_eq!(tx.state(), TxState::ChargedBack);
        assert_eq!(tx.dispute(), Err(EngineError::AlreadyChargedBack));
        assert_eq!(tx.resolve(), Err(EngineError::AlreadyChargedBack));
        assert_eq!(tx.chargeback(), Err(EngineError::AlreadyChargedBack));
    }

    #[test]
    fn resolve_and_chargeback_require_dispute() {
        let mut tx = deposit();
        assert_eq!(tx.resolve(), Err(EngineError::NotDisputed));
        assert_eq!(tx.chargeback(), Err(EngineError::NotDisputed));
        assert_eq!(tx.state(), TxState::Settled);
    }

    #[test]
    fn cannot_dispute_twice() {
        let mut tx = deposit();
        tx.dispute().unwrap();
        assert_eq!(tx.dispute(), Err(EngineError::AlreadyDisputed));
    }

    #[test]
    fn withdrawals_can_be_charged_back() {
        let mut tx = withdrawal();
        tx.dispute().unwrap();
        tx.chargeback().unwrap();
        assert_eq!(tx.state(), TxState::ChargedBack);
    }
}
