// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_badge::Badge;
use lunatui_button::{Button, Toggle, Variant, WidgetState};
use lunatui_card::Card;
use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Size, Style};
use lunatui_field::Field;
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant as ThemeVariant};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{Block, Paragraph, RenderCx, TextInput, Widget};

const NAME: &str = "Ada Lovelace";
const EMAIL: &str = "ada@lovelace.dev";
const SLOT: u32 = 90;

fn typed(full: &str, tick: u32, from: usize, every: u32) -> &str {
    let count = from + (tick / every) as usize;
    &full[..count.min(full.len())]
}

fn focus_at(tick: u32) -> u32 {
    [1, 2, 0][((tick / SLOT) % 3) as usize]
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
    tick: u32,
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
        let focus = focus_at(self.tick);
        let fill = surface(cx, Role::Background);
        let name = NAME;
        let email = typed(EMAIL, self.tick, 12, 4);
        let email_done = email.len() == EMAIL.len();
        let name_input = Self::input(fill, name, focus == 0);
        let email_input = Self::input(fill, email, focus == 1);
        let workspace = Self::input(fill, "ws-7f3a91", false);
        let toggle = Toggle::new("Email me about deploys", self.tick >= SLOT + 40)
            .state(if focus == 2 {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            })
            .on(Role::Card);
        let toggle = Fixed(&toggle, 26);
        let mut email_field = Field::new("Email")
            .required()
            .focused(focus == 1)
            .on(Role::Card)
            .body(&email_input);
        email_field = if email_done {
            email_field.help("Receipts and alerts go here")
        } else if email.len() > 4 {
            email_field.error("Enter a full address, like ada@example.com")
        } else {
            email_field.help("Receipts and alerts go here")
        };
        let fields = [
            Field::new("Full name")
                .required()
                .focused(focus == 0)
                .on(Role::Card)
                .help("As it appears on invoices")
                .body(&name_input),
            email_field,
            Field::new("Notifications")
                .focused(focus == 2)
                .on(Role::Card)
                .help("Sent to the email above")
                .body(&toggle),
            Field::new("Workspace ID")
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
            field.render(Rect::new(area.x, y, area.width.min(60), h), cx);
            y += h + 1;
        }
        let save = Button::new("Save changes").on(Role::Card);
        let cancel = Button::new("Cancel")
            .variant(Variant::Outline)
            .on(Role::Card);
        let w = save.measure(Size::new(30, 1)).width;
        save.render(Rect::new(area.x, y, w, 1), cx);
        let w2 = cancel.measure(Size::new(30, 1)).width;
        cancel.render(Rect::new(area.x + w + 2, y, w2, 1), cx);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let backdrop = surface(&cx, Role::Background);
    Block::new().style(backdrop).render(area, &mut cx);
    let title = backdrop
        .fg(cx.token(Role::Primary.token()))
        .with_attrs(Attrs::new().set_bold(true));
    Paragraph::new("LUNATUI  /  Account settings")
        .style(title)
        .render(Rect::new(2, 0, 40, 1), &mut cx);
    Badge::new("Free plan")
        .tone(Tone::Neutral)
        .render(Rect::new(106, 0, 12, 1), &mut cx);
    let profile = Profile { tick };
    Card::new("Profile")
        .description("A scripted run: the fields fill in and focus moves down the form")
        .body(&profile)
        .render(Rect::new(1, 2, 118, 32), &mut cx);
    Paragraph::new("Required fields are marked with *. Errors replace the help line.")
        .style(backdrop)
        .render(Rect::new(3, 35, 114, 1), &mut cx);
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
