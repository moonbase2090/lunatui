// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(feature = "color-profiles")]
mod profile;

#[cfg(feature = "color-profiles")]
pub use profile::Profile;
