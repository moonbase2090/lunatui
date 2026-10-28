// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{
    Attrs, Rect, Size, Style, UnderlineStyle, char_width, clip_display_prefix, text_display_width,
};
use lunatui_text::markdown::{Alignment, Block, Document, Inline, List, Table};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const BOLD: u16 = 1;
const ITALIC: u16 = 2;
const STRIKE: u16 = 4;
const CODE: u16 = 8;
const LINK: u16 = 16;
const DIM: u16 = 32;
const HEADING: u16 = 64;
const BAR: u16 = 128;
const MARKER: u16 = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Mark(u16);

impl Mark {
    const fn of(flag: u16) -> Self {
        Self(flag)
    }

    const fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    const fn with(self, flag: u16) -> Self {
        Self(self.0 | flag)
    }

    const fn without(self, flag: u16) -> Self {
        Self(self.0 & !flag)
    }
}
const BULLETS: [&str; 3] = ["•", "◦", "▪"];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Span {
    text: String,
    mark: Mark,
}

impl Span {
    fn new(text: impl Into<String>, mark: Mark) -> Self {
        Self {
            text: text.into(),
            mark,
        }
    }

    fn width(&self) -> usize {
        text_display_width(&self.text)
    }
}

#[derive(Clone, Debug, Default)]
struct Row {
    prefix: Vec<Span>,
    spans: Vec<Span>,
    code: bool,
}

#[derive(Clone, Debug)]
enum Atom {
    Word(String, Mark),
    Space(Mark),
    Break,
}

struct Frame {
    first: Vec<Span>,
    rest: Vec<Span>,
    used: bool,
}

fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(Span::width).sum()
}

fn push_span(spans: &mut Vec<Span>, span: Span) {
    if let Some(last) = spans.last_mut()
        && last.mark == span.mark
    {
        last.text.push_str(&span.text);
        return;
    }
    spans.push(span);
}

fn gather(inlines: &[Inline], mark: Mark, out: &mut Vec<Atom>) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => words(text, mark, out),
            Inline::Code(text) => words(text, mark.with(CODE), out),
            Inline::Emphasis(content) => gather(content, mark.with(ITALIC), out),
            Inline::Strong(content) => gather(content, mark.with(BOLD), out),
            Inline::Strikethrough(content) => gather(content, mark.with(STRIKE), out),
            Inline::Link { content, .. } => gather(content, mark.with(LINK), out),
            Inline::Image { alt, .. } => {
                out.push(Atom::Word(String::from("["), mark.with(DIM)));
                gather(alt, mark.with(DIM), out);
                out.push(Atom::Word(String::from("]"), mark.with(DIM)));
            }
            Inline::SoftBreak => out.push(Atom::Space(mark)),
            Inline::HardBreak => out.push(Atom::Break),
            Inline::Html(raw) => words(raw, mark.with(DIM), out),
        }
    }
}

fn words(text: &str, mark: Mark, out: &mut Vec<Atom>) {
    let mut word = String::new();
    let mut spaced = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !word.is_empty() {
                out.push(Atom::Word(core::mem::take(&mut word), mark));
            }
            spaced = true;
        } else {
            if spaced {
                out.push(Atom::Space(mark));
                spaced = false;
            }
            word.push(ch);
        }
    }
    if !word.is_empty() {
        out.push(Atom::Word(word, mark));
    } else if spaced {
        out.push(Atom::Space(mark));
    }
}

fn plain(inlines: &[Inline]) -> String {
    let mut atoms = Vec::new();
    gather(inlines, Mark::default(), &mut atoms);
    let mut text = String::new();
    for atom in &atoms {
        match atom {
            Atom::Word(word, _) => text.push_str(word),
            Atom::Space(_) | Atom::Break => text.push(' '),
        }
    }
    text
}

fn wrap(atoms: &[Atom], width: usize) -> Vec<Vec<Span>> {
    let width = width.max(1);
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut line: Vec<Span> = Vec::new();
    let mut used = 0;
    let mut pending: Option<Mark> = None;
    for atom in atoms {
        match atom {
            Atom::Break => {
                lines.push(core::mem::take(&mut line));
                used = 0;
                pending = None;
            }
            Atom::Space(mark) => {
                if used > 0 {
                    pending = Some(*mark);
                }
            }
            Atom::Word(word, mark) => {
                let size = text_display_width(word);
                let gap = usize::from(pending.is_some());
                if used + gap + size > width && used > 0 {
                    lines.push(core::mem::take(&mut line));
                    used = 0;
                    pending = None;
                }
                if let Some(space) = pending.take() {
                    push_span(&mut line, Span::new(" ", space.without(HEADING)));
                    used += 1;
                }
                if size <= width {
                    push_span(&mut line, Span::new(word.as_str(), *mark));
                    used += size;
                } else {
                    for ch in word.chars() {
                        let cells = char_width(ch);
                        if used > 0 && used + cells > width {
                            lines.push(core::mem::take(&mut line));
                            used = 0;
                        }
                        push_span(&mut line, Span::new(String::from(ch), *mark));
                        used += cells;
                    }
                }
            }
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn fit(text: &str, width: usize) -> String {
    if text_display_width(text) <= width {
        return String::from(text);
    }
    if width == 0 {
        return String::new();
    }
    let mut cut = String::from(clip_display_prefix(
        text,
        u16::try_from(width - 1).unwrap_or(0),
    ));
    cut.push('…');
    cut
}

struct Layout {
    width: usize,
    rows: Vec<Row>,
    frames: Vec<Frame>,
    depth: usize,
}

impl Layout {
    fn prefix_width(&self) -> usize {
        self.frames
            .iter()
            .map(|frame| spans_width(&frame.rest))
            .sum()
    }

    fn avail(&self) -> usize {
        self.width.saturating_sub(self.prefix_width()).max(1)
    }

    fn emit(&mut self, spans: Vec<Span>, code: bool) {
        let mut prefix = Vec::new();
        for frame in &mut self.frames {
            let source = if frame.used {
                &frame.rest
            } else {
                &frame.first
            };
            for span in source {
                push_span(&mut prefix, span.clone());
            }
            frame.used = true;
        }
        self.rows.push(Row {
            prefix,
            spans,
            code,
        });
    }

    fn blank(&mut self) {
        self.emit(Vec::new(), false);
    }

    fn rule(&mut self, glyph: &str, mark: Mark) {
        let text = glyph.repeat(self.avail());
        self.emit(vec![Span::new(text, mark)], false);
    }

    fn blocks(&mut self, blocks: &[Block], tight: bool) {
        for (index, block) in blocks.iter().enumerate() {
            if index > 0 && !tight {
                self.blank();
            }
            self.block(block);
        }
    }

    fn block(&mut self, block: &Block) {
        match block {
            Block::Paragraph(inlines) => self.text(inlines, Mark::default()),
            Block::Heading { level, content } => {
                self.text(content, Mark::of(HEADING));
                match level {
                    1 => self.rule("═", Mark::of(DIM)),
                    2 => self.rule("─", Mark::of(DIM)),
                    _ => {}
                }
            }
            Block::ThematicBreak => self.rule("─", Mark::of(DIM)),
            Block::BlockQuote(inner) => {
                let bar = vec![Span::new("▎ ", Mark::of(BAR))];
                self.frames.push(Frame {
                    first: bar.clone(),
                    rest: bar,
                    used: false,
                });
                self.blocks(inner, false);
                self.frames.pop();
            }
            Block::List(list) => self.list(list),
            Block::CodeBlock { info, code } => self.code(info, code),
            Block::Html(raw) => {
                let mut atoms = Vec::new();
                for (index, line) in raw.trim_end_matches('\n').split('\n').enumerate() {
                    if index > 0 {
                        atoms.push(Atom::Break);
                    }
                    words(line, Mark::of(DIM), &mut atoms);
                }
                for line in wrap(&atoms, self.avail()) {
                    self.emit(line, false);
                }
            }
            Block::Table(table) => self.table(table),
        }
    }

    fn text(&mut self, inlines: &[Inline], mark: Mark) {
        let mut atoms = Vec::new();
        gather(inlines, mark, &mut atoms);
        let lines = wrap(&atoms, self.avail());
        if lines.is_empty() {
            self.blank();
        }
        for line in lines {
            self.emit(line, false);
        }
    }

    fn list(&mut self, list: &List) {
        let markers: Vec<String> = list
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| match (item.checked, list.start) {
                (Some(true), _) => String::from("☑"),
                (Some(false), _) => String::from("☐"),
                (None, Some(start)) => {
                    format!(
                        "{}.",
                        start.saturating_add(u64::try_from(index).unwrap_or(0))
                    )
                }
                (None, None) => String::from(BULLETS[self.depth % BULLETS.len()]),
            })
            .collect();
        let widest = markers
            .iter()
            .map(|marker| text_display_width(marker))
            .max()
            .unwrap_or(1);
        for (index, (item, marker)) in list.items.iter().zip(&markers).enumerate() {
            if index > 0 && !list.tight {
                self.blank();
            }
            let pad = widest - text_display_width(marker);
            let mut first = String::new();
            for _ in 0..pad {
                first.push(' ');
            }
            first.push_str(marker);
            first.push(' ');
            let rest = " ".repeat(widest + 1);
            self.frames.push(Frame {
                first: vec![Span::new(first, Mark::of(MARKER))],
                rest: vec![Span::new(rest, Mark::default())],
                used: false,
            });
            self.depth += 1;
            self.blocks(&item.blocks, list.tight);
            self.depth -= 1;
            if item.blocks.is_empty() {
                self.blank();
            }
            self.frames.pop();
        }
    }

    fn code(&mut self, info: &str, code: &str) {
        self.emit(Vec::new(), true);
        let label = info.split_whitespace().next().unwrap_or("");
        if !label.is_empty() {
            self.emit(vec![Span::new(label, Mark::of(DIM))], true);
        }
        for line in code.trim_end_matches('\n').split('\n') {
            let line = line.replace('\t', "    ");
            self.emit(vec![Span::new(line, Mark::default())], true);
        }
        self.emit(Vec::new(), true);
    }

    fn table(&mut self, table: &Table) {
        let columns = table.alignments.len().max(1);
        let header: Vec<String> = table.header.iter().map(|cell| plain(cell)).collect();
        let body: Vec<Vec<String>> = table
            .rows
            .iter()
            .map(|row| row.iter().map(|cell| plain(cell)).collect())
            .collect();
        let mut widths = vec![1_usize; columns];
        for row in core::iter::once(&header).chain(&body) {
            for (column, cell) in row.iter().enumerate().take(columns) {
                widths[column] = widths[column].max(text_display_width(cell));
            }
        }
        let avail = self.avail();
        let chrome = columns * 3 + 1;
        while widths.iter().sum::<usize>() + chrome > avail {
            let Some(widest) = (0..columns).max_by_key(|&column| widths[column]) else {
                break;
            };
            if widths[widest] <= 3 {
                break;
            }
            widths[widest] -= 1;
        }
        let border = |left: &str, join: &str, right: &str| {
            let mut text = String::from(left);
            for (column, width) in widths.iter().enumerate() {
                text.push_str(&"─".repeat(width + 2));
                text.push_str(if column + 1 == columns { right } else { join });
            }
            text
        };
        self.emit(vec![Span::new(border("┌", "┬", "┐"), Mark::of(DIM))], false);
        let mut lines = vec![(&header, Mark::of(BOLD))];
        let separator_after = 0;
        for row in &body {
            lines.push((row, Mark::default()));
        }
        for (index, (cells, mark)) in lines.into_iter().enumerate() {
            let mut spans = vec![Span::new("│", Mark::of(DIM))];
            for (column, width) in widths.iter().enumerate() {
                let cell = fit(cells.get(column).map_or("", String::as_str), *width);
                let pad = width - text_display_width(&cell);
                let (left, right) = match table.alignments.get(column) {
                    Some(Alignment::Right) => (pad, 0),
                    Some(Alignment::Center) => (pad / 2, pad - pad / 2),
                    _ => (0, pad),
                };
                spans.push(Span::new(" ".repeat(1 + left), Mark::default()));
                spans.push(Span::new(cell, mark));
                spans.push(Span::new(" ".repeat(1 + right), Mark::default()));
                spans.push(Span::new("│", Mark::of(DIM)));
            }
            self.emit(spans, false);
            if index == separator_after {
                self.emit(vec![Span::new(border("├", "┼", "┤"), Mark::of(DIM))], false);
            }
        }
        self.emit(vec![Span::new(border("└", "┴", "┘"), Mark::of(DIM))], false);
    }
}

fn layout(document: &Document, width: usize) -> Vec<Row> {
    let mut layout = Layout {
        width: width.max(1),
        rows: Vec::new(),
        frames: Vec::new(),
        depth: 0,
    };
    layout.blocks(&document.blocks, false);
    while layout
        .rows
        .last()
        .is_some_and(|row| row.spans.is_empty() && !row.code)
    {
        layout.rows.pop();
    }
    layout.rows
}

/// Renders a parsed Markdown [`Document`]: headings, paragraphs with emphasis and links, lists
/// with task markers, block quotes, code blocks, tables, and rules.
///
/// Text wraps to the area's width and scrolls by whole rows, so an application can pair the
/// view with a [`Scrollbar`](crate::Scrollbar) using [`content_height`](Self::content_height).
/// Colors come from theme tokens: headings and the quote bar use the accent token, links use the
/// focus ring with an underline, code sits on the raised surface. Every cell in the area is
/// rewritten, so the view can be redrawn over an earlier frame; give it a
/// [`background`](Self::background) to keep a panel color.
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_text::markdown::{Options, parse};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{MarkdownView, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let document = parse("# Title\n\nSome *text*.", Options::gfm());
/// let mut grid = Grid::new(12, 4);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// MarkdownView::new(&document).render(Rect::new(0, 0, 12, 4), &mut cx);
/// assert_eq!(grid.row_display(0, &clusters), "Title       ");
/// assert_eq!(grid.row_display(3, &clusters), "Some text.  ");
/// ```
#[derive(Clone, Debug)]
pub struct MarkdownView<'a> {
    document: &'a Document,
    scroll: usize,
    background: Option<Style>,
    state: WidgetState,
}

impl<'a> MarkdownView<'a> {
    /// A view of `document`, scrolled to the top.
    #[must_use]
    pub const fn new(document: &'a Document) -> Self {
        Self {
            document,
            scroll: 0,
            background: None,
            state: WidgetState::Normal,
        }
    }

    /// First visible row of the laid-out document.
    #[must_use]
    pub const fn scroll(mut self, rows: usize) -> Self {
        self.scroll = rows;
        self
    }

    /// Style every cell of the area starts from, such as a panel background.
    #[must_use]
    pub const fn background(mut self, style: Style) -> Self {
        self.background = Some(style);
        self
    }

    /// Interaction state used for colors that are not set explicitly.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Number of rows the document takes at `width` columns, for sizing a scrollbar.
    #[must_use]
    pub fn content_height(&self, width: u16) -> usize {
        if width == 0 {
            return 0;
        }
        layout(self.document, usize::from(width)).len()
    }
}

struct Palette {
    base: Style,
    accent: lunatui_core::Color,
    link: lunatui_core::Color,
    raised: lunatui_core::Color,
    dim: bool,
}

impl Palette {
    fn style(&self, mark: Mark, code: bool) -> Style {
        let mut style = self.base;
        let mut attrs = Attrs::new();
        if mark.has(BOLD) || mark.has(HEADING) {
            attrs = attrs.set_bold(true);
        }
        if mark.has(ITALIC) {
            attrs = attrs.set_italic(true);
        }
        if mark.has(STRIKE) {
            attrs = attrs.set_strikethrough(true);
        }
        if mark.has(DIM) || self.dim {
            attrs = attrs.set_dim(true);
        }
        if mark.has(LINK) {
            attrs = attrs.set_underline(UnderlineStyle::Single);
            style = style.fg(self.link);
        }
        if mark.has(HEADING) || mark.has(BAR) || mark.has(MARKER) {
            style = style.fg(self.accent);
        }
        if mark.has(CODE) {
            style = style.bg(self.raised).fg(self.accent);
        } else if code {
            style = style.bg(self.raised);
        }
        style.with_attrs(attrs)
    }
}

impl MarkdownView<'_> {
    fn palette(&self, cx: &RenderCx<'_>) -> Palette {
        let mut base = self.background.unwrap_or_default();
        if base.fg == lunatui_core::Color::Default {
            base = base.fg(cx.token(Token::TextPrimary));
        }
        if self.state == WidgetState::Disabled {
            base = base.with_attrs(base.attrs.set_dim(true));
        }
        Palette {
            base,
            accent: cx.token(Token::Accent),
            link: cx.token(Token::FocusRing),
            raised: cx.token(Token::Surface1),
            dim: self.state == WidgetState::Disabled,
        }
    }
}

fn put_row(cx: &mut RenderCx<'_>, palette: &Palette, area: Rect, y: u16, row: &Row) {
    let blank = " ".repeat(usize::from(area.width));
    let fill = if row.code {
        palette.style(Mark::default(), true)
    } else {
        palette.base
    };
    cx.put_str(area.x, y, &blank, fill);
    let mut x = area.x;
    let right = area.x + area.width;
    for span in row.prefix.iter().chain(&row.spans) {
        if x >= right {
            break;
        }
        let room = right - x;
        let text = clip_display_prefix(&span.text, room);
        if text.is_empty() {
            continue;
        }
        cx.put_str(x, y, text, palette.style(span.mark, row.code));
        x = x.saturating_add(u16::try_from(text_display_width(text)).unwrap_or(room));
    }
}

impl Widget for MarkdownView<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let palette = self.palette(cx);
        let rows = layout(self.document, usize::from(area.width));
        for offset in 0..area.height {
            let y = area.y + offset;
            let index = self.scroll.saturating_add(usize::from(offset));
            match rows.get(index) {
                Some(row) => put_row(cx, &palette, area, y, row),
                None => put_row(cx, &palette, area, y, &Row::default()),
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let rows = self.content_height(avail.width);
        Size::new(
            avail.width,
            u16::try_from(rows).unwrap_or(u16::MAX).min(avail.height),
        )
    }
}
