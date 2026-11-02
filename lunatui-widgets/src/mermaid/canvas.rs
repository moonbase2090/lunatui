// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::char_width;

pub(super) const NORTH: u8 = 1;
pub(super) const EAST: u8 = 2;
pub(super) const SOUTH: u8 = 4;
pub(super) const WEST: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Weight {
    Dotted,
    Solid,
    Thick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    None,
    Fill,
    Node,
    Text,
    Edge,
    Label,
    Life,
    Message,
    Note,
    NoteText,
    Frame,
    Tag,
    Fail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Dir {
    North,
    East,
    South,
    West,
}

impl Dir {
    pub(super) const fn arm(self) -> u8 {
        match self {
            Self::North => NORTH,
            Self::East => EAST,
            Self::South => SOUTH,
            Self::West => WEST,
        }
    }

    pub(super) const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Glyph {
    Empty,
    Line {
        mask: u8,
        weight: Weight,
        round: bool,
        locked: bool,
    },
    Arrow(Dir),
    Deco(char),
    Text(char),
    Tail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cell {
    pub(super) glyph: Glyph,
    pub(super) role: Role,
}

const BLANK: Cell = Cell {
    glyph: Glyph::Empty,
    role: Role::None,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct Arms {
    pub(super) mask: u8,
    pub(super) weight: Weight,
    pub(super) round: bool,
    pub(super) locked: bool,
}

impl Arms {
    pub(super) const fn new(mask: u8, weight: Weight) -> Self {
        Self {
            mask,
            weight,
            round: false,
            locked: false,
        }
    }

    pub(super) const fn border(mask: u8, round: bool) -> Self {
        Self {
            mask,
            weight: Weight::Solid,
            round,
            locked: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Canvas {
    width: i32,
    height: i32,
    cells: Vec<Cell>,
}

impl Canvas {
    pub(super) fn new(width: i32, height: i32) -> Self {
        let width = width.max(0);
        let height = height.max(0);
        let count = usize::try_from(width).unwrap_or(0) * usize::try_from(height).unwrap_or(0);
        Self {
            width,
            height,
            cells: vec![BLANK; count],
        }
    }

    pub(super) const fn width(&self) -> i32 {
        self.width
    }

    pub(super) const fn height(&self) -> i32 {
        self.height
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        usize::try_from(y * self.width + x).ok()
    }

    pub(super) fn get(&self, x: i32, y: i32) -> Cell {
        self.index(x, y)
            .and_then(|at| self.cells.get(at))
            .copied()
            .unwrap_or(BLANK)
    }

    pub(super) fn set(&mut self, x: i32, y: i32, glyph: Glyph, role: Role) {
        if let Some(cell) = self.index(x, y).and_then(|at| self.cells.get_mut(at)) {
            *cell = Cell { glyph, role };
        }
    }

    pub(super) fn paint(&mut self, x: i32, y: i32, width: i32, height: i32, role: Role) {
        for row in y..y.saturating_add(height) {
            for column in x..x.saturating_add(width) {
                if self.get(column, row).glyph == Glyph::Empty {
                    self.set(column, row, Glyph::Empty, role);
                }
            }
        }
    }

    pub(super) fn line(&mut self, x: i32, y: i32, arms: Arms, role: Role) {
        let cell = self.get(x, y);
        let glyph = match cell.glyph {
            Glyph::Line {
                mask,
                weight,
                round,
                locked,
            } => Glyph::Line {
                mask: mask | arms.mask,
                weight: if locked {
                    weight
                } else {
                    weight.max(arms.weight)
                },
                round: round || arms.round,
                locked: locked || arms.locked,
            },
            Glyph::Empty | Glyph::Tail => Glyph::Line {
                mask: arms.mask,
                weight: arms.weight,
                round: arms.round,
                locked: arms.locked,
            },
            Glyph::Arrow(_) | Glyph::Deco(_) | Glyph::Text(_) => return,
        };
        let role = if matches!(cell.glyph, Glyph::Line { .. }) {
            cell.role
        } else {
            role
        };
        self.set(x, y, glyph, role);
    }

    pub(super) fn text(&mut self, x: i32, y: i32, text: &str, role: Role) -> i32 {
        let mut at = x;
        for ch in text.chars() {
            let columns = char_width(ch);
            if columns == 0 {
                continue;
            }
            self.set(at, y, Glyph::Text(ch), role);
            if columns > 1 {
                self.set(at + 1, y, Glyph::Tail, role);
            }
            at += i32::try_from(columns).unwrap_or(1);
        }
        at - x
    }
}

pub(super) fn text_columns(text: &str) -> i32 {
    text.chars()
        .map(|ch| i32::try_from(char_width(ch)).unwrap_or(0))
        .sum()
}
