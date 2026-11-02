// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::canvas::{
    Arms, Canvas, Dir, EAST, Glyph, NORTH, Role, SOUTH, WEST, Weight, text_columns,
};
use super::flow_parse::Shape;

pub(super) struct NodeBox {
    pub(super) lines: Vec<String>,
    pub(super) shape: Shape,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl NodeBox {
    pub(super) fn new(lines: Vec<String>, shape: Shape, horizontal: bool, roomy: bool) -> Self {
        let text = lines
            .iter()
            .map(|line| text_columns(line))
            .max()
            .unwrap_or(1);
        let rows = i32::try_from(lines.len()).unwrap_or(1).max(1);
        let margin = match shape {
            Shape::Rect | Shape::Round | Shape::Stadium => 4,
            Shape::Diamond | Shape::Circle => 6,
        };
        let mut width = text.max(1) + margin;
        let mut height = rows + 2;
        if horizontal && roomy {
            height = height.max(5);
        }
        if horizontal && height % 2 == 0 {
            height += 1;
        } else if !horizontal && width % 2 == 0 {
            width += 1;
        }
        Self {
            lines,
            shape,
            width,
            height,
        }
    }
}

pub(super) struct Mapper {
    pub(super) horizontal: bool,
    pub(super) flipped: bool,
    pub(super) total: i32,
}

impl Mapper {
    pub(super) fn point(&self, along: i32, across: i32) -> (i32, i32) {
        let along = if self.flipped {
            self.total - 1 - along
        } else {
            along
        };
        if self.horizontal {
            (along, across)
        } else {
            (across, along)
        }
    }

    pub(super) fn rect(
        &self,
        along: i32,
        length: i32,
        across: i32,
        breadth: i32,
    ) -> (i32, i32, i32, i32) {
        let first = if self.flipped {
            self.total - along - length
        } else {
            along
        };
        if self.horizontal {
            (first, across, length, breadth)
        } else {
            (across, first, breadth, length)
        }
    }
}

fn border(canvas: &mut Canvas, x: i32, y: i32, mask: u8, round: bool) {
    canvas.line(x, y, Arms::border(mask, round), Role::Node);
}

pub(super) fn draw_node(canvas: &mut Canvas, x: i32, y: i32, node: &NodeBox) {
    let (width, height) = (node.width, node.height);
    canvas.paint(x, y, width, height, Role::Fill);
    let round = node.shape != Shape::Rect;
    let last_x = x + width - 1;
    let last_y = y + height - 1;
    let rows = i32::try_from(node.lines.len()).unwrap_or(1).max(1);
    let middle = y + 1 + rows / 2;
    if node.shape == Shape::Diamond {
        for column in x + 2..=last_x - 2 {
            border(canvas, column, y, EAST | WEST, false);
            border(canvas, column, last_y, EAST | WEST, false);
        }
        canvas.set(x + 1, y, Glyph::Deco('╱'), Role::Node);
        canvas.set(last_x - 1, y, Glyph::Deco('╲'), Role::Node);
        canvas.set(x + 1, last_y, Glyph::Deco('╲'), Role::Node);
        canvas.set(last_x - 1, last_y, Glyph::Deco('╱'), Role::Node);
        canvas.set(x, middle, Glyph::Deco('<'), Role::Node);
        canvas.set(last_x, middle, Glyph::Deco('>'), Role::Node);
    } else {
        border(canvas, x, y, EAST | SOUTH, round);
        border(canvas, last_x, y, WEST | SOUTH, round);
        border(canvas, x, last_y, EAST | NORTH, round);
        border(canvas, last_x, last_y, WEST | NORTH, round);
        for column in x + 1..last_x {
            border(canvas, column, y, EAST | WEST, round);
            border(canvas, column, last_y, EAST | WEST, round);
        }
        for row in y + 1..last_y {
            border(canvas, x, row, NORTH | SOUTH, round);
            border(canvas, last_x, row, NORTH | SOUTH, round);
        }
        match node.shape {
            Shape::Stadium => {
                canvas.set(x, middle, Glyph::Deco('('), Role::Node);
                canvas.set(last_x, middle, Glyph::Deco(')'), Role::Node);
            }
            Shape::Circle => {
                canvas.set(x, middle, Glyph::Deco('('), Role::Node);
                canvas.set(x + 1, middle, Glyph::Deco('('), Role::Node);
                canvas.set(last_x - 1, middle, Glyph::Deco(')'), Role::Node);
                canvas.set(last_x, middle, Glyph::Deco(')'), Role::Node);
            }
            _ => {}
        }
    }
    for (row, line) in (y + 1..).zip(&node.lines) {
        let at = x + (width - text_columns(line)) / 2;
        canvas.text(at, row, line, Role::Text);
    }
}

fn step(from: (i32, i32), to: (i32, i32)) -> Dir {
    match ((to.0 - from.0).signum(), (to.1 - from.1).signum()) {
        (1, _) => Dir::East,
        (-1, _) => Dir::West,
        (_, 1) => Dir::South,
        _ => Dir::North,
    }
}

pub(super) fn expand(corners: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut cells: Vec<(i32, i32)> = Vec::new();
    for pair in corners.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let (dx, dy) = ((to.0 - from.0).signum(), (to.1 - from.1).signum());
        let mut at = from;
        loop {
            if cells.last() != Some(&at) {
                cells.push(at);
            }
            if at == to {
                break;
            }
            at = (at.0 + dx, at.1 + dy);
        }
    }
    cells
}

pub(super) fn draw_path(
    canvas: &mut Canvas,
    cells: &[(i32, i32)],
    weight: Weight,
    head_end: bool,
    head_start: bool,
) {
    let last = cells.len().saturating_sub(1);
    for (at, &(x, y)) in cells.iter().enumerate() {
        if (head_start && at == 0) || (head_end && at == last) {
            continue;
        }
        let mut mask = 0;
        if let Some(&before) = at.checked_sub(1).and_then(|prior| cells.get(prior)) {
            mask |= step((x, y), before).arm();
        }
        if let Some(&after) = cells.get(at + 1) {
            mask |= step((x, y), after).arm();
        }
        if head_start && at == 1 {
            canvas.set(x, y, Glyph::Arrow(step((x, y), cells[0])), Role::Edge);
        } else {
            let arms = Arms {
                round: true,
                ..Arms::new(mask, weight)
            };
            canvas.line(x, y, arms, Role::Edge);
        }
    }
    if head_end && last > 0 {
        let (x, y) = cells[last];
        canvas.set(
            x,
            y,
            Glyph::Arrow(step(cells[last - 1], cells[last])),
            Role::Edge,
        );
    }
}
