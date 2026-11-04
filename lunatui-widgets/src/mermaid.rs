// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod canvas;
mod flow_draw;
mod flow_graph;
mod flow_layout;
mod flow_parse;
mod glyphs;
mod label;
mod seq_layout;
mod seq_paint;
mod seq_parse;

use core::fmt;

use lunatui_core::{Rect, Size, Style};
use lunatui_theme::Token;

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Widget, WidgetState};
use canvas::{Canvas, Glyph, Role};

pub(super) const LIMIT: usize = 256;
const MAX_BYTES: usize = 64 * 1024;
const MAX_CELLS: i64 = 1_000_000;

/// Why a Mermaid source could not be parsed or drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MermaidError {
    line: usize,
    message: String,
}

impl MermaidError {
    pub(super) fn at(line: usize, message: &str) -> Self {
        Self {
            line,
            message: message.to_owned(),
        }
    }

    /// The 1-based line of the source the problem was found on.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// A short description of the problem.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for MermaidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl core::error::Error for MermaidError {}

/// How a [`MermaidView`] draws lines and arrowheads.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MermaidGlyphs {
    /// Box-drawing lines, rounded corners and `▶◀▲▼` arrowheads.
    #[default]
    Unicode,
    /// `-`, `|` and `+` lines with `> < ^ v` arrowheads, for terminals without box drawing.
    Ascii,
}

/// A parsed Mermaid flowchart or sequence diagram, already laid out.
///
/// Parsing does all the layout work once, so drawing the same diagram again is a copy of cells.
/// Supported: `flowchart` and `graph` (directions `TD`, `TB`, `BT`, `LR`, `RL`; the shapes
/// `[ ]`, `( )`, `([ ])`, `{ }` and `(( ))`; the links `-->`, `---`, `-.->`, `==>`, with `|text|`
/// or `-- text -->` labels, and chains) and `sequenceDiagram` (participants, actors, aliases,
/// the eight message arrows, notes, `autonumber`, and `loop`, `alt`/`else` and `opt` frames).
/// `style`, `classDef`, `class`, `linkStyle` and `click` lines are ignored and `%%` starts a
/// comment. Everything else, such as `subgraph`, is an error.
///
/// Input is untrusted: it may be at most 64 KiB, and a diagram holds at most 256 nodes, links,
/// participants and messages, with frames nested at most 8 deep.
///
/// # Example
///
/// ```
/// use lunatui_widgets::MermaidDiagram;
///
/// let diagram = MermaidDiagram::parse("flowchart LR\n  A --> B")?;
/// assert_eq!(diagram.size().height, 3);
/// assert_eq!(MermaidDiagram::parse("flowchart LR\n  subgraph x").unwrap_err().line(), 2);
/// # Ok::<(), lunatui_widgets::MermaidError>(())
/// ```
#[derive(Clone, Debug)]
pub struct MermaidDiagram {
    canvas: Canvas,
}

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut previous = None;
    for (at, ch) in line.char_indices() {
        if ch == '"' {
            quoted = !quoted;
        }
        if ch == '%' && previous == Some('%') && !quoted {
            return line.get(..at - 1).unwrap_or_default();
        }
        previous = Some(ch);
    }
    line
}

impl MermaidDiagram {
    /// Parses and lays out `source`.
    ///
    /// # Errors
    ///
    /// Returns a [`MermaidError`] with the 1-based line when the source is too large, uses
    /// syntax this widget does not draw, or lays out wider than a million cells.
    pub fn parse(source: &str) -> Result<Self, MermaidError> {
        if source.len() > MAX_BYTES {
            return Err(MermaidError::at(1, "the diagram is larger than 64 KiB"));
        }
        let lines: Vec<(usize, &str)> = source
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, strip_comment(line).trim()))
            .filter(|(_, line)| !line.is_empty())
            .collect();
        let Some((&(first, header), body)) = lines.split_first() else {
            return Err(MermaidError::at(1, "the diagram is empty"));
        };
        let (word, rest) = header
            .split_once(char::is_whitespace)
            .map_or((header, ""), |(word, rest)| (word, rest.trim()));
        let canvas = match word {
            "flowchart" | "graph" => Self::flow(first, rest, body)?,
            "sequenceDiagram" if rest.is_empty() => {
                let sequence = seq_parse::parse(body)?;
                seq_layout::layout(&sequence)?
            }
            _ => {
                return Err(MermaidError::at(
                    first,
                    "expected flowchart, graph or sequenceDiagram",
                ));
            }
        };
        Ok(Self { canvas })
    }

    fn flow(first: usize, rest: &str, body: &[(usize, &str)]) -> Result<Canvas, MermaidError> {
        let rest = rest.trim_end_matches(';').trim();
        let (word, tail) = rest
            .split_once(char::is_whitespace)
            .map_or((rest, ""), |(word, tail)| (word, tail.trim()));
        let direction = if word.is_empty() {
            flow_parse::Direction::TopDown
        } else {
            flow_parse::Direction::from_word(word)
                .ok_or_else(|| MermaidError::at(first, "the direction is TD, TB, BT, LR or RL"))?
        };
        let mut parser = flow_parse::Parser::new(direction);
        if !tail.is_empty() {
            return Err(MermaidError::at(first, "put statements on the lines below"));
        }
        for &(line, text) in body {
            parser.statement(line, text)?;
        }
        flow_layout::layout(&parser.finish())
    }

    /// The size the whole diagram takes, in columns and rows.
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(
            u16::try_from(self.canvas.width()).unwrap_or(u16::MAX),
            u16::try_from(self.canvas.height()).unwrap_or(u16::MAX),
        )
    }
}

pub(super) fn fits(width: i32, height: i32) -> bool {
    width <= i32::from(u16::MAX)
        && height <= i32::from(u16::MAX)
        && i64::from(width) * i64::from(height) <= MAX_CELLS
}

/// Draws a [`MermaidDiagram`] into an area, scrolled and clipped.
///
/// The diagram keeps its natural size ([`measure`](Widget::measure) reports it). When the area is
/// smaller, [`scroll`](Self::scroll) picks the top-left cell shown, and the part outside the area
/// is clipped. Colors come from theme tokens: node fills and edges from the surface and border
/// tokens, node borders and frame tags from `accent`, notes from `warn` and failed messages
/// from `fail`. Layout is precomputed; drawing copies the visible cells.
///
/// # Example
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{MermaidDiagram, MermaidView, RenderCx, Widget};
///
/// let diagram = MermaidDiagram::parse("flowchart TD\n  A[Start] --> B(Done)")?;
/// let size = diagram.size();
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(size.width, size.height);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// MermaidView::new(&diagram).render(Rect::new(0, 0, size.width, size.height), &mut cx);
/// assert_eq!(grid.row_display(1, &clusters), "│ Start │");
/// # Ok::<(), lunatui_widgets::MermaidError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MermaidView<'a> {
    diagram: &'a MermaidDiagram,
    scroll: (u16, u16),
    glyphs: MermaidGlyphs,
    style: Option<Style>,
    state: WidgetState,
}

impl<'a> MermaidView<'a> {
    /// A view of `diagram` from its top-left corner.
    #[must_use]
    pub const fn new(diagram: &'a MermaidDiagram) -> Self {
        Self {
            diagram,
            scroll: (0, 0),
            glyphs: MermaidGlyphs::Unicode,
            style: None,
            state: WidgetState::Normal,
        }
    }

    /// The column and row of the diagram drawn at the area's top-left cell, held inside the
    /// diagram so scrolling never shows empty space past its far edges.
    #[must_use]
    pub const fn scroll(mut self, x: u16, y: u16) -> Self {
        self.scroll = (x, y);
        self
    }

    /// Unicode or ASCII lines and arrowheads.
    #[must_use]
    pub const fn glyphs(mut self, glyphs: MermaidGlyphs) -> Self {
        self.glyphs = glyphs;
        self
    }

    /// The style every cell starts from, such as a panel background. Defaults to the primary text
    /// on `surface.0`.
    #[must_use]
    pub const fn background(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Interaction state: disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// The largest scroll column and row that still fill `area`.
    #[must_use]
    pub fn max_scroll(&self, area: Rect) -> (u16, u16) {
        let size = self.diagram.size();
        (
            size.width.saturating_sub(area.width),
            size.height.saturating_sub(area.height),
        )
    }
}

struct Styles {
    base: Style,
    roles: [Style; 13],
}

impl Styles {
    fn new(view: &MermaidView<'_>, cx: &RenderCx<'_>) -> Self {
        let ground = cx.token(Token::Surface0);
        let raised = cx.token(Token::Surface1);
        let text = cx.token(Token::TextPrimary);
        let mut base = view
            .style
            .unwrap_or_else(|| Style::new().fg(text).bg(ground));
        if view.state == WidgetState::Disabled {
            base = base.with_attrs(base.attrs.set_dim(true));
        }
        let on_raised = base.bg(raised);
        let tint = |style: Style, token| style.fg(cx.token(token));
        let bold = |style: Style| style.with_attrs(style.attrs.set_bold(true));
        let roles = [
            base,
            on_raised,
            tint(on_raised, Token::Accent),
            on_raised.fg(text),
            tint(base, Token::TextMuted),
            tint(base, Token::TextMuted),
            tint(base, Token::Border),
            base.fg(text),
            tint(on_raised, Token::Warn),
            on_raised.fg(text),
            tint(base, Token::Border),
            bold(tint(base, Token::Accent)),
            bold(tint(base, Token::Fail)),
        ];
        Self { base, roles }
    }

    fn of(&self, role: Role) -> Style {
        let at = match role {
            Role::None => 0,
            Role::Fill => 1,
            Role::Node => 2,
            Role::Text => 3,
            Role::Edge => 4,
            Role::Label => 5,
            Role::Life => 6,
            Role::Message => 7,
            Role::Note => 8,
            Role::NoteText => 9,
            Role::Frame => 10,
            Role::Tag => 11,
            Role::Fail => 12,
        };
        self.roles.get(at).copied().unwrap_or(self.base)
    }
}

impl Widget for MermaidView<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let styles = Styles::new(self, cx);
        cx.fill(area, styles.base, ' ');
        let (max_x, max_y) = self.max_scroll(area);
        let origin_x = i32::from(self.scroll.0.min(max_x));
        let origin_y = i32::from(self.scroll.1.min(max_y));
        let canvas = &self.diagram.canvas;
        for row in 0..area.height {
            for column in 0..area.width {
                let (x, y) = (origin_x + i32::from(column), origin_y + i32::from(row));
                let cell = canvas.get(x, y);
                if cell.role == Role::None {
                    continue;
                }
                let mut buffer = [0_u8; 4];
                let shown = match cell.glyph {
                    Glyph::Cluster(index) => canvas.cluster(index),
                    Glyph::Tail => {
                        if column == 0 {
                            " "
                        } else {
                            continue;
                        }
                    }
                    glyph => glyphs::text(glyph, self.glyphs, &mut buffer),
                };
                let shown = if lunatui_core::cluster_display_width(shown) > 1
                    && (column + 1 >= area.width || canvas.get(x + 1, y).glyph != Glyph::Tail)
                {
                    " "
                } else {
                    shown
                };
                cx.put_str(area.x + column, area.y + row, shown, styles.of(cell.role));
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        self.diagram.size().clamp(avail)
    }
}
