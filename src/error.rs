use thiserror::Error;

/// A CSV record that cannot be turned into an event.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error("missing amount")]
    MissingAmount,
    #[error("invalid amount `{0}`")]
    InvalidAmount(String),
}

/// A well-formed event rejected by the business rules.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    #[error("account is locked")]
    AccountLocked,
    #[error("insufficient available funds")]
    InsufficientFunds,
    #[error("amount must be positive")]
    NonPositiveAmount,
    #[error("transaction id already used")]
    DuplicateTx,
    #[error("transaction not found for this client")]
    UnknownTx,
    #[error("transaction is already under dispute")]
    AlreadyDisputed,
    #[error("transaction is not under dispute")]
    NotDisputed,
    #[error("transaction was already charged back")]
    AlreadyChargedBack,
    #[error("amount overflow")]
    Overflow,
}
