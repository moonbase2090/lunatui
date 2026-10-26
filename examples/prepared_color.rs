// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io::{self, Write};

use lunatui_core::Color;
use lunatui_core::paint::{PreparedRamp, RampOutput};

fn rgb(text: &str) -> io::Result<[u8; 3]> {
    if text.len() != 6 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "colors need six hex digits",
        ));
    }
    let value = u32::from_str_radix(text, 16)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let [_, red, green, blue] = value.to_be_bytes();
    Ok([red, green, blue])
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=3).contains(&args.len()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: prepared_color FROM_HEX TO_HEX [solid-index]",
        ));
    }
    let output = if let Some(index) = args.get(2) {
        let index = index
            .parse::<u8>()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        RampOutput::Solid(Color::Indexed(index))
    } else {
        RampOutput::TrueColor
    };
    let ramp = PreparedRamp::<80>::new(rgb(&args[0])?, rgb(&args[1])?, output)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let mut stdout = io::stdout().lock();
    for color in ramp.colors() {
        match color {
            Color::Rgb(red, green, blue) => write!(stdout, "\x1b[48;2;{red};{green};{blue}m ")?,
            Color::Indexed(index) => write!(stdout, "\x1b[48;5;{index}m ")?,
            Color::Default | Color::Token(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "example requires resolved colors",
                ));
            }
        }
    }
    writeln!(stdout, "\x1b[0m")
}
