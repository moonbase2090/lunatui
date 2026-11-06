// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::{GraphNode, NodeConnection, NodeGraphError};
use lunatui_core::{Rect, Size, cluster_display_width};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug)]
pub(super) enum Glyph {
    Char(char),
    Cluster(usize),
    Tail,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Role {
    Ground,
    Edge,
    Border(usize),
    Title(usize),
    Body,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Cell {
    pub glyph: Glyph,
    pub role: Role,
}
#[derive(Debug)]
pub(super) struct Canvas {
    pub cells: Vec<Cell>,
    pub clusters: Vec<String>,
    width: usize,
    height: usize,
}

impl Canvas {
    pub(super) fn build(
        nodes: &[GraphNode<'_>],
        edges: &[NodeConnection],
        places: &[Rect],
        size: Size,
    ) -> Result<Self, NodeGraphError> {
        let (width, height) = (usize::from(size.width), usize::from(size.height));
        if width * height > 16_777_216 / edges.len().max(1) {
            return Err(NodeGraphError::TooLarge);
        }
        let mut canvas = Self {
            cells: vec![
                Cell {
                    glyph: Glyph::Char(' '),
                    role: Role::Ground
                };
                width * height
            ],
            clusters: Vec::new(),
            width,
            height,
        };
        for (index, &rect) in places.iter().enumerate() {
            canvas.node(index, rect);
        }
        let mut arms = vec![0_u8; width * height];
        for edge in edges {
            let a = places[edge.from];
            let b = places[edge.to];
            let start = (
                usize::from(a.x + a.width - 1),
                usize::from(a.y + 1 + edge.output),
            );
            let end = (usize::from(b.x), usize::from(b.y + 1 + edge.input));
            let path = canvas.route(start, end)?;
            for pair in path.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let (forward, back) = if b == a + 1 {
                    (2, 8)
                } else if a == b + 1 {
                    (8, 2)
                } else if b > a {
                    (4, 1)
                } else {
                    (1, 4)
                };
                arms[a] |= forward;
                arms[b] |= back;
            }
            for &at in &path {
                if !matches!(canvas.cells[at].role, Role::Border(_)) {
                    canvas.cells[at] = Cell {
                        glyph: Glyph::Char(line(arms[at])),
                        role: Role::Edge,
                    };
                }
            }
            canvas.cells[start.1 * width + start.0].glyph = Glyph::Char('├');
            canvas.cells[end.1 * width + end.0].glyph = Glyph::Char('┤');
        }
        for (index, (node, &rect)) in nodes.iter().zip(places).enumerate() {
            canvas.text(
                rect.x + 1,
                rect.y,
                rect.width - 2,
                node.title,
                Role::Title(index),
            );
            for (row, text) in node
                .body
                .lines()
                .take(usize::from(rect.height - 2))
                .enumerate()
            {
                canvas.text(
                    rect.x + 1,
                    rect.y + 1 + u16::try_from(row).unwrap_or(0),
                    rect.width - 2,
                    text,
                    Role::Body,
                );
            }
        }
        Ok(canvas)
    }

    fn node(&mut self, index: usize, rect: Rect) {
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                let top = y == rect.y;
                let bottom = y == rect.y + rect.height - 1;
                let left = x == rect.x;
                let right = x == rect.x + rect.width - 1;
                let glyph = match (top, bottom, left, right) {
                    (true, _, true, _) => '╭',
                    (true, _, _, true) => '╮',
                    (_, true, true, _) => '╰',
                    (_, true, _, true) => '╯',
                    (true, _, _, _) | (_, true, _, _) => '─',
                    (_, _, true, _) | (_, _, _, true) => '│',
                    _ => ' ',
                };
                let border = top || bottom || left || right;
                self.cells[usize::from(y) * self.width + usize::from(x)] = Cell {
                    glyph: Glyph::Char(glyph),
                    role: if border {
                        Role::Border(index)
                    } else {
                        Role::Body
                    },
                };
            }
        }
    }

    fn text(&mut self, x: u16, y: u16, width: u16, text: &str, role: Role) {
        let mut column = 0;
        for cluster in text.graphemes(true) {
            let columns = cluster_display_width(cluster);
            if columns == 0 {
                continue;
            }
            if columns > usize::from(width - column) {
                break;
            }
            let at = usize::from(y) * self.width + usize::from(x + column);
            let glyph = if cluster.len() == 1 && cluster.is_ascii() {
                Glyph::Char(char::from(cluster.as_bytes()[0]))
            } else {
                let index = self.clusters.len();
                self.clusters.push(cluster.to_owned());
                Glyph::Cluster(index)
            };
            self.cells[at] = Cell { glyph, role };
            if columns > 1 {
                self.cells[at + 1] = Cell {
                    glyph: Glyph::Tail,
                    role,
                };
            }
            column += u16::try_from(columns).unwrap_or(0);
        }
    }

    fn route(
        &self,
        start: (usize, usize),
        end: (usize, usize),
    ) -> Result<Vec<usize>, NodeGraphError> {
        let start = start.1 * self.width + start.0;
        let goal = end.1 * self.width + end.0;
        let mut distance = vec![u32::MAX; self.cells.len()];
        let mut previous = vec![usize::MAX; self.cells.len()];
        let mut queue = BinaryHeap::new();
        distance[start] = 0;
        queue.push(Reverse((0_u32, 0_u32, start)));
        while let Some(Reverse((_, cost, at))) = queue.pop() {
            if at == goal {
                break;
            }
            if cost != distance[at] {
                continue;
            }
            let (x, y) = (at % self.width, at / self.width);
            let candidates = [
                x.checked_add(1)
                    .filter(|&x| x < self.width)
                    .map(|x| y * self.width + x),
                x.checked_sub(1).map(|x| y * self.width + x),
                y.checked_add(1)
                    .filter(|&y| y < self.height)
                    .map(|y| y * self.width + x),
                y.checked_sub(1).map(|y| y * self.width + x),
            ];
            for next in candidates.into_iter().flatten() {
                if next != goal && matches!(self.cells[next].role, Role::Border(_) | Role::Body) {
                    continue;
                }
                let cost = cost + 1;
                if cost >= distance[next] {
                    continue;
                }
                distance[next] = cost;
                previous[next] = at;
                let heuristic =
                    (next % self.width).abs_diff(end.0) + (next / self.width).abs_diff(end.1);
                queue.push(Reverse((
                    cost + u32::try_from(heuristic).unwrap_or(0),
                    cost,
                    next,
                )));
            }
        }
        if distance[goal] == u32::MAX {
            return Err(NodeGraphError::NoRoute);
        }
        let mut path = vec![goal];
        while path[path.len() - 1] != start {
            path.push(previous[path[path.len() - 1]]);
        }
        path.reverse();
        Ok(path)
    }
}

fn line(mask: u8) -> char {
    match mask {
        1 | 4 | 5 => '│',
        2 | 8 | 10 => '─',
        3 => '╰',
        6 => '╭',
        9 => '╯',
        12 => '╮',
        7 => '├',
        11 => '┴',
        13 => '┤',
        14 => '┬',
        _ => '┼',
    }
}
