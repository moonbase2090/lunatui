// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use std::hint::black_box;
use std::io;
use std::time::Duration;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion};
use lunatui_bench::scenes::ratatui::dashboard::{self, Frames, Model};
use lunatui_bench::scenes::ratatui::{ColorOutput, encode, log, set_color_output, table};
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Widget};
use ratatui::{Terminal, TerminalOptions, Viewport};

const SIZES: [(u16, u16); 2] = [(200, 60), (400, 120)];

fn configured() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(1))
        .sample_size(30)
}

fn dashboard_pipeline(c: &mut Criterion) -> io::Result<()> {
    for color in [ColorOutput::Enabled, ColorOutput::Disabled] {
        set_color_output(color);
        let color_label = match color {
            ColorOutput::Enabled => "color",
            ColorOutput::Disabled => "no_color",
        };
        for (width, height) in SIZES {
            let area = Rect::new(0, 0, width, height);
            let mut group = c.benchmark_group(format!("dashboard/{width}x{height}/{color_label}"));
            let model = Model::new();
            let mut scratch = Buffer::empty(area);
            group.bench_function("render", |b| {
                b.iter(|| {
                    scratch.reset();
                    dashboard::render(&model, area, &mut scratch);
                    black_box(&scratch);
                });
            });
            group.bench_function("reset", |b| {
                b.iter(|| {
                    scratch.reset();
                    black_box(&scratch);
                });
            });
            let frames = Frames::new(area);
            let identical = frames.scroll.clone();
            for (name, previous, next) in [
                ("diff_small", &frames.first, &frames.tick),
                ("diff_scroll", &frames.tick, &frames.scroll),
                ("diff_full", &frames.empty, &frames.scroll),
                ("diff_identical", &frames.scroll, &identical),
            ] {
                group.bench_function(name, |b| {
                    b.iter(|| black_box(previous.diff_iter(next).count()));
                });
            }
            let mut out = Vec::with_capacity(1 << 20);
            for (name, previous, next) in [
                ("encode_small", &frames.first, &frames.tick),
                ("encode_scroll", &frames.tick, &frames.scroll),
                ("encode_full", &frames.empty, &frames.scroll),
            ] {
                encode(previous, next, &mut out)?;
                group.bench_function(name, |b| {
                    b.iter(|| encode(previous, next, &mut out));
                });
            }
            draw_benches(&mut group, area)?;
            group.finish();
        }
    }
    set_color_output(ColorOutput::Enabled);
    Ok(())
}

fn draw_benches(group: &mut BenchmarkGroup<'_, WallTime>, area: Rect) -> io::Result<()> {
    let mut model = Model::new();
    let mut tick = 0;
    let Ok(mut test_terminal) = Terminal::new(TestBackend::new(area.width, area.height));
    let options = TerminalOptions {
        viewport: Viewport::Fixed(area),
    };
    let mut sink_terminal = Terminal::with_options(CrosstermBackend::new(io::sink()), options)?;
    let Ok(_) =
        test_terminal.draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()));
    sink_terminal.draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()))?;
    group.bench_function("draw_tick_test_backend", |b| {
        b.iter(|| {
            tick += 1;
            model.tick = tick;
            test_terminal
                .draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()))
                .map(|_| ())
        });
    });
    group.bench_function("draw_tick_crossterm_sink", |b| {
        b.iter(|| {
            tick += 1;
            model.tick = tick;
            sink_terminal
                .draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()))
                .map(|_| ())
        });
    });
    group.bench_function("draw_scroll_every_frame_crossterm_sink", |b| {
        b.iter(|| {
            tick += 1;
            model.tick = tick;
            model.scroll = usize::try_from(tick % 300).unwrap_or(0);
            sink_terminal
                .draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()))
                .map(|_| ())
        });
    });
    Ok(())
}

fn layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout");
    let six = Layout::horizontal([
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Percentage(30),
        Constraint::Min(5),
        Constraint::Fill(1),
        Constraint::Fill(2),
    ]);
    let area = Rect::new(0, 0, 200, 60);
    group.bench_function("split_6_hit", |b| {
        b.iter(|| black_box(six.split(black_box(area))));
    });
    let mut width = 0_u16;
    let mut next_area = move || {
        width = width.wrapping_add(1);
        Rect::new(0, 0, 100 + width % 30_000, 60)
    };
    group.bench_function("split_6_miss", |b| {
        b.iter(|| black_box(six.split(next_area())));
    });
    let twenty = Layout::horizontal((0..20).map(|i| {
        if i % 2 == 0 {
            Constraint::Length(5)
        } else {
            Constraint::Fill(1)
        }
    }));
    group.bench_function("split_20_mixed_miss", |b| {
        b.iter(|| black_box(twenty.split(next_area())));
    });
    for count in [2_usize, 4, 8, 12, 16, 20] {
        let fill = Layout::horizontal(vec![Constraint::Fill(1); count]);
        let length = Layout::horizontal(vec![Constraint::Length(5); count]);
        group.bench_function(format!("split_{count}_fill_miss"), |b| {
            b.iter(|| black_box(fill.split(next_area())));
        });
        group.bench_function(format!("split_{count}_length_miss"), |b| {
            b.iter(|| black_box(length.split(next_area())));
        });
    }
    group.finish();
}

fn diff_internals(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_internals_200x60");
    let area = Rect::new(0, 0, 200, 60);
    let ascii = Buffer::filled(area, Cell::new("a"));
    let ascii_copy = ascii.clone();
    group.bench_function("identical_ascii", |b| {
        b.iter(|| black_box(ascii.diff_iter(&ascii_copy).count()));
    });
    let boxes = Buffer::filled(area, Cell::new("─"));
    let boxes_copy = boxes.clone();
    group.bench_function("identical_box_drawing", |b| {
        b.iter(|| black_box(boxes.diff_iter(&boxes_copy).count()));
    });
    group.bench_function("slice_eq", |b| {
        b.iter(|| black_box(boxes.content == boxes_copy.content));
    });
    group.finish();
}

fn text(c: &mut Criterion) {
    let mut group = c.benchmark_group("text");
    let ascii = "The quick brown fox jumps over the lazy dog 0123456789 ".repeat(4);
    let cjk = "日本語のテキストを表示する速度を測定します。".repeat(4);
    let emoji = "🦀✨🚀👩‍💻🔥".repeat(10);
    let mut buf = Buffer::empty(Rect::new(0, 0, 400, 1));
    for (name, sample) in [("ascii", &ascii), ("cjk", &cjk), ("emoji_zwj", &emoji)] {
        group.bench_function(format!("set_string_{name}"), |b| {
            b.iter(|| buf.set_string(0, 0, black_box(sample), Style::new()));
        });
    }
    let line = Line::from(vec![
        "status ".into(),
        "ok".green(),
        " • ".dim(),
        "3 failing".red().bold(),
        " • 128 checks".into(),
    ]);
    group.bench_function("line_width_5_spans", |b| {
        b.iter(|| black_box(black_box(&line).width()));
    });
    group.finish();
}

fn breakdown(c: &mut Criterion) {
    let mut group = c.benchmark_group("breakdown_400x120");
    let area = Rect::new(0, 0, 400, 120);
    let mut buf = Buffer::empty(area);
    group.bench_function("set_style_bg_fill", |b| {
        b.iter(|| buf.set_style(area, Style::new().bg(Color::Rgb(1, 2, 3))));
    });
    group.bench_function("block_rounded_title", |b| {
        b.iter(|| {
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" t ")
                .render(area, &mut buf);
        });
    });
    let rows = table::rows(500);
    for built in [118, 500] {
        group.bench_function(format!("table_build_{built}_rows"), |b| {
            b.iter(|| table::render(&rows, built, area, &mut buf));
        });
    }
    let text = log::text(5000);
    for scroll in [0_u16, 1000, 4000] {
        group.bench_function(format!("log_wrap_scroll_{scroll}"), |b| {
            b.iter(|| log::render(&text, scroll, log::Layout::Wrapped, area, &mut buf));
        });
    }
    group.bench_function("log_no_wrap_scroll_4000", |b| {
        b.iter(|| log::render(&text, 4000, log::Layout::Unwrapped, area, &mut buf));
    });
    let visible = log::visible_slice(&text, 4000, 120);
    group.bench_function("log_wrap_preslice_120", |b| {
        b.iter(|| log::render(&visible, 0, log::Layout::Wrapped, area, &mut buf));
    });
    group.finish();
}

fn main() -> io::Result<()> {
    let mut criterion = configured().configure_from_args();
    dashboard_pipeline(&mut criterion)?;
    layout(&mut criterion);
    diff_internals(&mut criterion);
    text(&mut criterion);
    breakdown(&mut criterion);
    criterion.final_summary();
    Ok(())
}
