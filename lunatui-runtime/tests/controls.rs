// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{AccessibilityRole as Role, ControlMetadata, HitId, HitMap, LayerId};

#[test]
fn selectors_report_ambiguity_and_scope_names_by_role_and_layer()
-> Result<(), Box<dyn std::error::Error>> {
    let mut hits = HitMap::new();
    hits.push_control(
        HitId(1),
        Rect::new(2, 3, 8, 1),
        7,
        ControlMetadata::new("Save", Role::Button),
    );
    hits.push_control(
        HitId(2),
        Rect::new(5, 6, 8, 1),
        200,
        ControlMetadata::new("Save", Role::Button)
            .layer(Some(LayerId(9)))
            .disabled(true),
    );
    let dialog = hits
        .find_control(Role::Button, "Save", Some(LayerId(9)))?
        .ok_or("expected registered control")?;
    assert_eq!(
        (dialog.id, dialog.area, dialog.z, dialog.disabled),
        (HitId(2), Rect::new(5, 6, 8, 1), 200, true)
    );
    assert_eq!(
        hits.find_control(Role::Button, "Save", None)?
            .ok_or("expected registered control")?
            .id,
        HitId(1)
    );
    assert_eq!(hits.find_control(Role::TextInput, "Save", None)?, None);
    hits.push_control(
        HitId(3),
        Rect::new(10, 1, 8, 1),
        0,
        ControlMetadata::new("Save", Role::Button),
    );
    assert_eq!(
        hits.find_control(Role::Button, "Save", None)
            .err()
            .ok_or("expected ambiguous control")?
            .matches,
        2
    );
    Ok(())
}

#[test]
fn export_escapes_control_names_and_preserves_depth_layer_and_disabled_state()
-> Result<(), Box<dyn std::error::Error>> {
    let mut hits = HitMap::new();
    hits.push_control(
        HitId(7),
        Rect::new(1, 2, 8, 1),
        200,
        ControlMetadata::new("Say \"Hi\"\\\n\t\u{7}界", Role::Button)
            .disabled(true)
            .layer(Some(LayerId(4))),
    );
    let mut output = Vec::new();
    hits.write_accessibility_json(&mut output, Some(HitId(7)))?;
    assert_eq!(
        String::from_utf8(output)?,
        concat!(
            "{\"version\":2,\"regions\":[{\"id\":7,\"role\":\"button\",",
            "\"name\":\"Say \\\"Hi\\\"\\\\\\n\\t\\u0007界\",\"disabled\":true,\"layer\":4,",
            "\"area\":{\"x\":1,\"y\":2,\"width\":8,\"height\":1},\"z\":200,\"focused\":true}]}\n"
        )
    );
    assert_eq!(hits.pick(1, 2), None);
    Ok(())
}

#[test]
fn a_new_frame_removes_control_names_and_disabled_shields_do_not_click_through()
-> Result<(), Box<dyn std::error::Error>> {
    let mut hits = HitMap::new();
    hits.push(HitId(1), Rect::new(0, 0, 10, 2), 0);
    hits.push_control(
        HitId(2),
        Rect::new(1, 0, 8, 1),
        1,
        ControlMetadata::new("Delete", Role::Button).disabled(true),
    );
    assert_eq!(hits.pick(2, 0), None);
    assert_eq!(hits.pick(0, 0), Some(HitId(1)));
    hits.clear();
    assert_eq!(hits.find_control(Role::Button, "Delete", None)?, None);
    assert_eq!(hits.controls().count(), 0);
    assert_eq!(hits.pick(0, 0), None);
    Ok(())
}

#[test]
#[should_panic(expected = "controls require a nonempty name")]
fn whitespace_is_not_a_control_name() {
    let _ = ControlMetadata::new(" \t\n", Role::TextInput);
}
