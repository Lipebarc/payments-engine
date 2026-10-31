# payments-engine

A streaming payments processing engine that reads a CSV of transactions, maintains client account balances, handles disputes/chargebacks, and outputs final account states.

## Features

- Exact decimal arithmetic (4 decimal places) using `rust_decimal`
- Streaming processing — does not load the entire input into memory
- Type-safe state machine for transaction lifecycle (deposit, withdrawal, dispute, resolve, chargeback)
- Proper error handling for invalid transitions, insufficient funds, and malformed input

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run -- transactions.csv > accounts.csv
```

The program accepts a single CSV file as its only argument and writes the resulting account balances to stdout in CSV format.

## Output

```
client,available,held,total,locked
1,1.5000,0.0000,1.5000,false
2,2.0000,0.0000,2.0000,false
```

Row order is not guaranteed. Spacing and trailing zeros on whole numbers are normalized on output.

## Tests

```bash
cargo test
```