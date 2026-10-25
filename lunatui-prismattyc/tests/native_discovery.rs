// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_prismattyc::{
    DiscoveryDecoder, NativeDiscovery, NativePolicy, ansi_fallback_repaint_prefix,
    discover_from_input, discovery_query,
};

fn hex(bytes: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.as_bytes().chunks(2) {
        if pair.len() != 2 {
            break;
        }
        let value = match (nibble(pair[0]), nibble(pair[1])) {
            (Some(hi), Some(lo)) => (hi << 4) | lo,
            _ => 0,
        };
        out.push(value);
    }
    out
}

fn nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn affirmative_0_4() -> Vec<u8> {
    let offer = b"0123456789abcdef0123456789abcdef";
    let mut body = b"Prismattyc;cap;r;id=17;v=0.4;features=native.grid.v1;native.offer=".to_vec();
    body.extend_from_slice(offer);
    body.extend_from_slice(b";native.generation=7");
    let mut out = Vec::new();
    out.extend_from_slice(b"\x1b_");
    out.extend_from_slice(&body);
    out.extend_from_slice(b"\x1b\\");
    out
}

#[test]
fn silence_is_unavailable_and_ansi() {
    let result = discover_from_input(NativePolicy::DirectLocal, 17, b"");
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
    assert!(result.uses_ansi());
    assert!(!result.native_active());
}

#[test]
fn actual_host_0_3_reply_is_unavailable() {
    let input = hex(include_str!("fixtures/actual_host_0_3.hex").trim());
    let result = discover_from_input(NativePolicy::DirectLocal, 17, &input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
    assert!(result.uses_ansi());
    assert!(!result.native_active());
}

#[test]
fn actual_host_0_3_after_da_preserves_da() {
    let mut input = b"\x1b[?62c".to_vec();
    input.extend_from_slice(&hex(include_str!("fixtures/actual_host_0_3.hex").trim()));
    let result = discover_from_input(NativePolicy::DirectLocal, 17, &input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
    assert_eq!(result.preserved_input, b"\x1b[?62c");
}

#[test]
fn actual_host_0_3_with_keys_preserves_user_bytes() {
    let mut input = b"a".to_vec();
    input.extend_from_slice(&hex(include_str!("fixtures/actual_host_0_3.hex").trim()));
    input.push(b'b');
    let result = discover_from_input(NativePolicy::DirectLocal, 17, &input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
    assert_eq!(result.preserved_input, b"ab");
}

#[test]
fn wrong_id_is_stale_and_unavailable() {
    let input = b"\x1b_Prismattyc;cap;r;id=18;v=0.3;features=rich.tree.v1\x1b\\";
    let result = discover_from_input(NativePolicy::DirectLocal, 17, input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
}

#[test]
fn truncated_apc_is_unavailable_and_preserved_as_tail() {
    let full = hex(include_str!("fixtures/actual_host_0_3.hex").trim());
    let input = &full[..full.len().saturating_sub(1)];
    let result = discover_from_input(NativePolicy::DirectLocal, 17, input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
    assert_eq!(result.preserved_input, input);
}

#[test]
fn duplicate_id_is_malformed_unavailable() {
    let input = b"\x1b_Prismattyc;cap;r;id=17;id=18;v=0.3;features=rich.tree.v1\x1b\\";
    let result = discover_from_input(NativePolicy::DirectLocal, 17, input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
}

#[test]
fn identity_without_capability_is_unavailable() {
    let input = b"\x1bP>|Prismattyc(0.3)\x1b\\";
    let result = discover_from_input(NativePolicy::DirectLocal, 17, input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
    assert_eq!(result.preserved_input, input);
}

#[test]
fn unregistered_native_marker_in_0_3_is_unavailable() {
    let input = b"\x1b_Prismattyc;cap;r;id=17;v=0.3;features=native.grid.v1\x1b\\";
    let result = discover_from_input(NativePolicy::DirectLocal, 17, input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
    assert!(result.uses_ansi());
}

#[test]
fn disabled_policy_ignores_affirmative_advertisement() {
    let input = affirmative_0_4();
    let result = discover_from_input(NativePolicy::Disabled, 17, &input);
    assert_eq!(result.discovery, NativeDiscovery::Unavailable);
    assert_eq!(result.preserved_input, input);
    assert!(result.uses_ansi());
    assert!(!result.native_active());
}

#[test]
fn affirmative_0_4_is_advertised_but_ansi_only() {
    let input = affirmative_0_4();
    let result = discover_from_input(NativePolicy::DirectLocal, 17, &input);
    assert_eq!(
        result.discovery,
        NativeDiscovery::Advertised {
            id: 17,
            offer: [
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
                0xcd, 0xef
            ],
            generation: 7,
        }
    );
    assert!(result.uses_ansi());
    assert!(!result.native_active());
}

#[test]
fn fragmented_affirmative_0_4_still_advertises() {
    let input = affirmative_0_4();
    let mut decoder = DiscoveryDecoder::new(17);
    for chunk in input.chunks(3) {
        decoder.feed(chunk);
    }
    let result = decoder.finish();
    assert!(matches!(
        result.discovery,
        NativeDiscovery::Advertised { .. }
    ));
    assert!(result.uses_ansi());
    assert!(!result.native_active());
}

#[test]
fn encoder_transcript_unchanged_after_discovery_and_fallback_prefix() {
    let bytes = b"\x1b[1;1HAB\x1b[m\x1b[m\x1b[m\x1b[0m";
    let discovery = discover_from_input(NativePolicy::DirectLocal, 17, &affirmative_0_4());
    assert!(matches!(
        discovery.discovery,
        NativeDiscovery::Advertised { .. }
    ));
    assert!(discovery.uses_ansi());
    assert!(!discovery.native_active());

    let mut recovered = ansi_fallback_repaint_prefix().to_vec();
    recovered.extend_from_slice(bytes);
    assert_eq!(
        recovered,
        b"\x1bc\x1b[2J\x1b[H\x1b[1;1HAB\x1b[m\x1b[m\x1b[m\x1b[0m"
    );
}

#[test]
fn discovery_query_is_nonzero_id_envelope() {
    assert_eq!(
        discovery_query(17).as_deref(),
        Some(b"\x1b_Prismattyc;cap;q;id=17;max=0.4\x1b\\".as_slice())
    );
    assert_eq!(discovery_query(0), None);
}
