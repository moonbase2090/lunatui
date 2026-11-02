// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::canvas::{
    Arms, Canvas, Dir, EAST, Glyph, NORTH, Role, SOUTH, WEST, Weight, text_columns,
};
use super::flow_graph::count;
use super::seq_parse::{End, Item, Place};

pub(super) fn note_width(lines: &[String]) -> i32 {
    lines
        .iter()
        .map(|line| text_columns(line))
        .max()
        .unwrap_or(0)
        + 4
}

struct Frame {
    depth: i32,
    row: i32,
}

pub(super) struct Painter<'a> {
    canvas: &'a mut Canvas,
    centers: &'a [i32],
    width: i32,
    stack: Vec<Frame>,
}

fn rounded(mask: u8, weight: Weight) -> Arms {
    Arms {
        round: true,
        ..Arms::new(mask, weight)
    }
}

impl<'a> Painter<'a> {
    pub(super) const fn new(canvas: &'a mut Canvas, centers: &'a [i32], width: i32) -> Self {
        Self {
            canvas,
            centers,
            width,
            stack: Vec::new(),
        }
    }

    pub(super) fn item(&mut self, item: &Item, row: i32) {
        match item {
            Item::Message {
                from,
                to,
                dotted,
                end,
                text,
            } => {
                let weight = if *dotted {
                    Weight::Dotted
                } else {
                    Weight::Solid
                };
                if from == to {
                    self.own_message(self.centers[*from], row, weight, *end, text);
                } else {
                    let ends = (self.centers[*from], self.centers[*to]);
                    self.message(ends, row, weight, *end, text);
                }
            }
            Item::Note {
                place,
                first,
                second,
                lines,
            } => {
                let ends = (self.centers[*first], self.centers[*second]);
                self.note(*place, ends, row, lines);
            }
            Item::Open { kind, text } => self.open(kind.word(), text, row),
            Item::Else { text } => self.divider(text, row),
            Item::Close => self.close(row),
        }
    }

    fn head(&mut self, x: i32, row: i32, end: End, toward: Dir) {
        let glyph = match end {
            End::Cross => (Glyph::Deco('✕'), Role::Fail),
            End::Async => (Glyph::Deco('⟩'), Role::Edge),
            End::Head | End::Open => (Glyph::Arrow(toward), Role::Edge),
        };
        self.canvas.set(x, row, glyph.0, glyph.1);
    }

    fn message(&mut self, ends: (i32, i32), row: i32, weight: Weight, end: End, text: &str) {
        let (from, to) = ends;
        let east = from < to;
        let (low, high) = (from.min(to), from.max(to));
        let arrow_row = row + i32::from(!text.is_empty());
        if !text.is_empty() {
            let start = low + 1 + (high - low - 1 - text_columns(text)) / 2;
            self.canvas.text(start, row, text, Role::Message);
        }
        let toward = if east { Dir::East } else { Dir::West };
        for column in low + 1..high {
            let arms = Arms::new(EAST | WEST, weight);
            self.canvas.line(column, arrow_row, arms, Role::Edge);
        }
        let tee = Arms::new(NORTH | SOUTH | toward.arm(), Weight::Solid);
        self.canvas.line(from, arrow_row, tee, Role::Edge);
        if end == End::Open {
            let tee = Arms::new(NORTH | SOUTH | toward.opposite().arm(), Weight::Solid);
            self.canvas.line(to, arrow_row, tee, Role::Edge);
        } else {
            let target = if east { high - 1 } else { low + 1 };
            self.head(target, arrow_row, end, toward);
        }
    }

    fn own_message(&mut self, at: i32, row: i32, weight: Weight, end: End, text: &str) {
        let tee = Arms::new(NORTH | SOUTH | EAST, Weight::Solid);
        self.canvas.line(at, row, tee, Role::Edge);
        for column in at + 1..at + 3 {
            let arms = Arms::new(EAST | WEST, weight);
            self.canvas.line(column, row, arms, Role::Edge);
            self.canvas.line(column, row + 1, arms, Role::Edge);
        }
        let out = rounded(WEST | SOUTH, weight);
        let back = rounded(WEST | NORTH, weight);
        self.canvas.line(at + 3, row, out, Role::Edge);
        self.canvas.line(at + 3, row + 1, back, Role::Edge);
        if end == End::Open {
            self.canvas.line(at, row + 1, tee, Role::Edge);
        } else {
            self.head(at + 1, row + 1, end, Dir::West);
        }
        self.canvas.text(at + 5, row, text, Role::Message);
    }

    fn note(&mut self, place: Place, ends: (i32, i32), row: i32, lines: &[String]) {
        let (first, second) = ends;
        let text = note_width(lines) - 4;
        let (x, width) = match place {
            Place::Left => (first - 1 - (text + 4), text + 4),
            Place::Right => (first + 2, text + 4),
            Place::Over if first == second => {
                let width = text + 5 - (text + 4) % 2;
                (first - (width - 1) / 2, width)
            }
            Place::Over => (first - 1, (second - first + 3).max(text + 4)),
        };
        let height = count(lines.len().max(1)) + 2;
        for dy in 0..height {
            for dx in 0..width {
                self.canvas
                    .set(x + dx, row + dy, Glyph::Empty, Role::NoteText);
            }
        }
        let edge = |mask: u8| Glyph::Line {
            mask,
            weight: Weight::Solid,
            round: true,
            locked: true,
        };
        let (right, low) = (x + width - 1, row + height - 1);
        for dx in 1..width - 1 {
            self.canvas.set(x + dx, row, edge(EAST | WEST), Role::Note);
            self.canvas.set(x + dx, low, edge(EAST | WEST), Role::Note);
        }
        for dy in 1..height - 1 {
            self.canvas
                .set(x, row + dy, edge(NORTH | SOUTH), Role::Note);
            self.canvas
                .set(right, row + dy, edge(NORTH | SOUTH), Role::Note);
        }
        self.canvas.set(x, row, edge(EAST | SOUTH), Role::Note);
        self.canvas.set(right, row, edge(WEST | SOUTH), Role::Note);
        self.canvas.set(x, low, edge(EAST | NORTH), Role::Note);
        self.canvas.set(right, low, edge(WEST | NORTH), Role::Note);
        for (dy, line) in (1..).zip(lines) {
            let at = x + (width - text_columns(line)) / 2;
            self.canvas.text(at, row + dy, line, Role::NoteText);
        }
    }

    fn rule(&mut self, row: i32, depth: i32, weight: Weight) -> (i32, i32) {
        let (left, right) = (depth, self.width - 1 - depth);
        for column in left + 1..right {
            let arms = Arms::new(EAST | WEST, weight);
            self.canvas.line(column, row, arms, Role::Frame);
        }
        (left, right)
    }

    fn title(&mut self, at: (i32, i32), word: &str, text: &str) {
        let (mut x, row) = at;
        self.canvas.set(x, row, Glyph::Empty, Role::Frame);
        x += 1;
        x += self.canvas.text(x, row, word, Role::Tag);
        if !text.is_empty() {
            self.canvas.set(x, row, Glyph::Empty, Role::Frame);
            x += 1;
            x += self.canvas.text(x, row, &format!("[{text}]"), Role::Label);
        }
        self.canvas.set(x, row, Glyph::Empty, Role::Frame);
    }

    fn open(&mut self, word: &str, text: &str, row: i32) {
        let depth = count(self.stack.len());
        let (left, right) = self.rule(row, depth, Weight::Solid);
        let corner = rounded(EAST | SOUTH, Weight::Solid);
        self.canvas.line(left, row, corner, Role::Frame);
        let corner = rounded(WEST | SOUTH, Weight::Solid);
        self.canvas.line(right, row, corner, Role::Frame);
        self.title((left + 2, row), word, text);
        self.stack.push(Frame { depth, row });
    }

    fn divider(&mut self, text: &str, row: i32) {
        let depth = count(self.stack.len()) - 1;
        let (left, right) = self.rule(row, depth, Weight::Dotted);
        let end = Arms::new(EAST, Weight::Solid);
        self.canvas.line(left, row, end, Role::Frame);
        let end = Arms::new(WEST, Weight::Solid);
        self.canvas.line(right, row, end, Role::Frame);
        self.title((left + 2, row), "else", text);
    }

    fn close(&mut self, row: i32) {
        let Some(frame) = self.stack.pop() else {
            return;
        };
        let (left, right) = self.rule(row, frame.depth, Weight::Solid);
        let corner = rounded(EAST | NORTH, Weight::Solid);
        self.canvas.line(left, row, corner, Role::Frame);
        let corner = rounded(WEST | NORTH, Weight::Solid);
        self.canvas.line(right, row, corner, Role::Frame);
        for between in frame.row + 1..row {
            let arms = Arms::new(NORTH | SOUTH, Weight::Solid);
            self.canvas.line(left, between, arms, Role::Frame);
            self.canvas.line(right, between, arms, Role::Frame);
        }
    }
}
