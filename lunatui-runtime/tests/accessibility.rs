// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io::{self, Cursor};

use lunatui_core::Rect;
use lunatui_runtime::{AccessibilityRole as Role, FocusManager, HitId, HitMap};

#[test]
fn export_uses_registration_geometry_and_live_focus() -> io::Result<()> {
    let mut map = HitMap::new();
    let button = Rect::new(3, 4, 8, 2);
    let input = Rect::new(1, 2, 20, 1);
    map.push_with_role(HitId(7), button, 2, Role::Button);
    map.push(HitId(99), button, 9);
    map.push_with_role(HitId(8), input, 0, Role::TextInput);
    let mut focus = FocusManager::new();
    focus.register(HitId(7), button);
    focus.register(HitId(8), input);
    focus.focus(HitId(7));
    let mut output = Vec::new();
    map.write_accessibility_json(&mut output, focus.focused())?;
    assert_eq!(output, concat!(
        "{\"version\":1,\"regions\":[",
        "{\"id\":7,\"role\":\"button\",\"area\":{\"x\":3,\"y\":4,\"width\":8,\"height\":2},\"z\":2,\"focused\":true},",
        "{\"id\":8,\"role\":\"text_input\",\"area\":{\"x\":1,\"y\":2,\"width\":20,\"height\":1},\"z\":0,\"focused\":false}]}\n",
    ).as_bytes());
    focus.tab(false);
    output.clear();
    map.write_accessibility_json(&mut output, focus.focused())?;
    assert_eq!(output, concat!(
        "{\"version\":1,\"regions\":[",
        "{\"id\":7,\"role\":\"button\",\"area\":{\"x\":3,\"y\":4,\"width\":8,\"height\":2},\"z\":2,\"focused\":false},",
        "{\"id\":8,\"role\":\"text_input\",\"area\":{\"x\":1,\"y\":2,\"width\":20,\"height\":1},\"z\":0,\"focused\":true}]}\n",
    ).as_bytes());
    assert_eq!(map.pick(3, 4), Some(HitId(99)));
    Ok(())
}

#[test]
fn every_role_has_a_distinct_version_one_wire_name() -> io::Result<()> {
    for (role, expected) in [
        (Role::Button, "button"),
        (Role::Tab, "tab"),
        (Role::ListItem, "list_item"),
        (Role::TextInput, "text_input"),
        (Role::Checkbox, "checkbox"),
        (Role::RadioButton, "radio_button"),
        (Role::Slider, "slider"),
        (Role::ProgressBar, "progress_bar"),
    ] {
        let mut map = HitMap::new();
        map.push_with_role(HitId(1), Rect::new(0, 0, 1, 1), 0, role);
        let mut output = Vec::new();
        map.write_accessibility_json(&mut output, None)?;
        assert_eq!(output, format!(
            "{{\"version\":1,\"regions\":[{{\"id\":1,\"role\":\"{expected}\",\"area\":{{\"x\":0,\"y\":0,\"width\":1,\"height\":1}},\"z\":0,\"focused\":false}}]}}\n"
        ).as_bytes());
    }
    Ok(())
}

#[test]
fn clear_removes_metadata_and_unannotated_regions_export_nothing() -> io::Result<()> {
    let mut map = HitMap::new();
    map.push_with_role(HitId(1), Rect::new(0, 0, 2, 1), 0, Role::Tab);
    map.clear();
    map.push(HitId(2), Rect::new(0, 0, 2, 1), 0);
    let mut output = Vec::new();
    map.write_accessibility_json(&mut output, Some(HitId(1)))?;
    assert_eq!(output, b"{\"version\":1,\"regions\":[]}\n");
    assert_eq!(map.pick(0, 0), Some(HitId(2)));
    Ok(())
}

#[test]
fn repeated_ids_and_empty_rectangles_are_preserved() -> io::Result<()> {
    let mut map = HitMap::new();
    map.push_with_role(HitId(4), Rect::new(0, 0, 1, 1), 0, Role::ListItem);
    map.push_with_role(HitId(4), Rect::new(5, 6, 0, 0), 3, Role::ListItem);
    let mut output = Vec::new();
    map.write_accessibility_json(&mut output, Some(HitId(4)))?;
    assert_eq!(output, concat!(
        "{\"version\":1,\"regions\":[",
        "{\"id\":4,\"role\":\"list_item\",\"area\":{\"x\":0,\"y\":0,\"width\":1,\"height\":1},\"z\":0,\"focused\":true},",
        "{\"id\":4,\"role\":\"list_item\",\"area\":{\"x\":5,\"y\":6,\"width\":0,\"height\":0},\"z\":3,\"focused\":true}]}\n",
    ).as_bytes());
    assert_eq!(map.pick(5, 6), None);
    Ok(())
}

#[test]
fn unregistered_focus_does_not_mark_an_exported_region() -> io::Result<()> {
    let mut map = HitMap::new();
    map.push_with_role(HitId(1), Rect::new(0, 0, 1, 1), 0, Role::Checkbox);
    let mut output = Vec::new();
    map.write_accessibility_json(&mut output, Some(HitId(42)))?;
    assert_eq!(output, b"{\"version\":1,\"regions\":[{\"id\":1,\"role\":\"checkbox\",\"area\":{\"x\":0,\"y\":0,\"width\":1,\"height\":1},\"z\":0,\"focused\":false}]}\n");
    Ok(())
}

#[test]
fn writer_errors_reach_the_caller() {
    let map = HitMap::new();
    let mut storage = [0_u8; 4];
    let result = map.write_accessibility_json(Cursor::new(storage.as_mut_slice()), None);
    assert_eq!(
        result.err().map(|error| error.kind()),
        Some(io::ErrorKind::WriteZero)
    );
}
