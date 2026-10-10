// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fmt;

use lunatui_core::{
    Cell, CellWidth, ClusterTable, Color, DecodedGlyph, Grid, Style, UnderlineStyle,
};
use lunatui_term::{ColorMode, Encoder};

/// A vt100 screen fed by encoder bytes.
pub struct Oracle {
    parser: vt100::Parser,
}

impl Oracle {
    /// A blank screen of `rows` by `cols`.
    ///
    /// A zero dimension is treated as 1 so the parser is never built with an empty grid.
    #[must_use]
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: vt100::Parser::new(rows.max(1), cols.max(1), 0),
        }
    }

    /// Feeds terminal bytes into the screen.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    /// The screen's text, one row per line, as the parser stored it.
    #[must_use]
    pub fn contents(&self) -> String {
        self.parser.screen().contents()
    }

    /// Cells where the parsed screen disagrees with `grid`.
    ///
    /// `colors` compares foreground and background. Attributes the vt100 parser
    /// records (bold, dim, italic, single underline, reverse) are always compared.
    /// Continuation cells are checked only as wide-character tails.
    #[must_use]
    pub fn mismatches(&self, grid: &Grid, clusters: &ClusterTable, colors: bool) -> Vec<String> {
        let mut found = Vec::new();
        let screen = self.parser.screen();
        for row in 0..grid.height() {
            for col in 0..grid.width() {
                if found.len() >= 8 {
                    return found;
                }
                let Some(cell) = grid.get(col, row) else {
                    found.push(format!("row {row} col {col}: missing grid cell"));
                    continue;
                };
                let Some(screen_cell) = screen.cell(row, col) else {
                    found.push(format!("row {row} col {col}: missing screen cell"));
                    continue;
                };
                if cell.meta().width == CellWidth::Continuation {
                    if !screen_cell.is_wide_continuation() {
                        found.push(format!(
                            "row {row} col {col}: grid continuation is not a wide tail on screen"
                        ));
                    }
                    continue;
                }
                let grid_text = cell_text(cell, clusters);
                if !texts_match(screen_cell.contents(), &grid_text) {
                    found.push(format!(
                        "row {row} col {col}: text screen {:?} grid {grid_text:?}",
                        screen_cell.contents()
                    ));
                }
                if cell.meta().width == CellWidth::Two && !screen_cell.is_wide() {
                    found.push(format!(
                        "row {row} col {col}: grid wide lead is a single column on screen"
                    ));
                }
                compare_attrs(&mut found, row, col, screen_cell, cell.style());
                if colors {
                    compare_color(
                        &mut found,
                        row,
                        col,
                        "fg",
                        screen_cell.fgcolor(),
                        terminal_color(cell.style().fg),
                    );
                    compare_color(
                        &mut found,
                        row,
                        col,
                        "bg",
                        screen_cell.bgcolor(),
                        terminal_color(cell.style().bg),
                    );
                }
            }
        }
        found
    }
}

/// Why an encoded diff did not reproduce `next` on the vt100 screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleFailure {
    message: String,
}

impl OracleFailure {
    /// The mismatch text.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn with_seed(self, seed: u64) -> Self {
        Self {
            message: format!("seed {seed:#x}\n{}", self.message),
        }
    }

    pub(crate) fn at_exec(self, exec: u64) -> Self {
        Self {
            message: format!("exec {exec}\n{}", self.message),
        }
    }
}

impl fmt::Display for OracleFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

/// Paints `previous` from a blank screen, applies `encode(previous → next)`, and checks `next`.
///
/// # Errors
///
/// Returns a size mismatch, or up to eight cell mismatches.
pub fn diff_matches(
    previous: &Grid,
    next: &Grid,
    clusters: &ClusterTable,
    colors: ColorMode,
) -> Result<(), OracleFailure> {
    if previous.width() != next.width() || previous.height() != next.height() {
        return Err(size_failure(previous, next));
    }
    if next.width() == 0 || next.height() == 0 {
        return Err(size_failure(previous, next));
    }
    let mut oracle = Oracle::new(next.height(), next.width());
    let blank = Grid::new(next.width(), next.height());
    let mut encoder = Encoder::new();
    let paint = encoder
        .encode_diff(&blank, previous, clusters, colors)
        .to_vec();
    oracle.process(&paint);
    apply_diff(&mut oracle, previous, next, clusters, colors)
}

/// Applies `encode(previous → next)` onto a screen that already shows `previous`.
///
/// # Errors
///
/// Returns a size mismatch, or up to eight cell mismatches.
pub fn apply_diff(
    oracle: &mut Oracle,
    previous: &Grid,
    next: &Grid,
    clusters: &ClusterTable,
    colors: ColorMode,
) -> Result<(), OracleFailure> {
    if previous.width() != next.width()
        || previous.height() != next.height()
        || next.width() == 0
        || next.height() == 0
    {
        return Err(size_failure(previous, next));
    }
    let mut encoder = Encoder::new();
    let bytes = encoder
        .encode_diff(previous, next, clusters, colors)
        .to_vec();
    oracle.process(&bytes);
    let found = oracle.mismatches(next, clusters, colors == ColorMode::TrueColor);
    if found.is_empty() {
        Ok(())
    } else {
        Err(OracleFailure {
            message: found.join("\n"),
        })
    }
}

fn size_failure(previous: &Grid, next: &Grid) -> OracleFailure {
    OracleFailure {
        message: format!(
            "size previous {}x{} next {}x{}",
            previous.width(),
            previous.height(),
            next.width(),
            next.height()
        ),
    }
}

fn cell_text(cell: Cell, clusters: &ClusterTable) -> String {
    match cell.glyph().decode() {
        DecodedGlyph::Inline(scalar) => scalar.to_string(),
        DecodedGlyph::Cluster(id) => clusters
            .cluster_text(id)
            .map_or_else(|| "\u{FFFD}".to_owned(), str::to_owned),
        DecodedGlyph::Continuation => String::new(),
    }
}

fn texts_match(screen: &str, grid: &str) -> bool {
    screen == grid || (screen.is_empty() && grid == " ") || (grid.is_empty() && screen == " ")
}

fn terminal_color(color: Color) -> Color {
    match color {
        Color::Token(id) => u8::try_from(id % 256).map_or(Color::Indexed(0), Color::Indexed),
        other => other,
    }
}

fn screen_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Default,
        vt100::Color::Idx(index) => Color::Indexed(index),
        vt100::Color::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
    }
}

fn compare_color(
    found: &mut Vec<String>,
    row: u16,
    col: u16,
    role: &str,
    screen: vt100::Color,
    grid: Color,
) {
    let parsed = screen_color(screen);
    if parsed != grid && found.len() < 8 {
        found.push(format!(
            "row {row} col {col}: {role} screen {parsed:?} grid {grid:?}"
        ));
    }
}

fn compare_attrs(found: &mut Vec<String>, row: u16, col: u16, screen: &vt100::Cell, style: Style) {
    let attrs = style.attrs;
    flag(found, row, col, "bold", screen.bold(), attrs.bold());
    flag(found, row, col, "dim", screen.dim(), attrs.dim());
    flag(found, row, col, "italic", screen.italic(), attrs.italic());
    flag(
        found,
        row,
        col,
        "reverse",
        screen.inverse(),
        attrs.reverse(),
    );
    match attrs.underline() {
        UnderlineStyle::None => flag(found, row, col, "underline", screen.underline(), false),
        UnderlineStyle::Single => flag(found, row, col, "underline", screen.underline(), true),
        UnderlineStyle::Double
        | UnderlineStyle::Curly
        | UnderlineStyle::Dotted
        | UnderlineStyle::Dashed => {}
    }
}

fn flag(found: &mut Vec<String>, row: u16, col: u16, name: &str, screen: bool, grid: bool) {
    if screen != grid && found.len() < 8 {
        found.push(format!(
            "row {row} col {col}: {name} screen {screen} grid {grid}"
        ));
    }
}
