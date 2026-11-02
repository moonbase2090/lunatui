// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Text form of a [`SplitTree`], for saving and restoring a layout.
//!
//! ```text
//! spec   = "split1;" next ";" zoom ";" node
//! zoom   = "-" | number
//! node   = number | axis number ":" number ":" number "(" node "," node ")"
//! axis   = "h" | "v"
//! number = "0" | digit-without-leading-zero { digit }
//! ```
//!
//! A leaf is its pane id. A split is its axis (`h` for [`SplitAxis::Horizontal`], `v` for
//! [`SplitAxis::Vertical`]), then the first pane's ratio in thousandths, the first and second
//! minimums, and the two children in parentheses. `next` is the id the next new pane gets and
//! `zoom` the zoomed pane or `-`. For example `split1;3;-;h500:0:0(0,v700:12:12(1,2))`.
//!
//! The form has exactly one spelling for each tree: no spaces, signs or leading zeros.

use std::fmt::{self, Write};

use super::{FULL, Node, PaneId, Split, SplitAxis, SplitTree};

/// Longest spec [`SplitTree::from_spec`] reads, in bytes.
const MAX_LEN: usize = 8192;
/// Deepest nesting of splits [`SplitTree::from_spec`] reads.
const MAX_DEPTH: usize = 64;
const HEADER: &str = "split1;";

/// Why [`SplitTree::from_spec`] refused a spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SplitSpecError {
    /// The spec is longer than 8192 bytes.
    TooLong,
    /// The spec does not start with `split1;`.
    BadHeader,
    /// Splits nest more than 64 deep.
    TooDeep,
    /// The spec ends in the middle of an item.
    UnexpectedEnd,
    /// A character that does not belong, at this byte offset.
    Unexpected(usize),
    /// A number whose value does not fit, at this byte offset.
    Overflow(usize),
    /// A ratio above 1000, at this byte offset.
    RatioTooLarge(usize),
    /// Two panes share this id.
    DuplicatePane(u32),
    /// A pane id is not below the next-id counter, so a new pane could reuse it.
    NextTooLow,
    /// The zoomed pane is not in the tree.
    UnknownZoom(u32),
    /// Text after the end of the tree, at this byte offset.
    TrailingInput(usize),
}

impl fmt::Display for SplitSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(f, "split spec is longer than {MAX_LEN} bytes"),
            Self::BadHeader => write!(f, "split spec does not start with {HEADER:?}"),
            Self::TooDeep => write!(f, "split spec nests splits more than {MAX_DEPTH} deep"),
            Self::UnexpectedEnd => write!(f, "split spec ends too soon"),
            Self::Unexpected(at) => write!(f, "unexpected character in split spec at byte {at}"),
            Self::Overflow(at) => write!(f, "number in split spec at byte {at} is too large"),
            Self::RatioTooLarge(at) => {
                write!(f, "ratio in split spec at byte {at} is above {FULL}")
            }
            Self::DuplicatePane(id) => write!(f, "pane {id} appears twice in split spec"),
            Self::NextTooLow => write!(f, "split spec's next id does not exceed its pane ids"),
            Self::UnknownZoom(id) => write!(f, "split spec zooms pane {id}, which is not in it"),
            Self::TrailingInput(at) => write!(f, "split spec has extra text at byte {at}"),
        }
    }
}

impl std::error::Error for SplitSpecError {}

impl SplitTree {
    /// The tree as text: the axis, ratio and minimums of every split, the pane ids, the next-id
    /// counter and the zoomed pane, in the form the `split` module's spec grammar describes
    /// (for example `split1;3;-;h500:0:0(0,v700:12:12(1,2))`).
    ///
    /// [`SplitTree::from_spec`] reads it back to an equal tree. A tree with splits nested more
    /// than 64 deep, or a spec past 8192 bytes, is written but cannot be read back.
    #[must_use]
    pub fn to_spec(&self) -> String {
        let mut out = format!("{HEADER}{};", self.next);
        match self.zoomed {
            Some(PaneId(id)) => out.push_str(&id.to_string()),
            None => out.push('-'),
        }
        out.push(';');
        write_node(&self.root, &mut out);
        out
    }

    /// Reads a tree written by [`SplitTree::to_spec`]. The text is not trusted: it is refused
    /// when it is longer than 8192 bytes, nests splits more than 64 deep, repeats a pane id,
    /// has a ratio above 1000, a number that overflows, a next-id counter that does not exceed
    /// every pane id, a zoomed pane that is not in the tree, or any text after the tree.
    ///
    /// # Errors
    ///
    /// Returns a [`SplitSpecError`] naming the first problem found.
    pub fn from_spec(spec: &str) -> Result<Self, SplitSpecError> {
        if spec.len() > MAX_LEN {
            return Err(SplitSpecError::TooLong);
        }
        if !spec.starts_with(HEADER) {
            return Err(SplitSpecError::BadHeader);
        }
        let mut reader = Reader {
            bytes: spec.as_bytes(),
            at: HEADER.len(),
            ids: Vec::new(),
        };
        let next = reader.number()?;
        reader.expect(b';')?;
        let zoomed = if reader.peek() == Some(b'-') {
            reader.at += 1;
            None
        } else {
            Some(reader.number()?)
        };
        reader.expect(b';')?;
        let root = reader.node(0)?;
        if reader.at < reader.bytes.len() {
            return Err(SplitSpecError::TrailingInput(reader.at));
        }
        let mut ids = reader.ids;
        ids.sort_unstable();
        if let Some(pair) = ids.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(SplitSpecError::DuplicatePane(pair[0]));
        }
        if ids.last().is_some_and(|&largest| largest >= next) {
            return Err(SplitSpecError::NextTooLow);
        }
        if let Some(id) = zoomed
            && ids.binary_search(&id).is_err()
        {
            return Err(SplitSpecError::UnknownZoom(id));
        }
        Ok(Self {
            root,
            next,
            zoomed: zoomed.map(PaneId),
        })
    }
}

fn write_node(node: &Node, out: &mut String) {
    match node {
        Node::Leaf(PaneId(id)) => out.push_str(&id.to_string()),
        Node::Split(split) => {
            out.push(match split.axis {
                SplitAxis::Horizontal => 'h',
                SplitAxis::Vertical => 'v',
            });
            let _ = write!(out, "{}:{}:{}(", split.ratio, split.min[0], split.min[1]);
            write_node(&split.children[0], out);
            out.push(',');
            write_node(&split.children[1], out);
            out.push(')');
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    ids: Vec<u32>,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn expect(&mut self, want: u8) -> Result<(), SplitSpecError> {
        match self.peek() {
            Some(byte) if byte == want => {
                self.at += 1;
                Ok(())
            }
            Some(_) => Err(SplitSpecError::Unexpected(self.at)),
            None => Err(SplitSpecError::UnexpectedEnd),
        }
    }

    fn number(&mut self) -> Result<u32, SplitSpecError> {
        let start = self.at;
        let mut value = 0_u32;
        while let Some(digit @ b'0'..=b'9') = self.peek() {
            if self.at > start && self.bytes[start] == b'0' {
                return Err(SplitSpecError::Unexpected(self.at));
            }
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u32::from(digit - b'0')))
                .ok_or(SplitSpecError::Overflow(start))?;
            self.at += 1;
        }
        if self.at == start {
            return Err(match self.peek() {
                Some(_) => SplitSpecError::Unexpected(start),
                None => SplitSpecError::UnexpectedEnd,
            });
        }
        Ok(value)
    }

    fn small(&mut self) -> Result<u16, SplitSpecError> {
        let start = self.at;
        u16::try_from(self.number()?).map_err(|_| SplitSpecError::Overflow(start))
    }

    fn node(&mut self, depth: usize) -> Result<Node, SplitSpecError> {
        let axis = match self.peek() {
            Some(b'h') => SplitAxis::Horizontal,
            Some(b'v') => SplitAxis::Vertical,
            Some(b'0'..=b'9') => {
                let id = self.number()?;
                self.ids.push(id);
                return Ok(Node::Leaf(PaneId(id)));
            }
            Some(_) => return Err(SplitSpecError::Unexpected(self.at)),
            None => return Err(SplitSpecError::UnexpectedEnd),
        };
        if depth >= MAX_DEPTH {
            return Err(SplitSpecError::TooDeep);
        }
        self.at += 1;
        let at = self.at;
        let ratio = self.small()?;
        if ratio > FULL {
            return Err(SplitSpecError::RatioTooLarge(at));
        }
        self.expect(b':')?;
        let first = self.small()?;
        self.expect(b':')?;
        let second = self.small()?;
        self.expect(b'(')?;
        let a = self.node(depth + 1)?;
        self.expect(b',')?;
        let b = self.node(depth + 1)?;
        self.expect(b')')?;
        Ok(Node::Split(Box::new(Split {
            axis,
            ratio,
            min: [first, second],
            children: [a, b],
        })))
    }
}
