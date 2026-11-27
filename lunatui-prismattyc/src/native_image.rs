// SPDX-License-Identifier: MIT
//! Native Prismattyc image transport for direct local sessions.

pub use crate::client::backing::PreparedImage;
pub use crate::client::images::Images as ImageSink;
pub use crate::client::images::Key as ImageKey;
pub use crate::client::session::{Offer, Session};

/// A native image offer parsed from Prismattyc capability discovery.
pub type ImageOffer = Offer;

/// An active native image session that publishes complete grid and image snapshots.
pub type ImageSession = Session;

/// The native presenter used during a render pass.
pub type ImagePresenter = ImageSink;

/// Result of one terminal capability query, including unrelated bytes read with its reply.
pub struct Discovery {
    /// One-use native offer when the direct local host advertised image support.
    pub offer: Option<ImageOffer>,
    /// Terminal input that was interleaved with capability discovery.
    pub preserved_input: Vec<u8>,
}

/// Queries a direct local host on the terminal and returns its one-use image offer.
///
/// The terminal must be interactive. The caller must perform discovery before opening its
/// input reader so the correlated capability reply is consumed here. Unrelated bytes are
/// returned for the caller to prepend to its input reader.
///
/// # Errors
///
/// Returns an error when the terminal query cannot be written or read.
pub fn discover(tty: &mut std::fs::File) -> std::io::Result<Discovery> {
    use std::{io::Write, os::fd::AsRawFd, time::Instant};

    let mut random = [0; 4];
    getrandom::fill(&mut random).map_err(|error| std::io::Error::other(error.to_string()))?;
    let id = u32::from_le_bytes(random).max(1);
    let query =
        crate::discovery_query(id).ok_or_else(|| crate::invalid("native capability query id"))?;
    tty.write_all(&query)?;
    let bytes = read_capability_reply(tty.as_raw_fd(), Instant::now() + crate::DISCOVERY_BUDGET)?;
    let mut decoder = crate::DiscoveryDecoder::new(id);
    decoder.feed(&bytes);
    let result = decoder.finish();
    let offer = match result.discovery {
        crate::NativeDiscovery::Advertised {
            id,
            offer,
            generation,
        } => Some(ImageOffer {
            id,
            epoch: generation,
            value: offer,
        }),
        _ => None,
    };
    Ok(Discovery {
        offer,
        preserved_input: result.preserved_input,
    })
}

/// Opens the inherited control endpoint when Prismattyc supplied one to this child.
///
/// # Errors
///
/// Returns an error if the descriptor environment is malformed or cannot be duplicated.
pub fn inherited_stream() -> std::io::Result<Option<std::os::unix::net::UnixStream>> {
    crate::client::control::inherited_stream()
}

fn read_capability_reply(
    descriptor: std::os::fd::RawFd,
    deadline: std::time::Instant,
) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while bytes.len() < crate::MAX_APC_BODY {
        let now = std::time::Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline.saturating_duration_since(now).as_millis().max(1);
        let milliseconds = i32::try_from(remaining.min(i32::MAX as u128)).unwrap_or(i32::MAX);
        let mut poll = libc::pollfd {
            fd: descriptor,
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: poll reads one initialized pollfd for a valid terminal descriptor.
        let result = unsafe { libc::poll(&raw mut poll, 1, milliseconds) };
        if result == 0 {
            break;
        }
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        let mut byte = 0;
        // SAFETY: read writes at most one byte into `byte` using the borrowed TTY descriptor.
        let count = unsafe { libc::read(descriptor, (&raw mut byte).cast(), 1) };
        if count == 0 {
            break;
        }
        if count < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        bytes.push(byte);
        if has_complete_capability_reply(&bytes) {
            break;
        }
    }
    Ok(bytes)
}

fn has_complete_capability_reply(bytes: &[u8]) -> bool {
    let prefix = b"\x1b_Prismattyc;cap;r;";
    bytes
        .windows(prefix.len())
        .position(|window| window == prefix)
        .is_some_and(|start| {
            bytes[start + prefix.len()..]
                .windows(2)
                .any(|pair| pair == b"\x1b\\")
        })
}
