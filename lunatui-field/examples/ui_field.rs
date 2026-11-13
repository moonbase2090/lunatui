// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_badge::Badge;
use lunatui_button::{Button, Variant, WidgetState};
use lunatui_card::Card;
use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Size, Style};
use lunatui_field::Field;
use lunatui_runtime::{AccessibilityRole, Clock, HitId, HitMap};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant as ThemeVariant};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{Block, NamedControl, Paragraph, RenderCx, TextInput, Toggle, Widget};

const NAME: &str = "Example user";
const EMAIL: &str = "user@example.com";

fn typed(full: &str, tick: u32, start: u32, every: u32) -> &str {
    let count = (tick.saturating_sub(start) / every) as usize;
    &full[..count.min(full.len())]
}

fn focus_at(tick: u32) -> u32 {
    match tick {
        0..75 => 0,
        75..188 => 1,
        188..264 => 2,
        _ => 4,
    }
}

struct Fixed<'a>(&'a dyn Widget, u16);

impl Widget for Fixed<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        self.0.render(
            Rect::new(area.x, area.y, area.width.min(self.1), area.height),
            cx,
        );
    }

    fn measure(&self, avail: Size) -> Size {
        let inner = self.0.measure(avail);
        Size::new(inner.width.min(self.1), inner.height)
    }
}

struct Profile {
    clock: Clock,
}

impl Profile {
    fn input(fill: Style, value: &str, focused: bool) -> TextInput {
        TextInput::new(value, value.len())
            .field_style(fill)
            .padding(1)
            .show_cursor(focused)
    }
}

impl Widget for Profile {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let tick = u32::try_from(self.clock.now().as_millis() / 33).unwrap_or(u32::MAX);
        let focus = focus_at(tick);
        let fill = surface(cx, Role::Background);
        let name = typed(NAME, tick, 12, 4);
        let email_chars = (tick.saturating_sub(90) / 4).min(5) + tick.saturating_sub(132) / 4;
        let email = &EMAIL[..(email_chars as usize).min(EMAIL.len())];
        let email_done = email.len() == EMAIL.len();
        let cursor = tick % 24 < 15;
        let name_input = Self::input(fill, name, focus == 0 && cursor);
        let email_input = Self::input(fill, email, focus == 1 && cursor);
        let workspace = Self::input(fill, "ws-7f3a91", false).style(
            Style::new()
                .fg(cx.token(Role::Muted.token()))
                .with_attrs(Attrs::new().set_dim(true)),
        );
        let toggle = Toggle::new((205..228).contains(&tick) || tick >= 248)
            .label("Email me about deploys")
            .state(if focus == 2 {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            });
        let toggle = Fixed(&toggle, 27);
        let mut email_field = Field::new("Email")
            .id(HitId(2))
            .required()
            .focused(focus == 1)
            .on(Role::Card)
            .body(&email_input);
        email_field = if email_done {
            email_field.help("Receipts and alerts go here")
        } else if !email.is_empty() {
            email_field.error("Enter a complete email address.")
        } else {
            email_field.help("Receipts and alerts go here")
        };
        let fields = [
            Field::new("Full name")
                .id(HitId(1))
                .required()
                .focused(focus == 0)
                .on(Role::Card)
                .help("As it appears on invoices")
                .body(&name_input),
            email_field,
            Field::new("Notifications")
                .id(HitId(3))
                .role(AccessibilityRole::Checkbox)
                .focused(focus == 2)
                .on(Role::Card)
                .help("Sent to the email above")
                .body(&toggle),
            Field::new("Workspace ID")
                .id(HitId(4))
                .disabled(true)
                .on(Role::Card)
                .help("Set by your admin")
                .body(&workspace),
        ];
        let mut y = area.y;
        for field in &fields {
            let h = field.measure(Size::new(area.width, area.height)).height;
            if y + h > area.y + area.height {
                break;
            }
            field.render(Rect::new(area.x, y, area.width, h), cx);
            y += h + 1;
        }
        let save = Button::new("Save changes")
            .on(Role::Card)
            .state(if focus == 4 {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            });
        let cancel = Button::new("Cancel")
            .variant(Variant::Outline)
            .on(Role::Card);
        let w = save.measure(Size::new(30, 1)).width;
        NamedControl::new(HitId(5), AccessibilityRole::Button, &save)
            .render(Rect::new(area.x, y, w, 1), cx);
        let w2 = cancel.measure(Size::new(30, 1)).width;
        NamedControl::new(HitId(6), AccessibilityRole::Button, &cancel)
            .render(Rect::new(area.x + w + 2, y, w2, 1), cx);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn draw(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) -> HitMap {
    let mut hits = HitMap::new();
    let mut cx = RenderCx::new(grid, clusters, theme).hit_map(&mut hits);
    let backdrop = surface(&cx, Role::Background);
    Block::new().style(backdrop).render(area, &mut cx);
    let title = backdrop
        .fg(cx.token(Role::Primary.token()))
        .with_attrs(Attrs::new().set_bold(true));
    Paragraph::new("LUNATUI  /  Account settings")
        .style(title)
        .render(Rect::new(2, 0, 40, 1), &mut cx);
    let saved = tick >= 282;
    Badge::new(if saved { "Changes saved" } else { "Free plan" })
        .tone(if saved { Tone::Success } else { Tone::Neutral })
        .render(Rect::new(area.width.saturating_sub(17), 0, 15, 1), &mut cx);
    let profile = Profile {
        clock: Clock::at(Duration::from_millis(u64::from(tick) * 33)),
    };
    let card_width = area.width.saturating_sub(4).min(92);
    let card_x = area.x + (area.width - card_width) / 2;
    Card::new("Profile")
        .description("Manage your profile and notification preferences.")
        .body(&profile)
        .render(Rect::new(card_x, 2, card_width, 32), &mut cx);
    Paragraph::new(if saved {
        "Your profile is up to date. Notifications are enabled."
    } else {
        "Your workspace ID is managed by your administrator."
    })
    .style(backdrop)
    .render(Rect::new(3, 35, 114, 1), &mut cx);
    hits
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
    if args
        .iter()
        .any(|a| a == "--snapshot" || a == "--accessibility")
    {
        let tick = flag("--tick", 0);
        let hits = draw(&mut next, &mut clusters, &theme, area, tick);
        if args.iter().any(|a| a == "--accessibility") {
            hits.write_accessibility_json(io::stdout().lock(), Some(HitId(focus_at(tick) + 1)))?;
        } else if args.iter().any(|a| a == "--ansi") {
            let mut encoder = Encoder::new();
            io::stdout().write_all(encoder.encode_diff(
                &Grid::new(width, height),
                &next,
                &clusters,
                ColorMode::TrueColor,
            ))?;
        } else {
            print!("{}", next.display_text(&clusters));
        }
        return Ok(());
    }
    let frames = flag("--frames", 300);
    let fps = flag("--fps", 30).max(1);
    let period = Duration::from_secs(1) / fps;
    let mut previous = Grid::new(width, height);
    let mut encoder = Encoder::new();
    let mut out = io::stdout().lock();
    out.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let started = Instant::now();
    for tick in 0..frames {
        let _ = draw(&mut next, &mut clusters, &theme, area, tick);
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
