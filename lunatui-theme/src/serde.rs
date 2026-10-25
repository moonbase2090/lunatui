// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use lunatui_core::Color;
use serde::Deserialize;

use crate::theme::{Theme, Variant};
use crate::token::Token;

/// Error loading a theme file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeLoadError {
    message: String,
}

impl ThemeLoadError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ThemeLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ThemeLoadError {}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    variant: String,
    tokens: BTreeMap<String, String>,
}

/// Loads a theme from JSON.
///
/// # Errors
///
/// Returns [`ThemeLoadError`] when the JSON or token entries are invalid.
pub fn theme_from_json(source: &str) -> Result<Theme, ThemeLoadError> {
    let file: ThemeFile = serde_json::from_str(source)
        .map_err(|error| ThemeLoadError::new(format!("json: {error}")))?;
    build_theme(file, false)
}

/// Loads a theme from TOML.
///
/// # Errors
///
/// Returns [`ThemeLoadError`] when the TOML or token entries are invalid.
pub fn theme_from_toml(source: &str) -> Result<Theme, ThemeLoadError> {
    let file: ThemeFile =
        toml::from_str(source).map_err(|error| ThemeLoadError::new(format!("toml: {error}")))?;
    build_theme(file, false)
}

/// Loads a theme from TOML and requires the eight base file tokens to be present.
///
/// # Errors
///
/// Returns [`ThemeLoadError`] when the TOML is invalid or any token is missing.
pub fn theme_from_toml_strict(source: &str) -> Result<Theme, ThemeLoadError> {
    let file: ThemeFile =
        toml::from_str(source).map_err(|error| ThemeLoadError::new(format!("toml: {error}")))?;
    build_theme(file, true)
}

/// Loads a theme from JSON and requires the eight base file tokens to be present.
///
/// # Errors
///
/// Returns [`ThemeLoadError`] when the JSON is invalid or any token is missing.
pub fn theme_from_json_strict(source: &str) -> Result<Theme, ThemeLoadError> {
    let file: ThemeFile = serde_json::from_str(source)
        .map_err(|error| ThemeLoadError::new(format!("json: {error}")))?;
    build_theme(file, true)
}

fn build_theme(file: ThemeFile, require_all_tokens: bool) -> Result<Theme, ThemeLoadError> {
    let variant = parse_variant(&file.variant)?;
    if require_all_tokens {
        for token in Token::file_required() {
            let name = token.name();
            if !file.tokens.contains_key(name) {
                return Err(ThemeLoadError::new(format!("missing token: {name}")));
            }
        }
    }
    let mut theme = Theme::builtin(variant);
    for (name, hex) in file.tokens {
        let token = Token::from_name(&name)
            .ok_or_else(|| ThemeLoadError::new(format!("unknown token name: {name}")))?;
        let color = parse_hex_color(&hex)?;
        theme.set_token(token, color);
    }
    Ok(theme)
}

fn parse_variant(text: &str) -> Result<Variant, ThemeLoadError> {
    match text {
        "light" => Ok(Variant::Light),
        "dark" => Ok(Variant::Dark),
        other => Err(ThemeLoadError::new(format!("unknown variant: {other}"))),
    }
}

fn parse_hex_color(text: &str) -> Result<Color, ThemeLoadError> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    if hex.len() != 6 {
        return Err(ThemeLoadError::new(format!("expected #RRGGBB, got {text}")));
    }
    let red = parse_hex_byte(
        hex.get(0..2)
            .ok_or_else(|| ThemeLoadError::new("short hex"))?,
    )?;
    let green = parse_hex_byte(
        hex.get(2..4)
            .ok_or_else(|| ThemeLoadError::new("short hex"))?,
    )?;
    let blue = parse_hex_byte(
        hex.get(4..6)
            .ok_or_else(|| ThemeLoadError::new("short hex"))?,
    )?;
    Ok(Color::Rgb(red, green, blue))
}

fn parse_hex_byte(pair: &str) -> Result<u8, ThemeLoadError> {
    u8::from_str_radix(pair, 16)
        .map_err(|error| ThemeLoadError::new(format!("hex byte {pair}: {error}")))
}
