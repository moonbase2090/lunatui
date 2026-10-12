// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../lunatui-test/README.md")]

use std::io::{self, Read};
use std::process::ExitCode;

use lunatui_test::{fuzz_bytes, fuzz_campaign};

fn main() -> ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => {
            let mut data = Vec::new();
            io::stdin()
                .read_to_end(&mut data)
                .map_err(|err| err.to_string())?;
            fuzz_bytes(&data).map_err(|err| err.to_string())?;
            Ok(format!("fuzz_encoder: stdin {} bytes clean", data.len()))
        }
        Some("--execs") => {
            let count = args
                .next()
                .ok_or_else(|| "fuzz_encoder: --execs needs a count".to_owned())?;
            let execs = count
                .parse::<u64>()
                .map_err(|_| format!("fuzz_encoder: bad count {count}"))?;
            fuzz_campaign(execs).map_err(|err| err.to_string())?;
            Ok(format!("fuzz_encoder: {execs} execs clean"))
        }
        Some(other) => Err(format!("fuzz_encoder: unknown argument {other}")),
    }
}
