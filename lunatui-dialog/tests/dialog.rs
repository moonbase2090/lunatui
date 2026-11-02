// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_dialog::{AlertDialog, Dialog, DialogFocus, DialogKind, DialogOutcome, DialogState};
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::Role;
use lunatui_widgets::{RenderCx, Widget};

fn draw(widget: &dyn Widget) -> (Grid, ClusterTable, Theme) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(80, 20);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(0, 0, 80, 20), &mut cx);
    (grid, clusters, theme)
}

fn rows(grid: &Grid, clusters: &ClusterTable) -> String {
    grid.display_text(clusters)
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Result<Style, &'static str> {
    grid.get(x, y)
        .map(lunatui_core::Cell::style)
        .ok_or("missing cell")
}

fn find(grid: &Grid, clusters: &ClusterTable, needle: &str) -> Result<(u16, u16), &'static str> {
    for y in 0..20 {
        if let Some(x) = grid.row_display(y, clusters).find(needle) {
            let column = grid.row_display(y, clusters)[..x].chars().count();
            return Ok((u16::try_from(column).map_err(|_| "wide")?, y));
        }
    }
    Err("text not drawn")
}

#[test]
fn the_panel_has_a_rounded_border_title_and_message() {
    let (grid, clusters, _) = draw(&Dialog::alert("Saved", "Your changes are saved."));
    let text = rows(&grid, &clusters);
    assert!(text.contains('╭') && text.contains('╯'));
    assert!(text.contains("Saved"));
    assert!(text.contains("Your changes are saved."));
    assert!(text.contains("OK"));
    assert!(!text.contains("Cancel"));
}

#[test]
fn confirm_and_prompt_have_cancel_and_ok() {
    for dialog in [
        Dialog::confirm("Leave?", "Unsaved work."),
        Dialog::prompt("Name", "Pick one.", "", 0).placeholder("name"),
    ] {
        let (grid, clusters, _) = draw(&dialog);
        let text = rows(&grid, &clusters);
        assert!(text.contains("Cancel") && text.contains("OK"));
    }
}

#[test]
fn a_prompt_shows_its_text_or_placeholder() {
    let (grid, clusters, _) = draw(&Dialog::prompt("Name", "Pick one.", "edge", 4));
    assert!(rows(&grid, &clusters).contains("edge"));
    let (grid, clusters, _) = draw(&Dialog::prompt("Name", "Pick one.", "", 0).placeholder("name"));
    assert!(rows(&grid, &clusters).contains("name"));
}

#[test]
fn labels_replace_the_button_text() {
    let (grid, clusters, _) = draw(
        &Dialog::confirm("Leave?", "Unsaved work.")
            .confirm_label("Leave")
            .cancel_label("Stay"),
    );
    let text = rows(&grid, &clusters);
    assert!(text.contains("Stay") && text.contains("Leave"));
    assert!(!text.contains("Cancel"));
}

#[test]
fn an_alert_dialog_defaults_to_cancel_and_a_danger_action() -> Result<(), &'static str> {
    let widget = AlertDialog::new("Delete?", "This cannot be undone.");
    let (grid, clusters, theme) = draw(&widget);
    let text = rows(&grid, &clusters);
    assert!(text.contains("Cancel") && text.contains("Continue"));
    let (x, y) = find(&grid, &clusters, "Continue")?;
    assert_eq!(
        style_at(&grid, x, y)?.bg,
        theme.resolve(Role::Destructive.token())
    );
    Ok(())
}

#[test]
fn a_non_destructive_alert_dialog_uses_the_ring() -> Result<(), &'static str> {
    let widget = AlertDialog::new("Go live?", "Everyone will see it.")
        .action("Publish")
        .destructive(false);
    let (grid, clusters, theme) = draw(&widget);
    let (x, y) = find(&grid, &clusters, "Publish")?;
    assert_eq!(style_at(&grid, x, y)?.bg, theme.resolve(Role::Ring.token()));
    Ok(())
}

#[test]
fn state_moves_focus_between_the_buttons() -> Result<(), &'static str> {
    let mut state = DialogState::new();
    state.open(DialogKind::Confirm, None);
    let widget = AlertDialog::new("Delete?", "Sure?").state(&state);
    let (grid, clusters, theme) = draw(&widget);
    let ring = theme.resolve(Role::Destructive.token());
    let (cx, cy) = find(&grid, &clusters, "Continue")?;
    assert_eq!(style_at(&grid, cx, cy)?.bg, ring);
    state.previous();
    let widget = AlertDialog::new("Delete?", "Sure?").state(&state);
    let (grid, clusters, theme) = draw(&widget);
    let (cx, cy) = find(&grid, &clusters, "Cancel")?;
    assert_eq!(
        style_at(&grid, cx, cy)?.bg,
        theme.resolve(Role::Ring.token())
    );
    Ok(())
}

#[test]
fn clicks_find_the_button_and_close_the_dialog() -> Result<(), &'static str> {
    let area = Rect::new(0, 0, 80, 20);
    let widget = AlertDialog::new("Delete?", "Sure?");
    let (grid, clusters, _) = draw(&widget);
    let (x, y) = find(&grid, &clusters, "Cancel")?;
    assert_eq!(widget.control_at(area, x, y), Some(DialogFocus::Cancel));
    assert_eq!(widget.control_at(area, 0, 0), None);
    let mut state = DialogState::new();
    state.open(DialogKind::Confirm, Some(7));
    let closed = state
        .click(widget.control_at(area, x, y))
        .ok_or("did not close")?;
    assert_eq!(closed.outcome(), DialogOutcome::Cancelled);
    assert_eq!(closed.restore(), Some(7));
    assert!(!state.is_open());
    Ok(())
}

#[test]
fn the_panel_stays_inside_a_small_area() {
    let area = Rect::new(0, 0, 30, 8);
    let panel = Dialog::confirm("Leave?", "A long message that has to wrap across rows.")
        .width(60)
        .area(area);
    assert!(panel.x + panel.width <= area.width);
    assert!(panel.y + panel.height <= area.height);
}
