// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::simd::{
    differential_levels, first_mismatch, first_mismatch_for_level, first_mismatch_scalar,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn row_word_first_mismatch_matches_scalar(
        len in 1usize..4096,
        seed in any::<u64>(),
    ) {
        let left = vec![0_u128; len];
        let mut right = left.clone();
        let flip = usize::try_from(seed).unwrap_or(0) % len;
        right[flip] = u128::from(seed) + 1;
        let reference = first_mismatch_scalar(&left, &right);
        prop_assert_eq!(first_mismatch(&left, &right), reference);
        for &level in differential_levels() {
            prop_assert_eq!(first_mismatch_for_level(level)(&left, &right), reference);
        }
    }
}
