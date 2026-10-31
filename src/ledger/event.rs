use super::amount::Amount;

pub type ClientId = u16;
pub type TxId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub client: ClientId,
    pub tx: TxId,
    pub kind: EventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Deposit(Amount),
    Withdrawal(Amount),
    Dispute,
    Resolve,
    Chargeback,
}

impl EventKind {
    pub fn creates_transaction(self) -> bool {
        matches!(self, Self::Deposit(_) | Self::Withdrawal(_))
    }
}
