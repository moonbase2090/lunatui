// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
