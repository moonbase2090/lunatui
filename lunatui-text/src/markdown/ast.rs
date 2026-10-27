// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// A parsed Markdown document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    /// Top-level blocks in source order.
    pub blocks: Vec<Block>,
}

/// A block-level element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// A paragraph of inline content.
    Paragraph(Vec<Inline>),
    /// An ATX or setext heading.
    Heading {
        /// Heading level from 1 (largest) to 6.
        level: u8,
        /// Inline content of the heading.
        content: Vec<Inline>,
    },
    /// A thematic break (`---`, `***` or `___`).
    ThematicBreak,
    /// A block quote and the blocks it contains.
    BlockQuote(Vec<Block>),
    /// A bullet or ordered list.
    List(List),
    /// An indented or fenced code block.
    CodeBlock {
        /// Info string of a fenced block; empty for indented blocks.
        info: String,
        /// The code, with a trailing newline when the block is not empty.
        code: String,
    },
    /// A raw HTML block, kept as written.
    Html(String),
    /// A table (GFM extension).
    Table(Table),
}

/// A list and its items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    /// Number of the first item of an ordered list; `None` for a bullet list.
    pub start: Option<u64>,
    /// Whether the list is tight: no blank lines between items or their blocks.
    pub tight: bool,
    /// The list items.
    pub items: Vec<Item>,
}

/// One list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// `Some` for a task item (`[ ]` or `[x]`); `Some(true)` when checked.
    pub checked: Option<bool>,
    /// Blocks inside the item.
    pub blocks: Vec<Block>,
}

/// Horizontal alignment of a table column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    /// No alignment given.
    #[default]
    None,
    /// Left aligned (`:---`).
    Left,
    /// Centered (`:---:`).
    Center,
    /// Right aligned (`---:`).
    Right,
}

/// A table: a header row, a delimiter row that sets alignment, and body rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    /// Alignment of each column.
    pub alignments: Vec<Alignment>,
    /// Header cells.
    pub header: Vec<Vec<Inline>>,
    /// Body rows; each row has one cell per column.
    pub rows: Vec<Vec<Vec<Inline>>>,
}

/// An inline element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    /// Literal text with escapes and entities already resolved.
    Text(String),
    /// A code span with its delimiters and edge spaces removed.
    Code(String),
    /// Emphasis (`*x*` or `_x_`).
    Emphasis(Vec<Inline>),
    /// Strong emphasis (`**x**` or `__x__`).
    Strong(Vec<Inline>),
    /// Strikethrough (`~~x~~`, GFM extension).
    Strikethrough(Vec<Inline>),
    /// A link.
    Link {
        /// Destination, with escapes and entities resolved.
        dest: String,
        /// Title; empty when there is none.
        title: String,
        /// Link text.
        content: Vec<Inline>,
    },
    /// An image.
    Image {
        /// Destination, with escapes and entities resolved.
        dest: String,
        /// Title; empty when there is none.
        title: String,
        /// Alternative text as inline content.
        alt: Vec<Inline>,
    },
    /// A soft line break inside a paragraph.
    SoftBreak,
    /// A hard line break.
    HardBreak,
    /// Raw inline HTML, kept as written.
    Html(String),
}

/// A syntax extension beyond `CommonMark`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extension {
    /// Pipe tables.
    Tables,
    /// `~~strikethrough~~`.
    Strikethrough,
    /// `[ ]` and `[x]` task markers at the start of list items.
    TaskLists,
    /// Bare `www.` addresses, `http(s)://` URLs and email addresses become links.
    Autolinks,
}

impl Extension {
    const fn bit(self) -> u8 {
        match self {
            Self::Tables => 1,
            Self::Strikethrough => 2,
            Self::TaskLists => 4,
            Self::Autolinks => 8,
        }
    }
}

/// Parser switches: which extensions are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    extensions: u8,
}

impl Options {
    /// Strict `CommonMark`: every extension off.
    #[must_use]
    pub const fn commonmark() -> Self {
        Self { extensions: 0 }
    }

    /// `CommonMark` plus the GitHub Flavored Markdown extensions.
    #[must_use]
    pub const fn gfm() -> Self {
        Self {
            extensions: Extension::Tables.bit()
                | Extension::Strikethrough.bit()
                | Extension::TaskLists.bit()
                | Extension::Autolinks.bit(),
        }
    }

    /// These options with `extension` turned on.
    #[must_use]
    pub const fn with(self, extension: Extension) -> Self {
        Self {
            extensions: self.extensions | extension.bit(),
        }
    }

    /// These options with `extension` turned off.
    #[must_use]
    pub const fn without(self, extension: Extension) -> Self {
        Self {
            extensions: self.extensions & !extension.bit(),
        }
    }

    /// Whether `extension` is on.
    #[must_use]
    pub const fn enabled(self, extension: Extension) -> bool {
        self.extensions & extension.bit() != 0
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::gfm()
    }
}
