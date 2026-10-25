// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

/// Hard ceiling for one discovery exchange, including a silent host.
pub const DISCOVERY_BUDGET: Duration = Duration::from_millis(200);

/// Maximum accepted capability major.minor for native discovery.
pub const MAX_CAPABILITY_VERSION: &str = "0.4";

/// Feature identifier that advertises shared-memory grid transport.
pub const NATIVE_GRID_FEATURE: &str = "native.grid.v1";

/// Maximum printable-ASCII APC body accepted by the dedicated decoder.
pub const MAX_APC_BODY: usize = 4096;

/// Whether the process may send a native capability query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum NativePolicy {
    /// No query is sent. Discovery reports unavailable. This is the default.
    #[default]
    Disabled,
    /// Direct-local opt-in: the caller may exchange a capability 0.4 query.
    DirectLocal,
}

/// Result of parsing host capability traffic for native.grid.v1.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub enum NativeDiscovery {
    /// Native setup is not available. ANSI remains the only renderer.
    Unavailable,
    /// Host advertised native.grid.v1 under capability 0.4 with authority keys.
    ///
    /// This is discovery only. It never grants active native ownership.
    Advertised {
        /// Correlated capability query id.
        id: u32,
        /// Single-use 128-bit offer from `native.offer`.
        offer: [u8; 16],
        /// Nonzero PTY authority generation from `native.generation`.
        generation: u64,
    },
}

/// Outcome of one discovery attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct DiscoveryResult {
    /// Parsed discovery state.
    pub discovery: NativeDiscovery,
    /// Bytes that are not consumed as a correlated capability reply.
    pub preserved_input: Vec<u8>,
}

impl DiscoveryResult {
    /// Rendering always uses ANSI in this cut. Native transport stays off.
    #[must_use]
    pub const fn uses_ansi(&self) -> bool {
        true
    }

    /// Native ownership is never active from discovery alone.
    #[must_use]
    pub const fn native_active(&self) -> bool {
        false
    }
}

/// Builds `ESC _ Prismattyc;cap;q;id=<id>;max=0.4 ESC \\`.
///
/// Returns `None` when `id` is zero.
#[must_use]
pub fn discovery_query(id: u32) -> Option<Vec<u8>> {
    if id == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(48);
    out.extend_from_slice(b"\x1b_Prismattyc;cap;q;id=");
    out.extend_from_slice(id.to_string().as_bytes());
    out.extend_from_slice(b";max=0.4\x1b\\");
    Some(out)
}

/// ANSI reset, clear screen, and home cursor used after native recovery.
#[must_use]
pub fn ansi_fallback_repaint_prefix() -> &'static [u8] {
    b"\x1bc\x1b[2J\x1b[H"
}

/// Incremental capability decoder for fragmented APC replies.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct DiscoveryDecoder {
    expected_id: u32,
    bytes: Vec<u8>,
}

impl DiscoveryDecoder {
    /// Decoder that correlates replies to `expected_id`.
    pub fn new(expected_id: u32) -> Self {
        Self {
            expected_id,
            bytes: Vec::new(),
        }
    }

    /// Appends `chunk` for later parsing.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);
    }

    /// Current discovery state after every complete sequence seen so far.
    pub fn snapshot(&self) -> DiscoveryResult {
        let (discovery, mut preserved, pending) = drain(self.expected_id, &self.bytes);
        preserved.extend_from_slice(&pending);
        DiscoveryResult {
            discovery,
            preserved_input: preserved,
        }
    }

    /// Finishes discovery, treating any incomplete tail as preserved input.
    pub fn finish(self) -> DiscoveryResult {
        self.snapshot()
    }
}

/// Runs discovery against already-read terminal bytes without touching a TTY.
///
/// When [`NativePolicy::Disabled`], returns [`NativeDiscovery::Unavailable`] and
/// preserves the entire input. When [`NativePolicy::DirectLocal`], parses for a
/// correlated capability 0.4 reply. An affirmative advertisement still leaves
/// [`DiscoveryResult::uses_ansi`] true and [`DiscoveryResult::native_active`] false.
pub fn discover_from_input(
    policy: NativePolicy,
    expected_id: u32,
    input: &[u8],
) -> DiscoveryResult {
    if policy != NativePolicy::DirectLocal || expected_id == 0 {
        return DiscoveryResult {
            discovery: NativeDiscovery::Unavailable,
            preserved_input: input.to_vec(),
        };
    }
    let mut decoder = DiscoveryDecoder::new(expected_id);
    decoder.feed(input);
    decoder.finish()
}

fn drain(expected_id: u32, input: &[u8]) -> (NativeDiscovery, Vec<u8>, Vec<u8>) {
    let mut preserved = Vec::new();
    let mut i = 0;
    let mut discovery = NativeDiscovery::Unavailable;
    while i < input.len() {
        if input[i] != 0x1b {
            preserved.push(input[i]);
            i += 1;
            continue;
        }
        if i + 1 >= input.len() {
            return (discovery, preserved, input[i..].to_vec());
        }
        match input[i + 1] {
            b'_' => match take_apc(&input[i..]) {
                ApcTake::Complete { body, total } => {
                    match classify_capability_reply(expected_id, body) {
                        CapClass::MatchedAdvertised {
                            id,
                            offer,
                            generation,
                        } => {
                            discovery = NativeDiscovery::Advertised {
                                id,
                                offer,
                                generation,
                            };
                            i += total;
                        }
                        CapClass::MatchedUnavailable | CapClass::StaleMatched => {
                            i += total;
                        }
                        CapClass::Unrelated => {
                            preserved.extend_from_slice(&input[i..i + total]);
                            i += total;
                        }
                        CapClass::Malformed => {
                            discovery = NativeDiscovery::Unavailable;
                            i += total;
                        }
                    }
                }
                ApcTake::Malformed { total } => {
                    discovery = NativeDiscovery::Unavailable;
                    i += total;
                }
                ApcTake::Incomplete => {
                    return (discovery, preserved, input[i..].to_vec());
                }
            },
            b'P' => match take_string_seq(&input[i..], b'P') {
                Some(total) => {
                    preserved.extend_from_slice(&input[i..i + total]);
                    i += total;
                }
                None => return (discovery, preserved, input[i..].to_vec()),
            },
            b']' => match take_string_seq(&input[i..], b']') {
                Some(total) => {
                    preserved.extend_from_slice(&input[i..i + total]);
                    i += total;
                }
                None => return (discovery, preserved, input[i..].to_vec()),
            },
            b'[' => match take_csi(&input[i..]) {
                Some(total) => {
                    preserved.extend_from_slice(&input[i..i + total]);
                    i += total;
                }
                None => return (discovery, preserved, input[i..].to_vec()),
            },
            _ => {
                preserved.push(input[i]);
                i += 1;
            }
        }
    }
    (discovery, preserved, Vec::new())
}

enum ApcTake<'a> {
    Complete { body: &'a [u8], total: usize },
    Malformed { total: usize },
    Incomplete,
}

fn take_apc(bytes: &[u8]) -> ApcTake<'_> {
    if bytes.len() < 2 || bytes[0] != 0x1b || bytes[1] != b'_' {
        return ApcTake::Incomplete;
    }
    let mut idx = 2;
    while idx < bytes.len() {
        if bytes[idx] == 0x1b {
            if idx + 1 >= bytes.len() {
                return ApcTake::Incomplete;
            }
            if bytes[idx + 1] == b'\\' {
                let body = &bytes[2..idx];
                let total = idx + 2;
                if body.len() > MAX_APC_BODY || !body.iter().all(|b| b.is_ascii() && *b >= 0x20) {
                    return ApcTake::Malformed { total };
                }
                return ApcTake::Complete { body, total };
            }
            return ApcTake::Malformed { total: idx + 1 };
        }
        if bytes[idx] == 0x07 {
            return ApcTake::Malformed { total: idx + 1 };
        }
        idx += 1;
    }
    ApcTake::Incomplete
}

fn take_string_seq(bytes: &[u8], intro: u8) -> Option<usize> {
    if bytes.len() < 2 || bytes[0] != 0x1b || bytes[1] != intro {
        return None;
    }
    let mut idx = 2;
    while idx < bytes.len() {
        if bytes[idx] == 0x1b {
            if idx + 1 >= bytes.len() {
                return None;
            }
            if bytes[idx + 1] == b'\\' {
                return Some(idx + 2);
            }
            return Some(idx + 1);
        }
        if bytes[idx] == 0x07 {
            return Some(idx + 1);
        }
        idx += 1;
    }
    None
}

fn take_csi(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 2 || bytes[0] != 0x1b || bytes[1] != b'[' {
        return None;
    }
    let mut idx = 2;
    while idx < bytes.len() {
        let b = bytes[idx];
        if (0x40..=0x7e).contains(&b) {
            return Some(idx + 1);
        }
        idx += 1;
    }
    None
}

enum CapClass {
    MatchedAdvertised {
        id: u32,
        offer: [u8; 16],
        generation: u64,
    },
    MatchedUnavailable,
    StaleMatched,
    Unrelated,
    Malformed,
}

fn classify_capability_reply(expected_id: u32, body: &[u8]) -> CapClass {
    let Ok(text) = std::str::from_utf8(body) else {
        return CapClass::Malformed;
    };
    let mut parts = text.split(';');
    if parts.next() != Some("Prismattyc") {
        return CapClass::Unrelated;
    }
    if parts.next() != Some("cap") {
        return CapClass::Unrelated;
    }
    if parts.next() != Some("r") {
        return CapClass::Malformed;
    }

    let mut id: Option<u32> = None;
    let mut version: Option<&str> = None;
    let mut features: Option<&str> = None;
    let mut offer: Option<[u8; 16]> = None;
    let mut generation: Option<u64> = None;

    for part in parts {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key {
            "id" => {
                if id.is_some() {
                    return CapClass::Malformed;
                }
                let Ok(parsed) = value.parse::<u32>() else {
                    return CapClass::Malformed;
                };
                if parsed == 0 {
                    return CapClass::Malformed;
                }
                id = Some(parsed);
            }
            "v" => {
                if version.is_some() {
                    return CapClass::Malformed;
                }
                if !valid_capability_version(value) {
                    return CapClass::Malformed;
                }
                version = Some(value);
            }
            "features" => {
                if features.is_some() {
                    return CapClass::Malformed;
                }
                if value.is_empty() || value.split(',').any(str::is_empty) {
                    return CapClass::Malformed;
                }
                features = Some(value);
            }
            "native.offer" => {
                if offer.is_some() {
                    return CapClass::Malformed;
                }
                let Some(parsed) = parse_offer(value) else {
                    return CapClass::Malformed;
                };
                offer = Some(parsed);
            }
            "native.generation" => {
                if generation.is_some() {
                    return CapClass::Malformed;
                }
                let Ok(parsed) = value.parse::<u64>() else {
                    return CapClass::Malformed;
                };
                if parsed == 0 {
                    return CapClass::Malformed;
                }
                generation = Some(parsed);
            }
            _ => {}
        }
    }
    finish_cap_class(expected_id, id, version, features, offer, generation)
}

fn finish_cap_class(
    expected_id: u32,
    id: Option<u32>,
    version: Option<&str>,
    features: Option<&str>,
    offer: Option<[u8; 16]>,
    generation: Option<u64>,
) -> CapClass {
    let Some(got_id) = id else {
        return CapClass::Malformed;
    };
    let Some(ver) = version else {
        return CapClass::Malformed;
    };
    let Some(feat) = features else {
        return CapClass::Malformed;
    };
    if got_id != expected_id {
        return CapClass::StaleMatched;
    }
    if ver != "0.4" {
        return CapClass::MatchedUnavailable;
    }
    let has_native = feat.split(',').any(|f| f == NATIVE_GRID_FEATURE);
    if !has_native {
        return CapClass::MatchedUnavailable;
    }
    let (Some(offer), Some(generation)) = (offer, generation) else {
        return CapClass::MatchedUnavailable;
    };
    CapClass::MatchedAdvertised {
        id: got_id,
        offer,
        generation,
    }
}

fn valid_capability_version(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(major) = parts.next() else {
        return false;
    };
    let Some(minor) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    let Ok(major) = major.parse::<u32>() else {
        return false;
    };
    let Ok(minor) = minor.parse::<u32>() else {
        return false;
    };
    major == 0 && minor <= 4
}

fn parse_offer(value: &str) -> Option<[u8; 16]> {
    if value.len() != 32 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, chunk) in value.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_policy_preserves_input_and_stays_unavailable() {
        let input = b"\x1b_Prismattyc;cap;q;id=17;max=0.4\x1b\\";
        let result = discover_from_input(NativePolicy::Disabled, 17, input);
        assert_eq!(result.discovery, NativeDiscovery::Unavailable);
        assert_eq!(result.preserved_input, input);
        assert!(result.uses_ansi());
        assert!(!result.native_active());
    }

    #[test]
    fn query_bytes_match_envelope() {
        assert_eq!(
            discovery_query(17).as_deref(),
            Some(b"\x1b_Prismattyc;cap;q;id=17;max=0.4\x1b\\".as_slice())
        );
        assert_eq!(discovery_query(0), None);
    }
}
