// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use lunatui_core::Rect;

use super::{HitId, HitMap, HitRegion};

/// Semantic role of a hit region in the experimental version-1 export.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    /// A control that activates an action.
    Button,
    /// One tab in a tab strip.
    Tab,
    /// One selectable list item.
    ListItem,
    /// An editable text control.
    TextInput,
    /// A control that toggles a checked state.
    Checkbox,
    /// One choice in a mutually exclusive group.
    RadioButton,
    /// A control that adjusts a value along a range.
    Slider,
    /// An indicator of progress through a range.
    ProgressBar,
}

impl AccessibilityRole {
    const fn wire_name(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Tab => "tab",
            Self::ListItem => "list_item",
            Self::TextInput => "text_input",
            Self::Checkbox => "checkbox",
            Self::RadioButton => "radio_button",
            Self::Slider => "slider",
            Self::ProgressBar => "progress_bar",
        }
    }
}

impl HitMap {
    /// Records a region and its role for experimental accessibility export.
    ///
    /// Pointer routing follows the same depth and tie rules as [`Self::push`].
    pub fn push_with_role(&mut self, id: HitId, area: Rect, z: u16, role: AccessibilityRole) {
        self.regions.push(HitRegion {
            id,
            area,
            z,
            role: Some(role),
        });
    }

    /// Writes a version-1 JSON snapshot of annotated regions, followed by a newline.
    ///
    /// Records retain registration order, cell coordinates, depth and repeated IDs.
    /// Every record matching `focused` is marked focused. Unannotated regions are
    /// omitted. Export does not infer visibility or hierarchy, and does not alter
    /// pointer routing or focus. The format contains no application text or values.
    ///
    /// # Errors
    ///
    /// Propagates writer errors. On error the destination may contain a partial snapshot.
    pub fn write_accessibility_json(
        &self,
        mut writer: impl Write,
        focused: Option<HitId>,
    ) -> io::Result<()> {
        writer.write_all(b"{\"version\":1,\"regions\":[")?;
        let mut separator = "";
        for region in &self.regions {
            let Some(role) = region.role else {
                continue;
            };
            write!(
                writer,
                "{separator}{{\"id\":{},\"role\":\"{}\",\"area\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\"z\":{},\"focused\":{}}}",
                region.id.0,
                role.wire_name(),
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
