// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod ast;
mod autolink;
mod block;
mod chars;
mod html;
mod inline;
mod links;
mod table;
mod tables;

pub use ast::{Alignment, Block, Document, Inline, Item, List, Options, Table};

/// Parses `source` as Markdown.
///
/// The parser follows `CommonMark`. Tables, strikethrough, task lists and bare
/// autolinks are on by default; see [`Options`].
#[must_use]
pub fn parse(source: &str, options: Options) -> Document {
    block::parse(source, options)
}
