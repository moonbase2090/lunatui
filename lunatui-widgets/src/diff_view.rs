// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

/// How a line differs between the two sides of a [`Diff`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffKind {
    /// Present on both sides.
    Context,
    /// Only on the new side.
    Added,
    /// Only on the old side.
    Removed,
}

/// One line of a [`DiffHunk`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    /// Whether the line is context, added, or removed.
    pub kind: DiffKind,
    /// The one-based line number on the old side; `None` for an added line.
    pub old: Option<u32>,
    /// The one-based line number on the new side; `None` for a removed line.
    pub new: Option<u32>,
    /// The text without its `+`, `-`, or space marker.
    pub text: String,
}

/// A run of changed lines with its surrounding context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffHunk {
    /// The `@@ -a,b +c,d @@ section` line.
    pub header: String,
    /// The lines in order.
    pub lines: Vec<DiffLine>,
}

/// A parsed unified diff of one file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diff {
    old_name: String,
    new_name: String,
    hunks: Vec<DiffHunk>,
}

impl Diff {
    /// Parses unified diff text (the output of `diff -u` or `git diff` for one file).
    ///
    /// The `---` and `+++` lines give the file names, each `@@` line starts a hunk, and lines
    /// starting with a space, `+`, or `-` are context, added, and removed lines. Anything else
    /// before the first hunk (`diff --git`, `index`) and `\ No newline` markers are skipped. An
    /// empty line inside a hunk counts as an empty context line.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut diff = Self::default();
        let mut counters = (0_u32, 0_u32);
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("@@ ") {
                counters = hunk_start(rest).unwrap_or((1, 1));
                diff.hunks.push(DiffHunk {
                    header: line.to_owned(),
                    lines: Vec::new(),
                });
            } else if diff.hunks.is_empty() {
                if let Some(name) = line.strip_prefix("--- ") {
                    name.split('\t')
                        .next()
                        .unwrap_or("")
                        .clone_into(&mut diff.old_name);
                } else if let Some(name) = line.strip_prefix("+++ ") {
                    name.split('\t')
                        .next()
                        .unwrap_or("")
                        .clone_into(&mut diff.new_name);
                }
            } else if let Some((hunk, entry)) =
                diff.hunks.last_mut().zip(body_line(line, &mut counters))
            {
                hunk.lines.push(entry);
            }
        }
        diff
    }

    /// The file name on the old side, from the `---` line.
    #[must_use]
    pub fn old_name(&self) -> &str {
        &self.old_name
    }

    /// The file name on the new side, from the `+++` line.
    #[must_use]
    pub fn new_name(&self) -> &str {
        &self.new_name
    }

    /// The hunks in order.
    #[must_use]
    pub fn hunks(&self) -> &[DiffHunk] {
        &self.hunks
    }

    /// How many lines were added and removed.
    #[must_use]
    pub fn stats(&self) -> (usize, usize) {
        let count = |kind| {
            self.hunks
                .iter()
                .flat_map(|hunk| &hunk.lines)
                .filter(|line| line.kind == kind)
                .count()
        };
        (count(DiffKind::Added), count(DiffKind::Removed))
    }

    fn widest_number(&self) -> u32 {
        self.hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .flat_map(|line| [line.old, line.new])
            .flatten()
            .max()
            .unwrap_or(0)
    }
}

fn hunk_start(rest: &str) -> Option<(u32, u32)> {
    let mut parts = rest.split_whitespace();
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    let first = |range: &str| range.split(',').next().and_then(|n| n.parse().ok());
    Some((first(old)?, first(new)?))
}

fn body_line(line: &str, (old, new): &mut (u32, u32)) -> Option<DiffLine> {
    let mut chars = line.chars();
    let marker = chars.next();
    let text = chars.as_str().to_owned();
    let (kind, old_number, new_number) = match marker {
        Some('+') => (DiffKind::Added, None, Some(*new)),
        Some('-') => (DiffKind::Removed, Some(*old), None),
        Some(' ') | None => (DiffKind::Context, Some(*old), Some(*new)),
        Some(_) => return None,
    };
    if old_number.is_some() {
        *old += 1;
    }
    if new_number.is_some() {
        *new += 1;
    }
    Some(DiffLine {
        kind,
        old: old_number,
        new: new_number,
        text,
    })
}

/// How a [`DiffView`] lays its lines out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffLayout {
    /// One column: removed lines, then added lines, with both line numbers.
    #[default]
    Unified,
    /// Old on the left and new on the right, with changed lines paired side by side.
    SideBySide,
}

enum Row<'a> {
    Header(&'a str),
    Line(&'a DiffLine, Option<&'a DiffLine>),
    Pair(Option<&'a DiffLine>, Option<&'a DiffLine>),
}

/// A diff drawn with hunk headers, line numbers, colored added and removed lines, and the changed
/// part of each paired line highlighted.
///
/// Added lines use the ok color and removed lines the fail color. In
/// [`DiffLayout::SideBySide`], a run of removed lines and the added lines that follow it are
/// paired row by row, and the shorter side is left blank. With
/// [`intra_line`](DiffView::intra_line) (on by default) the text between the common start and
/// common end of a paired removed and added line is drawn inverted. [`scroll`](DiffView::scroll)
/// is the first row shown; [`rows`](DiffView::rows) and [`max_scroll`](DiffView::max_scroll) give
/// an app the numbers for a scrollbar.
#[derive(Clone, Debug)]
pub struct DiffView<'a> {
    diff: &'a Diff,
    layout: DiffLayout,
    scroll: usize,
    numbers: bool,
    intra_line: bool,
    state: WidgetState,
}

impl<'a> DiffView<'a> {
    /// A unified view of `diff` from its first row.
    #[must_use]
    pub const fn new(diff: &'a Diff) -> Self {
        Self {
            diff,
            layout: DiffLayout::Unified,
            scroll: 0,
            numbers: true,
            intra_line: true,
            state: WidgetState::Normal,
        }
    }

    /// Unified or side by side.
    #[must_use]
    pub const fn layout(mut self, layout: DiffLayout) -> Self {
        self.layout = layout;
        self
    }

    /// The first row shown (stops at [`max_scroll`](DiffView::max_scroll)).
    #[must_use]
    pub const fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    /// Shows line numbers. On by default.
    #[must_use]
    pub const fn line_numbers(mut self, numbers: bool) -> Self {
        self.numbers = numbers;
        self
    }

    /// Highlights the changed part of paired lines. On by default.
    #[must_use]
    pub const fn intra_line(mut self, intra_line: bool) -> Self {
        self.intra_line = intra_line;
        self
    }

    /// Interaction state: disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    fn build_rows(&self) -> Vec<Row<'a>> {
        let mut rows = Vec::new();
        for hunk in &self.diff.hunks {
            rows.push(Row::Header(&hunk.header));
            let lines = &hunk.lines;
            let mut at = 0;
            while at < lines.len() {
                if lines[at].kind == DiffKind::Context {
                    let line = &lines[at];
                    rows.push(match self.layout {
                        DiffLayout::Unified => Row::Line(line, None),
                        DiffLayout::SideBySide => Row::Pair(Some(line), Some(line)),
                    });
                    at += 1;
                    continue;
                }
                let removed_end = run_end(lines, at, DiffKind::Removed);
                let added_end = run_end(lines, removed_end, DiffKind::Added);
                let removed = &lines[at..removed_end];
                let added = &lines[removed_end..added_end];
                self.push_run(&mut rows, removed, added);
                at = added_end;
            }
        }
        rows
    }

    fn push_run(&self, rows: &mut Vec<Row<'a>>, removed: &'a [DiffLine], added: &'a [DiffLine]) {
        match self.layout {
            DiffLayout::Unified => {
                for (index, line) in removed.iter().enumerate() {
                    rows.push(Row::Line(line, added.get(index)));
                }
                for (index, line) in added.iter().enumerate() {
                    rows.push(Row::Line(line, removed.get(index)));
                }
            }
            DiffLayout::SideBySide => {
                for index in 0..removed.len().max(added.len()) {
                    rows.push(Row::Pair(removed.get(index), added.get(index)));
                }
            }
        }
    }

    /// The number of rows the whole diff takes: one per hunk header plus one per line in the
    /// unified layout, or one per context line and per paired change in the side-by-side layout.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.build_rows().len()
    }

    /// The largest useful [`scroll`](DiffView::scroll) for a view `rows` tall.
    #[must_use]
    pub fn max_scroll(&self, rows: u16) -> usize {
        self.rows().saturating_sub(usize::from(rows))
    }

    fn number_width(&self) -> u16 {
        if self.numbers {
            u16::try_from(self.diff.widest_number().max(1).ilog10() + 1).unwrap_or(1)
        } else {
            0
        }
    }
}

fn run_end(lines: &[DiffLine], from: usize, kind: DiffKind) -> usize {
    from + lines[from..]
        .iter()
        .take_while(|line| line.kind == kind)
        .count()
}

fn common_ends(a: &str, b: &str) -> (usize, usize) {
    let start = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    let room = a.chars().count().min(b.chars().count()) - start;
    let end = a
        .chars()
        .rev()
        .zip(b.chars().rev())
        .take(room)
        .take_while(|(x, y)| x == y)
        .count();
    let bytes = |chars: &mut dyn Iterator<Item = char>| chars.map(char::len_utf8).sum();
    (
        bytes(&mut a.chars().take(start)),
        bytes(&mut a.chars().rev().take(end)),
    )
}

struct Palette {
    text: Style,
    dim: Style,
    header: Style,
    added: Style,
    removed: Style,
    added_hit: Style,
    removed_hit: Style,
}

impl Palette {
    fn new(cx: &RenderCx<'_>, state: WidgetState) -> Self {
        let disabled = matches!(state, WidgetState::Disabled);
        let surface = cx.token(Token::Surface0);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled));
        let hit = |token| {
            Style::new()
                .fg(surface)
                .bg(cx.token(token))
                .with_attrs(Attrs::new().set_bold(true).set_dim(disabled))
        };
        Self {
            text: base,
            dim: base.with_attrs(Attrs::new().set_dim(true)),
            header: Style::new()
                .fg(cx.token(Token::Accent))
                .bg(cx.token(Token::Surface1))
                .with_attrs(Attrs::new().set_dim(disabled)),
            added: base.fg(cx.token(Token::Ok)),
            removed: base.fg(cx.token(Token::Fail)),
            added_hit: hit(Token::Ok),
            removed_hit: hit(Token::Fail),
        }
    }

    fn line(&self, kind: DiffKind) -> (Style, Style) {
        match kind {
            DiffKind::Context => (self.text, self.text),
            DiffKind::Added => (self.added, self.added_hit),
            DiffKind::Removed => (self.removed, self.removed_hit),
        }
    }
}

fn put_clipped(
    cx: &mut RenderCx<'_>,
    at: &mut u16,
    left: &mut u16,
    y: u16,
    text: &str,
    style: Style,
) {
    let clipped = clip_display_prefix(text, *left);
    cx.put_str(*at, y, clipped, style);
    let used = u16::try_from(width(clipped)).unwrap_or(*left);
    *at = at.saturating_add(used);
    *left = left.saturating_sub(used);
}

impl DiffView<'_> {
    fn put_text(
        &self,
        cx: &mut RenderCx<'_>,
        palette: &Palette,
        (x, y, room): (u16, u16, u16),
        line: &DiffLine,
        other: Option<&DiffLine>,
    ) {
        let (plain, hit) = palette.line(line.kind);
        let (mut at, mut left) = (x, room);
        let paired = other.filter(|_| self.intra_line && line.kind != DiffKind::Context);
        let Some(other) = paired else {
            put_clipped(cx, &mut at, &mut left, y, &line.text, plain);
            return;
        };
        let (start, end) = common_ends(&line.text, &other.text);
        let middle_end = line.text.len() - end;
        put_clipped(cx, &mut at, &mut left, y, &line.text[..start], plain);
        put_clipped(
            cx,
            &mut at,
            &mut left,
            y,
            &line.text[start..middle_end],
            hit,
        );
        put_clipped(cx, &mut at, &mut left, y, &line.text[middle_end..], plain);
    }

    fn put_side(
        &self,
        cx: &mut RenderCx<'_>,
        palette: &Palette,
        (x, y, room): (u16, u16, u16),
        side: Option<(&DiffLine, Option<&DiffLine>, bool)>,
    ) {
        let digits = usize::from(self.number_width());
        let Some((line, other, old_side)) = side else {
            return;
        };
        let (plain, _) = palette.line(line.kind);
        let number = if old_side { line.old } else { line.new };
        let marker = match line.kind {
            DiffKind::Context => ' ',
            DiffKind::Added => '+',
            DiffKind::Removed => '-',
        };
        let shown = number.map_or_else(String::new, |n| n.to_string());
        let numbers = if digits > 0 {
            format!("{shown:>digits$} ")
        } else {
            String::new()
        };
        let gutter = format!("{numbers}{marker} ");
        let used = u16::try_from(width(&gutter)).unwrap_or(room).min(room);
        cx.put_str(x, y, clip_display_prefix(&gutter, room), palette.dim);
        cx.put_str(x + used.saturating_sub(2), y, &marker.to_string(), plain);
        self.put_text(cx, palette, (x + used, y, room - used), line, other);
    }
}

impl Widget for DiffView<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let palette = Palette::new(cx, self.state);
        let blank = " ".repeat(usize::from(area.width));
        for row in 0..area.height {
            cx.put_str(area.x, area.y + row, &blank, palette.text);
        }
        let rows = self.build_rows();
        let first = self
            .scroll
            .min(rows.len().saturating_sub(usize::from(area.height)));
        let half = area.width.saturating_sub(1) / 2;
        for (offset, row) in rows
            .iter()
            .skip(first)
            .take(usize::from(area.height))
            .enumerate()
        {
            let y = area.y + u16::try_from(offset).unwrap_or(0);
            match row {
                Row::Header(header) => {
                    cx.put_str(area.x, y, &blank, palette.header);
                    cx.put_str(
                        area.x,
                        y,
                        clip_display_prefix(header, area.width),
                        palette.header,
                    );
                }
                Row::Line(line, other) => {
                    let digits = usize::from(self.number_width());
                    let old = line.old.map_or_else(String::new, |n| n.to_string());
                    let new = line.new.map_or_else(String::new, |n| n.to_string());
                    let gutter = if digits > 0 {
                        format!("{old:>digits$} {new:>digits$} ")
                    } else {
                        String::new()
                    };
                    let marker = match line.kind {
                        DiffKind::Context => ' ',
                        DiffKind::Added => '+',
                        DiffKind::Removed => '-',
                    };
                    let (plain, _) = palette.line(line.kind);
                    cx.put_str(
                        area.x,
                        y,
                        clip_display_prefix(&gutter, area.width),
                        palette.dim,
                    );
                    let used = u16::try_from(width(&gutter)).unwrap_or(0).min(area.width);
                    cx.put_str(area.x + used, y, &marker.to_string(), plain);
                    let at = (used + 2).min(area.width);
                    self.put_text(
                        cx,
                        &palette,
                        (area.x + at, y, area.width - at),
                        line,
                        *other,
                    );
                }
                Row::Pair(old, new) => {
                    let right = area.x + half + 1;
                    cx.put_str(area.x + half, y, "│", palette.dim);
                    self.put_side(
                        cx,
                        &palette,
                        (area.x, y, half),
                        old.map(|l| (l, *new, true)),
                    );
                    self.put_side(
                        cx,
                        &palette,
                        (right, y, area.width.saturating_sub(half + 1)),
                        new.map(|l| (l, *old, false)),
                    );
                }
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let rows = u16::try_from(self.rows()).unwrap_or(u16::MAX);
        Size::new(avail.width, rows.min(avail.height))
    }
}
