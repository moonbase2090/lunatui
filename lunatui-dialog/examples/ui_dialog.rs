// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_badge::Badge;
use lunatui_button::Button;
use lunatui_card::Card;
use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Size};
use lunatui_dialog::{AlertDialog, Dialog, DialogKind, DialogOutcome, DialogState};
use lunatui_runtime::{FocusManager, LayerStack};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant as ThemeVariant};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget, WidgetState};

const OLD_NAME: &str = "api-staging";
const NEW_NAME: &str = "api-edge";
const TYPE_FROM: u32 = 40;
const TYPE_EVERY: u32 = 4;
const TO_CANCEL: u32 = 84;
const TO_OK: u32 = 98;
const RENAME: u32 = 112;
const ASK: u32 = 130;
const HESITATE: u32 = 150;
const RESUME: u32 = 170;
const DELETE: u32 = 190;
const REPORT: u32 = 205;
const DISMISS: u32 = 245;
const OPEN_PROMPT: u32 = 30;

struct Model {
    state: DialogState,
    typed: usize,
    renamed: bool,
    deleted: bool,
}

impl Model {
    fn at(tick: u32) -> Self {
        let mut state = DialogState::new();
        let mut layers = LayerStack::new();
        let mut focus = FocusManager::new();
        let at = |mark: u32| tick >= mark;
        if at(OPEN_PROMPT) {
            state.open(DialogKind::Prompt, &mut layers, None);
        }
        let typed = if at(TYPE_FROM) {
            usize::try_from((tick - TYPE_FROM) / TYPE_EVERY + 1)
                .unwrap_or(usize::MAX)
                .min(NEW_NAME.len())
        } else {
            0
        };
        if at(TO_CANCEL) {
            state.next();
        }
        if at(TO_OK) {
            state.next();
        }
        let mut renamed = false;
        if at(RENAME) {
            renamed = state
                .activate(&mut layers, &mut focus)
                .is_some_and(|closed| closed.outcome() == DialogOutcome::Confirmed);
        }
        if at(ASK) {
            state.open(DialogKind::Confirm, &mut layers, None);
        }
        if at(HESITATE) {
            state.previous();
        }
        if at(RESUME) {
            state.next();
        }
        let mut deleted = false;
        if at(DELETE) {
            deleted = state
                .activate(&mut layers, &mut focus)
                .is_some_and(|closed| closed.outcome() == DialogOutcome::Confirmed);
        }
        if at(REPORT) {
            state.open(DialogKind::Alert, &mut layers, None);
        }
        if at(DISMISS) {
            let _ = state.activate(&mut layers, &mut focus);
        }
        Self {
            state,
            typed,
            renamed,
            deleted,
        }
    }

    fn name(&self) -> &'static str {
        if self.renamed { NEW_NAME } else { OLD_NAME }
    }

    fn typed_text(&self) -> &'static str {
        &NEW_NAME[..self.typed]
    }
}

struct Projects {
    name: &'static str,
    deleted: bool,
}

impl Widget for Projects {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let rows: [(&str, &str, &str, Tone); 7] = [
            ("api-gateway", "us-west-2", "healthy", Tone::Success),
            (self.name, "us-east-1", "degraded", Tone::Warning),
            ("billing-worker", "eu-central-1", "healthy", Tone::Success),
            ("docs-site", "us-west-2", "healthy", Tone::Success),
            ("auth-service", "us-east-1", "healthy", Tone::Success),
            ("metrics-collector", "eu-central-1", "paused", Tone::Neutral),
            ("web-app", "us-west-2", "healthy", Tone::Success),
        ];
        let line = surface(cx, Role::Card);
        let mut y = area.y;
        for (index, (name, region, status, tone)) in rows.iter().enumerate() {
            if index == 1 && self.deleted {
                continue;
            }
            Paragraph::new(*name)
                .style(line)
                .render(Rect::new(area.x, y, 24, 1), cx);
            Paragraph::new(*region)
                .style(line.fg(cx.token(Role::Muted.token())))
                .render(Rect::new(area.x + 26, y, 16, 1), cx);
            Badge::new(status)
                .tone(*tone)
                .render(Rect::new(area.x + 44, y, 10, 1), cx);
            y += 2;
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    *grid = Grid::new(area.width, area.height);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let backdrop = surface(&cx, Role::Background);
    Block::new().style(backdrop).render(area, &mut cx);
    let title = backdrop
        .fg(cx.token(Role::Primary.token()))
        .with_attrs(Attrs::new().set_bold(true));
    Paragraph::new("LUNATUI  /  Projects")
        .style(title)
        .render(Rect::new(2, 0, 50, 1), &mut cx);
    let model = Model::at(tick);
    let count = if model.deleted {
        "6 projects"
    } else {
        "7 projects"
    };
    Badge::new(count)
        .tone(Tone::Neutral)
        .render(Rect::new(104, 0, 14, 1), &mut cx);
    let projects = Projects {
        name: model.name(),
        deleted: model.deleted,
    };
    Card::new("Projects")
        .description("Rename a project, or delete one you no longer need")
        .body(&projects)
        .render(Rect::new(1, 2, 118, 24), &mut cx);
    Button::new("New project")
        .state(WidgetState::Normal)
        .on(Role::Background)
        .render(Rect::new(3, 28, 16, 1), &mut cx);
    Paragraph::new("Enter confirms, Tab moves between controls, Esc cancels.")
        .style(backdrop)
        .render(Rect::new(3, 35, 114, 1), &mut cx);
    let delete_prompt =
        format!("Delete {NEW_NAME}? Its deployments stop and its history is removed.");
    match model.state.kind() {
        Some(DialogKind::Prompt) => Dialog::prompt(
            "Rename project",
            format!("Choose a new name for {OLD_NAME}."),
            model.typed_text(),
            model.typed,
        )
        .placeholder("project-name")
        .confirm_label("Rename")
        .state(&model.state)
        .render(area, &mut cx),
        Some(DialogKind::Confirm) => AlertDialog::new("Delete project", delete_prompt.as_str())
            .action("Delete")
            .state(&model.state)
            .render(area, &mut cx),
        Some(DialogKind::Alert) => {
            Dialog::alert("Project deleted", format!("{NEW_NAME} was removed."))
                .state(&model.state)
                .render(area, &mut cx);
        }
        _ => {}
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let flag = |name: &str, default: u32| -> u32 {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let width = u16::try_from(flag("--width", 120)).unwrap_or(120);
    let height = u16::try_from(flag("--height", 36)).unwrap_or(36);
    let theme = Theme::builtin(ThemeVariant::Dark);
    let area = Rect::new(0, 0, width, height);
    let mut clusters = ClusterTable::new();
    let mut next = Grid::new(width, height);
    if args.iter().any(|a| a == "--snapshot") {
        draw(&mut next, &mut clusters, &theme, area, flag("--tick", 0));
        print!("{}", next.display_text(&clusters));
        return Ok(());
    }
    let frames = flag("--frames", 270);
    let fps = flag("--fps", 30).max(1);
    let period = Duration::from_secs(1) / fps;
    let mut previous = Grid::new(width, height);
    let mut encoder = Encoder::new();
    let mut out = io::stdout().lock();
    out.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let started = Instant::now();
    for tick in 0..frames {
        draw(&mut next, &mut clusters, &theme, area, tick);
        let mut frame = b"\x1b[?2026h".to_vec();
        frame.extend_from_slice(encoder.encode_diff(
            &previous,
            &next,
            &clusters,
            ColorMode::TrueColor,
        ));
        frame.extend_from_slice(b"\x1b[?2026l");
        out.write_all(&frame)?;
        out.flush()?;
        std::mem::swap(&mut previous, &mut next);
        let due = started + period * (tick + 1);
        thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    out.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(tick: u32) -> String {
        let theme = Theme::builtin(ThemeVariant::Dark);
        let mut grid = Grid::new(120, 36);
        let mut clusters = ClusterTable::new();
        draw(
            &mut grid,
            &mut clusters,
            &theme,
            Rect::new(0, 0, 120, 36),
            tick,
        );
        grid.display_text(&clusters)
    }

    #[test]
    fn the_script_walks_prompt_confirm_alert_and_back() {
        assert!(!frame(0).contains("Rename project"));
        assert!(frame(60).contains("Rename project"));
        assert!(frame(140).contains("Delete project"));
        assert!(frame(215).contains("Project deleted"));
        assert!(!frame(260).contains("Project deleted"));
    }

    #[test]
    fn rename_and_delete_change_the_list() {
        assert!(frame(100).contains(OLD_NAME));
        assert!(frame(120).contains("api-edge   "));
        let after = frame(260);
        assert!(!after.contains(NEW_NAME));
        assert!(after.contains("6 projects"));
        assert!(frame(0).contains("7 projects"));
    }

    #[test]
    fn typing_grows_the_field_one_character_at_a_time() {
        let typed = |tick| Model::at(tick).typed;
        assert_eq!(typed(TYPE_FROM - 1), 0);
        assert_eq!(typed(TYPE_FROM), 1);
        assert_eq!(typed(TYPE_FROM + TYPE_EVERY), 2);
        assert_eq!(typed(TO_CANCEL), NEW_NAME.len());
    }
}
