// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../lunatui-runtime/README.md")]

use std::io;

use lunatui_core::Rect;
use lunatui_runtime::{AccessibilityRole, FocusManager, HitId, HitMap};

fn main() -> io::Result<()> {
    let mut hits = HitMap::new();
    let mut focus = FocusManager::new();
    let button = Rect::new(1, 1, 8, 1);
    let input = Rect::new(1, 3, 20, 1);
    hits.push_with_role(HitId(1), button, 0, AccessibilityRole::Button);
    hits.push_with_role(HitId(2), input, 0, AccessibilityRole::TextInput);
    hits.push(HitId(99), Rect::new(0, 5, 24, 1), 0);
    focus.register(HitId(1), button);
    focus.register(HitId(2), input);
    focus.focus(HitId(1));
    let mut output = io::stdout().lock();
    hits.write_accessibility_json(&mut output, focus.focused())?;
    focus.tab(false);
    hits.write_accessibility_json(&mut output, focus.focused())
}
