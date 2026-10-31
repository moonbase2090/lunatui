// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Style};
use lunatui_theme::Token;
use lunatui_widgets::{
    Block, BorderType, Paragraph, ProgressBar, ProgressTone, RenderCx, StatusBar, Widget,
    WidgetState,
};

const STAGES: [(&str, u32, u32); 5] = [
    ("Checkout", 0, 40),
    ("Compile", 30, 110),
    ("Test suites", 100, 80),
    ("Package", 170, 30),
    ("Upload", 195, 45),
];

fn progress(tick: u32, start: u32, span: u32) -> f64 {
    if tick <= start {
        0.0
    } else {
        f64::from((tick - start).min(span)) / f64::from(span)
    }
}

fn bar(index: usize, tick: u32) -> ProgressBar<'static> {
    let (_, start, span) = STAGES[index];
    let done = progress(tick, start, span);
    match index {
        1 => ProgressBar::new()
            .segment(0.5_f64.min(done), ProgressTone::Accent)
            .segment((done - 0.5).max(0.0) * 0.9, ProgressTone::Ok)
            .segment(if done >= 1.0 { 0.04 } else { 0.0 }, ProgressTone::Fail),
        2 => ProgressBar::new()
            .segment(done * 0.78, ProgressTone::Ok)
            .segment(done * 0.15, ProgressTone::Warn)
            .segment(
                if done > 0.6 { (done - 0.6) * 0.1 } else { 0.0 },
                ProgressTone::Fail,
            ),
        4 if done > 0.0 && done < 1.0 => ProgressBar::new().indeterminate(tick),
        4 if done >= 1.0 => ProgressBar::new().segment(1.0, ProgressTone::Ok),
        _ => ProgressBar::new().segment(done, ProgressTone::Ok),
    }
}

pub(super) fn draw(area: Rect, tick: u32, pixels: bool, state: WidgetState, cx: &mut RenderCx<'_>) {
    let surface = cx.theme().resolve(Token::Surface0);
    let raised = cx.theme().resolve(Token::Surface1);
    let text = cx.theme().resolve(Token::TextPrimary);
    let accent = cx.theme().resolve(Token::Accent);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    let blank = vec![" ".repeat(usize::from(area.width)); usize::from(area.height)].join("\n");
    Paragraph::new(blank).style(fill).render(area, cx);
    Paragraph::new(" ".repeat(usize::from(area.width)))
        .style(head)
        .render(Rect::new(area.x, area.y, area.width, 1), cx);
    Paragraph::new("Release pipeline  ·  lunatui v0.1.15")
        .style(head)
        .render(
            Rect::new(area.x + 1, area.y, area.width.saturating_sub(1), 1),
            cx,
        );
    let left = Rect::new(area.x + 2, area.y + 2, 74, 24);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        24,
    );
    for (title, pane) in [(" Stages ", left), (" Log ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, cx);
    }
    stages(cx, left, tick, [fill, dim], pixels, state);
    log(cx, right, tick, [fill, dim]);
    let total = (0..STAGES.len())
        .map(|i| progress(tick, STAGES[i].1, STAGES[i].2))
        .sum::<f64>()
        / 5.0;
    ProgressBar::new()
        .ratio(total)
        .label("Overall ")
        .show_percent(true)
        .state(state)
        .pixels(pixels)
        .render(
            Rect::new(area.x + 2, left.y + 25, area.width.saturating_sub(4), 1),
            cx,
        );
    StatusBar::new(" r rerun stage  ●  l follow log  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            cx,
        );
}

fn stages(
    cx: &mut RenderCx<'_>,
    area: Rect,
    tick: u32,
    styles: [Style; 2],
    pixels: bool,
    state: WidgetState,
) {
    let [fill, dim] = styles;
    let notes = [
        "14 files",
        "412 of 760 crates cached",
        "1,087 tests",
        "2 artifacts",
        "uploading to the release",
    ];
    for (index, (name, start, span)) in STAGES.iter().enumerate() {
        let y = area.y + 2 + u16::try_from(index).unwrap_or(0) * 4;
        let done = progress(tick, *start, *span);
        let status = if done >= 1.0 && index != 4 {
            "done"
        } else if done > 0.0 {
            "running"
        } else {
            "waiting"
        };
        Paragraph::new(*name)
            .style(fill)
            .render(Rect::new(area.x + 3, y, 56, 1), cx);
        Paragraph::new(status)
            .style(if status == "running" { fill } else { dim })
            .render(Rect::new(area.x + 59, y, 7, 1), cx);
        bar(index, tick)
            .show_percent(index != 4)
            .state(state)
            .pixels(pixels)
            .render(Rect::new(area.x + 3, y + 1, area.width - 6, 1), cx);
        Paragraph::new(notes[index])
            .style(dim)
            .render(Rect::new(area.x + 3, y + 2, area.width - 6, 1), cx);
    }
}

fn log(cx: &mut RenderCx<'_>, area: Rect, tick: u32, styles: [Style; 2]) {
    let [fill, dim] = styles;
    let lines = [
        "checkout main @ 7577d26",
        "fetch 14 files",
        "cargo build --release",
        "compiling lunatui-widgets",
        "compiling lunatui-runtime",
        "cargo test --workspace",
        "test form ... ok",
        "test slider ... ok",
        "test toast ... ok",
        "1 warning: unused import",
        "package lunatui-0.1.15.crate",
        "upload release assets",
    ];
    let shown = usize::try_from(tick / 20).unwrap_or(0).min(lines.len());
    for (row, line) in (0_u16..).zip(&lines[..shown]) {
        let style = if row + 1 == u16::try_from(shown).unwrap_or(0) {
            fill
        } else {
            dim
        };
        Paragraph::new(format!("{:02}  {line}", row + 1))
            .style(style)
            .render(
                Rect::new(
                    area.x + 2,
                    area.y + 1 + row,
                    area.width.saturating_sub(4),
                    1,
                ),
                cx,
            );
    }
}
