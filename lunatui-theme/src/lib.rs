// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod bundled;
#[cfg(feature = "color-profiles")]
mod profile;
mod serde;
mod theme;
mod token;

pub use bundled::{BUNDLED_COUNT, BundledTheme, bundled_catalog, bundled_meta, load_bundled};
#[cfg(feature = "color-profiles")]
pub use profile::Profile;
pub use serde::{
    ThemeLoadError, theme_from_json, theme_from_json_strict, theme_from_toml,
    theme_from_toml_strict,
};
pub use theme::{Theme, Variant};
pub use token::Token;
