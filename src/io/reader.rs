use std::io::Read;

use csv::{ReaderBuilder, StringRecord, Trim};
use serde::Deserialize;

use crate::error::ParseError;
use crate::ledger::{Amount, ClientId, Event, EventKind, TxId};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RecordType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// Raw CSV row; borrows from the reused record buffer to avoid per-row allocation.
#[derive(Debug, Deserialize)]
struct Record<'a> {
    #[serde(rename = "type")]
    kind: RecordType,
    client: ClientId,
    tx: TxId,
    amount: Option<&'a str>,
}

impl TryFrom<Record<'_>> for Event {
    type Error = ParseError;

    fn try_from(record: Record<'_>) -> Result<Self, Self::Error> {
        let amount = || -> Result<Amount, ParseError> {
            record.amount.ok_or(ParseError::MissingAmount)?.parse()
        };
        let kind = match record.kind {
            RecordType::Deposit => EventKind::Deposit(amount()?),
            RecordType::Withdrawal => EventKind::Withdrawal(amount()?),
            RecordType::Dispute => EventKind::Dispute,
            RecordType::Resolve => EventKind::Resolve,
            RecordType::Chargeback => EventKind::Chargeback,
        };
        Ok(Event {
            client: record.client,
            tx: record.tx,
            kind,
        })
    }
}

/// Streams events from CSV input one row at a time.
pub struct EventReader<R: Read> {
    reader: csv::Reader<R>,
    headers: StringRecord,
    record: StringRecord,
}

impl<R: Read> EventReader<R> {
    pub fn new(input: R) -> Result<Self, ParseError> {
        let mut reader = ReaderBuilder::new()
            .trim(Trim::All) // removes whitespace from all fields, including the type field
            .flexible(true) // allows rows with missing fields, which are reported as errors in `next()`
            .from_reader(input);
        let headers = reader.headers()?.clone();
        Ok(Self {
            reader,
            headers,
            record: StringRecord::new(),
        })
    }
}

impl<R: Read> Iterator for EventReader<R> {
    type Item = Result<Event, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(&mut self.record) {
            Ok(false) => None,
            Ok(true) => Some(
                self.record
                    .deserialize::<Record>(Some(&self.headers))
                    .map_err(ParseError::from)
                    .and_then(Event::try_from),
            ),
            Err(err) => Some(Err(err.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(input: &str) -> Vec<Result<Event, ParseError>> {
        EventReader::new(input.as_bytes()).unwrap().collect()
    }

    fn amt(s: &str) -> Amount {
        s.parse().unwrap()
    }

    #[test]
    fn parses_all_event_types_with_whitespace() {
        let input = "type, client, tx, amount\n\
                     deposit, 1, 1, 1.0\n\
                     withdrawal,  2 , 2 ,  0.5\n\
                     dispute, 1, 1,\n\
                     resolve, 1, 1\n\
                     chargeback, 1, 1, \n";
        let events: Vec<Event> = read(input).into_iter().map(Result::unwrap).collect();
        let kinds: Vec<EventKind> = events.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            [
                EventKind::Deposit(amt("1")),
                EventKind::Withdrawal(amt("0.5")),
                EventKind::Dispute,
                EventKind::Resolve,
                EventKind::Chargeback,
            ]
        );
        assert_eq!((events[1].client, events[1].tx), (2, 2));
    }

    #[test]
    fn malformed_rows_are_reported_without_stopping_the_stream() {
        let input = "type,client,tx,amount\n\
                     deposit,1,1,\n\
                     teleport,1,2,1\n\
                     deposit,70000,3,1\n\
                     deposit,1,4,1.00001\n\
                     deposit,1,5,2\n";
        let results = read(input);
        assert_eq!(results.len(), 5);
        assert!(matches!(results[0], Err(ParseError::MissingAmount)));
        assert!(matches!(results[1], Err(ParseError::Csv(_))));
        assert!(matches!(results[2], Err(ParseError::Csv(_))));
        assert!(matches!(results[3], Err(ParseError::InvalidAmount(_))));
        assert!(results[4].is_ok());
    }
}
