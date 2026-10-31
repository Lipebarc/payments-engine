use std::collections::{HashMap, HashSet};

use crate::error::EngineError;
use crate::ledger::{Account, ClientId, Event, TxId};

/// Routes events to client accounts and enforces rules spanning all clients.
#[derive(Debug, Default)]
pub struct Engine {
    accounts: HashMap<ClientId, Account>,
    tx_ids: HashSet<TxId>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(&mut self, event: Event) -> Result<(), EngineError> {
        if event.kind.creates_transaction() {
            if self.tx_ids.contains(&event.tx) {
                return Err(EngineError::DuplicateTx);
            }
            self.accounts
                .entry(event.client)
                .or_insert_with(|| Account::new(event.client))
                .apply(event.tx, event.kind)?;
            self.tx_ids.insert(event.tx);
            Ok(())
        } else {
            // Referencing an unknown client cannot succeed, so no account is created for it.
            self.accounts
                .get_mut(&event.client)
                .ok_or(EngineError::UnknownTx)?
                .apply(event.tx, event.kind)
        }
    }

    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{Amount, EventKind};

    fn event(client: ClientId, tx: TxId, kind: EventKind) -> Event {
        Event { client, tx, kind }
    }

    fn deposit(client: ClientId, tx: TxId, amount: &str) -> Event {
        event(client, tx, EventKind::Deposit(amount.parse().unwrap()))
    }

    fn account(engine: &Engine, client: ClientId) -> &Account {
        engine.accounts().find(|a| a.client() == client).unwrap()
    }

    #[test]
    fn creates_accounts_on_first_transaction() {
        let mut engine = Engine::new();
        engine.process(deposit(2, 1, "1")).unwrap();
        engine.process(deposit(1, 2, "2")).unwrap();
        assert_eq!(engine.accounts().count(), 2);
        assert_eq!(
            account(&engine, 1).available(),
            "2".parse::<Amount>().unwrap()
        );
    }

    #[test]
    fn transaction_ids_are_unique_across_clients() {
        let mut engine = Engine::new();
        engine.process(deposit(1, 1, "1")).unwrap();
        assert_eq!(
            engine.process(deposit(2, 1, "5")),
            Err(EngineError::DuplicateTx)
        );
        assert_eq!(engine.accounts().count(), 1);
    }

    #[test]
    fn failed_transaction_does_not_reserve_its_id() {
        let mut engine = Engine::new();
        let failed = event(1, 1, EventKind::Withdrawal("1".parse().unwrap()));
        assert_eq!(engine.process(failed), Err(EngineError::InsufficientFunds));
        assert!(engine.process(deposit(1, 1, "1")).is_ok());
    }

    #[test]
    fn dispute_from_another_client_is_rejected() {
        let mut engine = Engine::new();
        engine.process(deposit(1, 1, "1")).unwrap();
        engine.process(deposit(2, 2, "1")).unwrap();
        assert_eq!(
            engine.process(event(2, 1, EventKind::Dispute)),
            Err(EngineError::UnknownTx)
        );
        assert_eq!(account(&engine, 1).held(), Amount::ZERO);
    }

    #[test]
    fn dispute_for_unknown_client_creates_no_account() {
        let mut engine = Engine::new();
        assert_eq!(
            engine.process(event(7, 1, EventKind::Dispute)),
            Err(EngineError::UnknownTx)
        );
        assert_eq!(engine.accounts().count(), 0);
    }
}
