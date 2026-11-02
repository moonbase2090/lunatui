// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

#[cfg(unix)]
mod embedded_terminal_app;

#[cfg(unix)]
fn main() -> std::io::Result<()> {
    embedded_terminal_app::run()
}

#[cfg(not(unix))]
fn main() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "this live PTY example requires Unix",
    ))
}
