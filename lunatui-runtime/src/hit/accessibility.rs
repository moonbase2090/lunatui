// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use lunatui_core::Rect;

use super::{AccessibilityRole, Annotation, HitId, HitMap, HitRegion};

impl HitMap {
    /// Records a legacy role-only annotation. For an interactive control, use
    /// [`Self::push_control`] with a required accessible name instead.
    /// Role-only annotations are not returned by [`Self::controls`].
    pub fn push_with_role(&mut self, id: HitId, area: Rect, z: u16, role: AccessibilityRole) {
        self.regions.push(HitRegion {
            id,
            area,
            z,
            annotation: Annotation::Role(role),
        });
    }

    /// Writes a version-2 accessibility snapshot, followed by a newline.
    ///
    /// Control names, roles, disabled state and layers accompany the rendered
    /// geometry. Input values, help and validation text are never included.
    /// Legacy role-only annotations have a null name and layer. Raw geometry
    /// is omitted. Records retain registration order and repeated IDs; focus
    /// is compared with the caller's current [`crate::FocusManager`] ID.
    ///
    /// # Errors
    ///
    /// Propagates writer errors. A failed write may leave a partial snapshot.
    pub fn write_accessibility_json(
        &self,
        mut writer: impl Write,
        focused: Option<HitId>,
    ) -> io::Result<()> {
        writer.write_all(b"{\"version\":2,\"regions\":[")?;
        let mut separator = "";
        for region in &self.regions {
            let (role, control) = match &region.annotation {
                Annotation::Geometry => continue,
                Annotation::Role(role) => (*role, None),
                Annotation::Control(metadata) => {
                    let control = metadata.record(region.id, region.area, region.z);
                    (control.role, Some(control))
                }
            };
            write!(
                writer,
                "{separator}{{\"id\":{},\"role\":\"{}\",\"name\":",
                region.id.0,
                role.wire_name()
            )?;
            if let Some(control) = control {
                write_string(&mut writer, control.name)?;
            } else {
                writer.write_all(b"null")?;
            }
            write!(
                writer,
                ",\"disabled\":{},\"layer\":",
                control.is_some_and(|c| c.disabled)
            )?;
            if let Some(layer) = control.and_then(|c| c.layer) {
                write!(writer, "{}", layer.0)?;
            } else {
                writer.write_all(b"null")?;
            }
            write!(
                writer,
                ",\"area\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\"z\":{},\"focused\":{}}}",
                region.area.x,
                region.area.y,
                region.area.width,
                region.area.height,
                region.z,
                focused == Some(region.id),
            )?;
            separator = ",";
        }
        writer.write_all(b"]}\n")
    }
}

fn write_string(writer: &mut impl Write, value: &str) -> io::Result<()> {
    writer.write_all(b"\"")?;
    for character in value.chars() {
        match character {
            '"' => writer.write_all(b"\\\"")?,
            '\\' => writer.write_all(b"\\\\")?,
            '\n' => writer.write_all(b"\\n")?,
            '\r' => writer.write_all(b"\\r")?,
            '\t' => writer.write_all(b"\\t")?,
            c if c <= '\u{1f}' => write!(writer, "\\u{:04x}", u32::from(c))?,
            c => {
                writer.write_all(c.encode_utf8(&mut [0; 4]).as_bytes())?;
            }
        }
    }
    writer.write_all(b"\"")
}
