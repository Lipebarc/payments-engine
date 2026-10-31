use std::io::Write;

use serde::Serialize;

use crate::ledger::{Account, ClientId};

const HEADER: [&str; 5] = ["client", "available", "held", "total", "locked"];

#[derive(Debug, Serialize)]
struct AccountRow {
    client: ClientId,
    available: String,
    held: String,
    total: String,
    locked: bool,
}

impl From<&Account> for AccountRow {
    fn from(account: &Account) -> Self {
        Self {
            client: account.client(),
            available: account.available().to_string(),
            held: account.held().to_string(),
            total: account.total().to_string(),
            locked: account.is_locked(),
        }
    }
}

pub fn write_accounts<'a, W: Write>(
    accounts: impl IntoIterator<Item = &'a Account>,
    output: W,
) -> Result<(), csv::Error> {
    // Header is written manually so it appears even when there are no accounts.
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(output);
    writer.write_record(HEADER)?;
    for account in accounts {
        writer.serialize(AccountRow::from(account))?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::EventKind;

    #[test]
    fn writes_header_and_four_decimal_places() {
        let mut account = Account::new(1);
        account
            .apply(1, EventKind::Deposit("1.5".parse().unwrap()))
            .unwrap();
        let mut out = Vec::new();
        write_accounts([&account], &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "client,available,held,total,locked\n1,1.5000,0.0000,1.5000,false\n"
        );
    }

    #[test]
    fn writes_header_even_without_accounts() {
        let mut out = Vec::new();
        write_accounts([], &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "client,available,held,total,locked\n"
        );
    }
}
