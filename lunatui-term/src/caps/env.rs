// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

pub(crate) fn var_string(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

pub(crate) fn var_is_set(key: &str) -> bool {
    std::env::var_os(key).is_some()
}

pub(crate) fn var_nonempty(key: &str) -> bool {
    std::env::var(key).is_ok_and(|value| !value.is_empty())
}

pub(crate) fn var_forces_color(key: &str) -> bool {
    std::env::var(key).is_ok_and(|value| {
        let value = value.to_ascii_lowercase();
        value == "1" || value == "true" || value == "yes"
    })
}

pub(crate) fn first_var(keys: &[&str]) -> String {
    for key in keys {
        if let Ok(value) = std::env::var(key)
            && !value.is_empty()
        {
            return value;
        }
    }
    String::new()
}
