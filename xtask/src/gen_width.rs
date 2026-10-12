// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod table;
mod ucd;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::git;
use table::{Sources, classify, render};

const UNICODE_VERSION: (u8, u8, u8) = (18, 0, 0);
const OUTPUT: &str = "lunatui-core/src/width/tables.rs";
const UCD_FILES: [&str; 5] = [
    "EastAsianWidth.txt",
    "extracted/DerivedGeneralCategory.txt",
    "DerivedCoreProperties.txt",
    "emoji/emoji-data.txt",
    "HangulSyllableType.txt",
];

pub(crate) fn run() -> ExitCode {
    match generate() {
        Ok(summary) => {
            println!("xtask gen-width: {summary}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("xtask gen-width: {error}");
            ExitCode::FAILURE
        }
    }
}

fn generate() -> io::Result<String> {
    git::enter_toplevel()?;
    let (major, minor, update) = UNICODE_VERSION;
    let version = format!("{major}.{minor}.{update}");
    let cache = Path::new("target/ucd").join(&version);
    let mut texts = Vec::new();
    for file in UCD_FILES {
        texts.push(fs::read_to_string(download(&version, &cache, file)?)?);
    }
    let sources = Sources {
        east_asian_width: &texts[0],
        general_category: &texts[1],
        core_properties: &texts[2],
        emoji_data: &texts[3],
        hangul_syllable_type: &texts[4],
    };
    let tables = classify(&sources)?;
    if let Some(parent) = Path::new(OUTPUT).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(OUTPUT, render(&tables, UNICODE_VERSION))?;
    Ok(format!(
        "wrote {OUTPUT} from Unicode {version}: {} zero-width and {} wide ranges",
        tables.zero.len(),
        tables.wide.len()
    ))
}

fn download(version: &str, cache: &Path, file: &str) -> io::Result<PathBuf> {
    let path = cache.join(file);
    if path.is_file() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let url = format!("https://www.unicode.org/Public/{version}/ucd/{file}");
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--output",
        ])
        .arg(&path)
        .arg(&url)
        .status()?;
    if status.success() {
        Ok(path)
    } else {
        Err(io::Error::other(format!(
            "could not download {url} ({status})"
        )))
    }
}
