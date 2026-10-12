// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../lunatui-term/README.md")]

use std::io::{self, Read};
use std::process::ExitCode;

use lunatui_term::fuzz_input_parser;

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
            fuzz_input_parser(&data).map_err(str::to_owned)?;
            Ok(format!(
                "fuzz_input_parser: stdin {} bytes clean",
                data.len()
            ))
        }
        Some("--execs") => {
            let count = args
                .next()
                .ok_or_else(|| "fuzz_input_parser: --execs needs a count".to_owned())?;
            let execs = count
                .parse::<u64>()
                .map_err(|_| format!("fuzz_input_parser: bad count {count}"))?;
            campaign(execs)?;
            Ok(format!("fuzz_input_parser: {execs} execs clean"))
        }
        Some(other) => Err(format!("fuzz_input_parser: unknown argument {other}")),
    }
}

fn campaign(execs: u64) -> Result<(), String> {
    let mut state = 0xA5A5_1234_u64;
    let mut bytes = Vec::new();
    for _ in 0..execs {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let Ok(byte) = u8::try_from((state >> 33) & 0xff) else {
            continue;
        };
        bytes.push(byte);
        if bytes.len() == 256 {
            fuzz_input_parser(&bytes).map_err(str::to_owned)?;
            bytes.clear();
        }
    }
    fuzz_input_parser(&bytes).map_err(str::to_owned)
}
