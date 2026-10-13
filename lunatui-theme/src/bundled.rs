// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use crate::serde::{ThemeLoadError, theme_from_toml_strict};
use crate::theme::Theme;

/// Metadata for one embedded theme file.
#[derive(Clone, Copy, Debug)]
pub struct BundledTheme {
    /// Stable id (filename stem).
    pub id: &'static str,
    /// Display name from the Prismattyc catalog.
    pub name: &'static str,
}

const ENTRIES: &[(&str, &str, &str)] = &[
    (
        "catppuccin-mocha",
        "Catppuccin Mocha",
        include_str!("../themes/catppuccin-mocha.toml"),
    ),
    ("dracula", "Dracula", include_str!("../themes/dracula.toml")),
    ("ghost", "Ghost", include_str!("../themes/ghost.toml")),
    (
        "hive-analogous",
        "Hive Analogous",
        include_str!("../themes/hive-analogous.toml"),
    ),
    (
        "hive-complementary",
        "Hive Complementary",
        include_str!("../themes/hive-complementary.toml"),
    ),
    (
        "hive-high-contrast-dual",
        "Hive High Contrast Dual",
        include_str!("../themes/hive-high-contrast-dual.toml"),
    ),
    (
        "hive-monochromatic",
        "Hive Monochromatic",
        include_str!("../themes/hive-monochromatic.toml"),
    ),
    (
        "hive-monochromatic-light",
        "Hive Monochromatic Light",
        include_str!("../themes/hive-monochromatic-light.toml"),
    ),
    (
        "hive-muted-professional",
        "Hive Muted Professional",
        include_str!("../themes/hive-muted-professional.toml"),
    ),
    (
        "hive-muted-professional-light",
        "Hive Muted Professional Light",
        include_str!("../themes/hive-muted-professional-light.toml"),
    ),
    (
        "hive-night-hive",
        "Hive Night Hive",
        include_str!("../themes/hive-night-hive.toml"),
    ),
    (
        "hive-split-complementary",
        "Hive Split Complementary",
        include_str!("../themes/hive-split-complementary.toml"),
    ),
    (
        "hive-tri-tone",
        "Hive Tri-Tone",
        include_str!("../themes/hive-tri-tone.toml"),
    ),
    (
        "hive-tri-tone-light",
        "Hive Tri-Tone Light",
        include_str!("../themes/hive-tri-tone-light.toml"),
    ),
    (
        "hive-triadic",
        "Hive Triadic",
        include_str!("../themes/hive-triadic.toml"),
    ),
    (
        "hive-two-tone",
        "Hive Two-Tone",
        include_str!("../themes/hive-two-tone.toml"),
    ),
    (
        "japanesque",
        "Japanesque",
        include_str!("../themes/japanesque.toml"),
    ),
    (
        "monokai-dimmed",
        "Monokai Dimmed",
        include_str!("../themes/monokai-dimmed.toml"),
    ),
    (
        "monokai-pro",
        "Monokai Pro",
        include_str!("../themes/monokai-pro.toml"),
    ),
    (
        "monokai-remastered",
        "Monokai Remastered",
        include_str!("../themes/monokai-remastered.toml"),
    ),
    (
        "monokai-soda",
        "Monokai Soda",
        include_str!("../themes/monokai-soda.toml"),
    ),
    (
        "monokai-vivid",
        "Monokai Vivid",
        include_str!("../themes/monokai-vivid.toml"),
    ),
    ("monokai", "Monokai", include_str!("../themes/monokai.toml")),
    (
        "prismattyc-dark",
        "Prismattyc Dark",
        include_str!("../themes/prismattyc-dark.toml"),
    ),
    (
        "prismattyc-default",
        "Prismattyc Default",
        include_str!("../themes/prismattyc-default.toml"),
    ),
    (
        "prismattyc-light",
        "Prismattyc Light",
        include_str!("../themes/prismattyc-light.toml"),
    ),
    (
        "prismattyc",
        "Prismattyc (match system)",
        include_str!("../themes/prismattyc.toml"),
    ),
    (
        "rose-pine-moon",
        "Rosé Pine Moon",
        include_str!("../themes/rose-pine-moon.toml"),
    ),
    (
        "tokyo-night",
        "Tokyo Night",
        include_str!("../themes/tokyo-night.toml"),
    ),
];

/// Number of bundled Prismattyc themes shipped in this crate.
pub const BUNDLED_COUNT: usize = ENTRIES.len();

/// Catalog entries (ids and names).
#[must_use]
pub fn bundled_catalog() -> Vec<BundledTheme> {
    ENTRIES
        .iter()
        .map(|(id, name, _)| BundledTheme { id, name })
        .collect()
}

/// Looks up catalog metadata by id.
#[must_use]
pub fn bundled_meta(id: &str) -> Option<BundledTheme> {
    ENTRIES
        .iter()
        .find(|(entry_id, _, _)| *entry_id == id)
        .map(|(id, name, _)| BundledTheme { id, name })
}

/// Loads a bundled theme by id.
///
/// # Errors
///
/// Returns [`ThemeLoadError`] when `id` is unknown or the embedded file is invalid.
pub fn load_bundled(id: &str) -> Result<Theme, ThemeLoadError> {
    let (_, _, raw) = ENTRIES
        .iter()
        .find(|(entry_id, _, _)| *entry_id == id)
        .ok_or_else(|| ThemeLoadError::new(format!("unknown bundled theme id: {id}")))?;
    theme_from_toml_strict(raw)
}
