// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Bracketed paste, streamed in bounded chunks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Paste {
    /// `CSI 200 ~` opened a paste.
    Start,
    /// A slice of the paste. Each slice is at most 1024 bytes.
    Chunk(String),
    /// `CSI 201 ~` closed the paste.
    End,
}

pub(crate) const CHUNK: usize = 1024;

pub(crate) fn encode_paste(paste: &Paste) -> Vec<u8> {
    match paste {
        Paste::Start => b"\x1b[200~".to_vec(),
        Paste::End => b"\x1b[201~".to_vec(),
        Paste::Chunk(text) => text.as_bytes().to_vec(),
    }
}
