// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_test::Clock;

#[test]
fn clock_advances_deterministically() {
    let mut clock = Clock::new();
    assert_eq!(clock.now(), Duration::ZERO);
    clock.advance(Duration::from_millis(16));
    clock.advance(Duration::from_millis(16));
    assert_eq!(clock.now(), Duration::from_millis(32));
}
