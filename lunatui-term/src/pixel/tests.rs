// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::push_base64;

fn encoded(data: &[u8]) -> String {
    let mut out = Vec::new();
    push_base64(&mut out, data);
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn base64_matches_the_rfc_4648_vectors() {
    assert_eq!(encoded(b""), "");
    assert_eq!(encoded(b"f"), "Zg==");
    assert_eq!(encoded(&[b'f', b'o']), "Zm8=");
    assert_eq!(encoded(b"foo"), "Zm9v");
    assert_eq!(encoded(b"foob"), "Zm9vYg==");
    assert_eq!(encoded(b"fooba"), "Zm9vYmE=");
    assert_eq!(encoded(b"foobar"), "Zm9vYmFy");
}

#[test]
fn base64_uses_the_url_unsafe_alphabet_ends() {
    assert_eq!(encoded(&[0xfb, 0xff, 0xbf]), "+/+/");
    assert_eq!(encoded(&[0xff, 0xff, 0xff]), "////");
    assert_eq!(encoded(&[0x00, 0x00, 0x00]), "AAAA");
}
