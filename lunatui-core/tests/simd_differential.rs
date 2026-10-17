// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::simd::{
    differential_levels, first_mismatch, first_mismatch_for_level, first_mismatch_scalar,
    runtime_dispatch_level,
};
use proptest::prelude::*;

#[test]
fn runtime_dispatch_is_stable() {
    let first = runtime_dispatch_level();
    let second = runtime_dispatch_level();
    assert_eq!(first, second);
}

#[test]
fn dispatched_matches_scalar_on_empty() {
    let empty: &[u128] = &[];
    assert_eq!(first_mismatch(empty, empty), None);
    assert_eq!(first_mismatch_scalar(empty, empty), None);
}

#[test]
fn unequal_length_reports_first_extra_index() {
    let a = [1_u128];
    let b = [1_u128, 2];
    assert_eq!(first_mismatch_scalar(&a, &b), Some(1));
    assert_eq!(first_mismatch(&a, &b), Some(1));
    for &level in differential_levels() {
        assert_eq!(first_mismatch_for_level(level)(&a, &b), Some(1));
    }
}

#[test]
fn dispatched_matches_scalar_on_mismatch() {
    let a = [1_u128, 2, 3];
    let b = [1_u128, 9, 3];
    assert_eq!(first_mismatch_scalar(&a, &b), Some(1));
    assert_eq!(first_mismatch(&a, &b), Some(1));
    for &level in differential_levels() {
        assert_eq!(first_mismatch_for_level(level)(&a, &b), Some(1));
    }
}

proptest! {
    #[test]
    fn all_levels_match_scalar_reference(
        a in prop::collection::vec(any::<u128>(), 0..512),
        b in prop::collection::vec(any::<u128>(), 0..512),
    ) {
        let reference = first_mismatch_scalar(&a, &b);
        prop_assert_eq!(first_mismatch(&a, &b), reference);
        for &level in differential_levels() {
            prop_assert_eq!(first_mismatch_for_level(level)(&a, &b), reference);
        }
    }
}
