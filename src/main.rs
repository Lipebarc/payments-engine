use std::fs::File;
use std::io::{self, BufReader};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use payments_engine::Engine;
use payments_engine::io::{EventReader, write_accounts};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let (Some(path), None) = (args.next(), args.next()) else {
        bail!("usage: payments-engine <transactions.csv>");
    };

    let file = File::open(&path).with_context(|| format!("cannot open {}", path.display()))?;
    let reader = EventReader::new(BufReader::new(file)).context("cannot read CSV header")?;

    let mut engine = Engine::new();
    for (index, result) in reader.enumerate() {
        // Line 1 is the header.
        let line = index + 2;
        match result {
            Ok(event) => {
                // Rejections are expected partner errors, so they are not reported.
                let _ = engine.process(event);
            }
            Err(err) => eprintln!("warning: skipping line {line}: {err}"),
        }
    }

    write_accounts(engine.accounts(), io::stdout().lock()).context("cannot write output")
}
