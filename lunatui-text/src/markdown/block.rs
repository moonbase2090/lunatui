// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::ast::{Alignment, Block, Document, Inline, Item, List, Options, Table};
use super::chars::unescape;
use super::html::{html_block_ends, html_block_start};
use super::inline;
use super::links::{Refs, parse_definition};
use super::table::{delimiter_row, split_row};

const MAX_DEPTH: usize = 64;

#[derive(Debug)]
enum Kind {
    Document,
    BlockQuote,
    List {
        ordered: bool,
        start: u64,
        delim: u8,
    },
    Item {
        content_indent: usize,
    },
    Paragraph,
    Heading(u8),
    ThematicBreak,
    FencedCode {
        fence_char: u8,
        fence_len: usize,
        indent: usize,
        info: String,
    },
    IndentedCode,
    Html(u8),
    Table {
        alignments: Vec<Alignment>,
        header: String,
    },
    Removed,
}

#[derive(Debug)]
struct Node {
    kind: Kind,
    parent: usize,
    children: Vec<usize>,
    open: bool,
    text: String,
    lines: usize,
    ends_blank: bool,
}

struct Cursor<'a> {
    line: &'a str,
    offset: usize,
    column: usize,
    tab_rem: usize,
}

impl<'a> Cursor<'a> {
    fn new(line: &'a str) -> Self {
        Self {
            line,
            offset: 0,
            column: 0,
            tab_rem: 0,
        }
    }

    fn skip_offset(&self) -> usize {
        let bytes = self.line.as_bytes();
        let mut index = if self.tab_rem > 0 {
            self.offset + 1
        } else {
            self.offset
        };
        while index < bytes.len() && matches!(bytes[index], b' ' | b'\t') {
            index += 1;
        }
        index
    }

    fn indent(&self) -> usize {
        let bytes = self.line.as_bytes();
        let mut column = self.column;
        let mut count = 0;
        let mut index = self.offset;
        if self.tab_rem > 0 {
            count += self.tab_rem;
            column += self.tab_rem;
            index += 1;
        }
        while index < bytes.len() {
            match bytes[index] {
                b' ' => {
                    count += 1;
                    column += 1;
                }
                b'\t' => {
                    let width = 4 - column % 4;
                    count += width;
                    column += width;
                }
                _ => break,
            }
            index += 1;
        }
        count
    }

    fn is_blank(&self) -> bool {
        self.line[self.offset..]
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'))
    }

    fn rest_nonblank(&self) -> &'a str {
        &self.line[self.skip_offset()..]
    }

    fn advance_columns(&mut self, count: usize) {
        let bytes = self.line.as_bytes();
        let mut left = count;
        while left > 0 {
            if self.tab_rem > 0 {
                let take = left.min(self.tab_rem);
                self.tab_rem -= take;
                self.column += take;
                left -= take;
                if self.tab_rem == 0 {
                    self.offset += 1;
                }
                continue;
            }
            match bytes.get(self.offset) {
                Some(b' ') => {
                    self.offset += 1;
                    self.column += 1;
                    left -= 1;
                }
                Some(b'\t') => {
                    let width = 4 - self.column % 4;
                    if width <= left {
                        self.offset += 1;
                        self.column += width;
                        left -= width;
                    } else {
                        self.tab_rem = width - left;
                        self.column += left;
                        left = 0;
                    }
                }
                _ => break,
            }
        }
    }

    fn advance_bytes(&mut self, count: usize) {
        self.offset += count;
        self.column += count;
    }

    fn rest_text(&self) -> String {
        if self.tab_rem > 0 {
            let mut text = " ".repeat(self.tab_rem);
            text.push_str(&self.line[self.offset + 1..]);
            text
        } else {
            String::from(&self.line[self.offset..])
        }
    }
}

struct LineState {
    container: usize,
    all_matched: bool,
    started: bool,
    consumed: bool,
}

enum Start {
    None,
    Container,
    Leaf,
    Retry,
}

struct Blocks {
    nodes: Vec<Node>,
    refs: Refs,
    options: Options,
}

pub(super) fn parse(source: &str, options: Options) -> Document {
    let normalized = normalize(source);
    let mut blocks = Blocks::new(options);
    let mut lines = normalized.split('\n').peekable();
    while let Some(line) = lines.next() {
        if lines.peek().is_none() && line.is_empty() {
            break;
        }
        blocks.line(line);
    }
    blocks.finish()
}

fn normalize(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            '\0' => out.push('\u{FFFD}'),
            _ => out.push(c),
        }
    }
    out
}

impl Blocks {
    fn new(options: Options) -> Self {
        Self {
            nodes: alloc::vec![Node {
                kind: Kind::Document,
                parent: 0,
                children: Vec::new(),
                open: true,
                text: String::new(),
                lines: 0,
                ends_blank: false,
            }],
            refs: Refs::default(),
            options,
        }
    }

    fn tip(&self) -> usize {
        let mut id = 0;
        while let Some(&last) = self.nodes[id].children.last() {
            if !self.nodes[last].open {
                break;
            }
            id = last;
        }
        id
    }

    fn depth(&self, mut id: usize) -> usize {
        let mut depth = 0;
        while id != 0 {
            id = self.nodes[id].parent;
            depth += 1;
        }
        depth
    }

    fn can_contain(&self, parent: usize, child: &Kind) -> bool {
        match self.nodes[parent].kind {
            Kind::Document | Kind::BlockQuote | Kind::Item { .. } => {
                !matches!(child, Kind::Item { .. })
            }
            Kind::List { .. } => matches!(child, Kind::Item { .. }),
            _ => false,
        }
    }

    fn push_node(&mut self, parent: usize, kind: Kind, open: bool) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node {
            kind,
            parent,
            children: Vec::new(),
            open,
            text: String::new(),
            lines: 0,
            ends_blank: false,
        });
        self.nodes[parent].children.push(id);
        id
    }

    fn add_child(&mut self, mut parent: usize, kind: Kind, open: bool) -> usize {
        while !self.can_contain(parent, &kind) {
            parent = self.close(parent);
        }
        self.push_node(parent, kind, open)
    }

    fn close(&mut self, id: usize) -> usize {
        let parent = self.nodes[id].parent;
        if !self.nodes[id].open {
            return parent;
        }
        self.nodes[id].open = false;
        match self.nodes[id].kind {
            Kind::Paragraph => self.finalize_paragraph(id),
            Kind::IndentedCode => trim_blank_tail(&mut self.nodes[id].text),
            _ => {}
        }
        parent
    }

    fn close_unmatched(&mut self, container: usize) {
        let mut tip = self.tip();
        while tip != container {
            tip = self.close(tip);
        }
    }

    fn finalize_paragraph(&mut self, id: usize) {
        let mut text = core::mem::take(&mut self.nodes[id].text);
        while let Some(definition) = parse_definition(&text) {
            self.refs
                .insert(&definition.label, definition.dest, definition.title);
            text = String::from(&text[definition.consumed..]);
        }
        let trimmed = text.trim_end_matches([' ', '\t', '\n']);
        if trimmed.is_empty() {
            self.nodes[id].kind = Kind::Removed;
        } else {
            self.nodes[id].text = String::from(trimmed);
        }
    }

    fn add_line(&mut self, id: usize, text: &str) {
        let node = &mut self.nodes[id];
        if node.lines > 0 {
            node.text.push('\n');
        }
        node.text.push_str(text);
        node.lines += 1;
    }

    fn add_code_line(&mut self, id: usize, text: &str) {
        let node = &mut self.nodes[id];
        node.text.push_str(text);
        node.text.push('\n');
        node.lines += 1;
    }

    fn line(&mut self, line: &str) {
        let mut cursor = Cursor::new(line);
        let Some((container, all_matched)) = self.match_open(&mut cursor) else {
            return;
        };
        let mut state = LineState {
            container,
            all_matched,
            started: false,
            consumed: false,
        };
        self.open_new_blocks(&mut cursor, &mut state);
        self.add_text(&mut cursor, &mut state);
    }

    fn match_open(&mut self, cursor: &mut Cursor<'_>) -> Option<(usize, bool)> {
        let mut container = 0;
        loop {
            let Some(&child) = self.nodes[container].children.last() else {
                return Some((container, true));
            };
            if !self.nodes[child].open {
                return Some((container, true));
            }
            let blank = cursor.is_blank();
            match &self.nodes[child].kind {
                Kind::BlockQuote => {
                    if !eat_quote_marker(cursor) {
                        return Some((container, false));
                    }
                    container = child;
                }
                Kind::List { .. } => container = child,
                Kind::Item { content_indent } => {
                    let content_indent = *content_indent;
                    if blank {
                        if self.nodes[child].children.is_empty() {
                            return Some((container, false));
                        }
                    } else if cursor.indent() >= content_indent {
                        cursor.advance_columns(content_indent);
                    } else {
                        return Some((container, false));
                    }
                    container = child;
                }
                Kind::Paragraph | Kind::Table { .. } => {
                    return Some(if blank {
                        (container, false)
                    } else {
                        (child, true)
                    });
                }
                Kind::FencedCode { .. } => return self.match_fence(cursor, child),
                Kind::IndentedCode => {
                    let indent = cursor.indent();
                    if indent >= 4 {
                        cursor.advance_columns(4);
                    } else if blank {
                        cursor.advance_columns(indent);
                    } else {
                        return Some((container, false));
                    }
                    return Some((child, true));
                }
                Kind::Html(kind) => {
                    let ends_at_blank = *kind >= 6;
                    return Some(if blank && ends_at_blank {
                        (container, false)
                    } else {
                        (child, true)
                    });
                }
                Kind::Document | Kind::Heading(_) | Kind::ThematicBreak | Kind::Removed => {
                    return Some((container, false));
                }
            }
        }
    }

    fn match_fence(&mut self, cursor: &mut Cursor<'_>, child: usize) -> Option<(usize, bool)> {
        let Kind::FencedCode {
            fence_char,
            fence_len,
            indent,
            ..
        } = self.nodes[child].kind
        else {
            return Some((child, true));
        };
        let line_indent = cursor.indent();
        let rest = cursor.rest_nonblank();
        if line_indent <= 3 {
            let run = rest.bytes().take_while(|&byte| byte == fence_char).count();
            if run >= fence_len && rest[run..].bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
                self.close(child);
                return None;
            }
        }
        cursor.advance_columns(line_indent.min(indent));
        Some((child, true))
    }

    fn open_new_blocks(&mut self, cursor: &mut Cursor<'_>, state: &mut LineState) {
        loop {
            if matches!(
                self.nodes[state.container].kind,
                Kind::FencedCode { .. } | Kind::IndentedCode | Kind::Html(_)
            ) || cursor.is_blank()
            {
                return;
            }
            match self.try_start(cursor, state) {
                Start::Container | Start::Retry => {}
                Start::None | Start::Leaf => return,
            }
        }
    }

    fn text_leaf(&self, id: usize) -> bool {
        matches!(self.nodes[id].kind, Kind::Paragraph | Kind::Table { .. })
    }

    fn begin(&mut self, state: &mut LineState, kind: Kind, open: bool) -> usize {
        if !state.all_matched {
            self.close_unmatched(state.container);
            state.all_matched = true;
        }
        let id = self.add_child(state.container, kind, open);
        state.container = id;
        state.started = true;
        id
    }

    fn try_start(&mut self, cursor: &mut Cursor<'_>, state: &mut LineState) -> Start {
        let indent = cursor.indent();
        if indent >= 4 {
            let tip = self.tip();
            if matches!(self.nodes[tip].kind, Kind::Paragraph) || self.text_leaf(state.container) {
                return Start::None;
            }
            self.begin(state, Kind::IndentedCode, true);
            cursor.advance_columns(4);
            return Start::Leaf;
        }
        let rest = cursor.rest_nonblank();
        let first = rest.as_bytes()[0];
        let nested = self.depth(state.container) < MAX_DEPTH;
        if first == b'>' && nested {
            cursor.advance_columns(indent);
            cursor.advance_bytes(1);
            if matches!(
                cursor.line.as_bytes().get(cursor.offset),
                Some(b' ' | b'\t')
            ) {
                cursor.advance_columns(1);
            }
            self.begin(state, Kind::BlockQuote, true);
            return Start::Container;
        }
        if first == b'#'
            && let Some(start) = self.atx_heading(cursor, state, rest)
        {
            return start;
        }
        if (first == b'`' || first == b'~')
            && let Some(start) = self.fenced_code(cursor, state, indent, rest)
        {
            return start;
        }
        if first == b'<' {
            let seventh_allowed = !self.text_leaf(state.container);
            if let Some(kind) = html_block_start(rest, seventh_allowed) {
                self.begin(state, Kind::Html(kind), true);
                return Start::Leaf;
            }
        }
        if self.text_leaf(state.container)
            && matches!(first, b'=' | b'-' | b'|' | b':')
            && let Some(start) = self.table_or_setext(cursor, state, rest)
        {
            return start;
        }
        if is_thematic_break(rest) {
            self.begin(state, Kind::ThematicBreak, false);
            state.consumed = true;
            return Start::Leaf;
        }
        if nested && let Some(start) = self.list_item(cursor, state, indent, rest) {
            return start;
        }
        Start::None
    }

    fn atx_heading(
        &mut self,
        cursor: &mut Cursor<'_>,
        state: &mut LineState,
        rest: &str,
    ) -> Option<Start> {
        let level = rest.bytes().take_while(|&byte| byte == b'#').count();
        if level > 6 {
            return None;
        }
        let after = &rest[level..];
        if !(after.is_empty() || after.starts_with([' ', '\t'])) {
            return None;
        }
        let content = atx_content(after);
        let id = self.begin(
            state,
            Kind::Heading(u8::try_from(level).unwrap_or(6)),
            false,
        );
        self.nodes[id].text = content;
        state.consumed = true;
        cursor.offset = cursor.line.len();
        Some(Start::Leaf)
    }

    fn fenced_code(
        &mut self,
        cursor: &mut Cursor<'_>,
        state: &mut LineState,
        indent: usize,
        rest: &str,
    ) -> Option<Start> {
        let fence_char = rest.as_bytes()[0];
        let fence_len = rest.bytes().take_while(|&byte| byte == fence_char).count();
        if fence_len < 3 {
            return None;
        }
        let info = rest[fence_len..].trim_matches([' ', '\t']);
        if fence_char == b'`' && info.contains('`') {
            return None;
        }
        let info = unescape(info);
        self.begin(
            state,
            Kind::FencedCode {
                fence_char,
                fence_len,
                indent,
                info,
            },
            true,
        );
        state.consumed = true;
        cursor.offset = cursor.line.len();
        Some(Start::Leaf)
    }

    fn table_or_setext(
        &mut self,
        cursor: &mut Cursor<'_>,
        state: &mut LineState,
        rest: &str,
    ) -> Option<Start> {
        let container = state.container;
        let is_paragraph = matches!(self.nodes[container].kind, Kind::Paragraph);
        if !is_paragraph {
            return None;
        }
        if self.options.tables
            && let Some(alignments) = delimiter_row(rest)
            && let Some(start) = self.start_table(cursor, state, alignments)
        {
            return Some(start);
        }
        let trimmed = rest.trim_end_matches([' ', '\t']);
        let level = if !trimmed.is_empty() && trimmed.bytes().all(|byte| byte == b'=') {
            1
        } else if !trimmed.is_empty() && trimmed.bytes().all(|byte| byte == b'-') {
            2
        } else {
            return None;
        };
        Some(self.setext_heading(cursor, state, level))
    }

    fn setext_heading(
        &mut self,
        cursor: &mut Cursor<'_>,
        state: &mut LineState,
        level: u8,
    ) -> Start {
        let id = state.container;
        self.finalize_paragraph(id);
        self.nodes[id].open = false;
        if matches!(self.nodes[id].kind, Kind::Removed) {
            state.container = self.nodes[id].parent;
            state.all_matched = true;
            return Start::Retry;
        }
        self.nodes[id].kind = Kind::Heading(level);
        state.container = self.nodes[id].parent;
        state.all_matched = true;
        state.consumed = true;
        cursor.offset = cursor.line.len();
        Start::Leaf
    }

    fn start_table(
        &mut self,
        cursor: &mut Cursor<'_>,
        state: &mut LineState,
        alignments: Vec<Alignment>,
    ) -> Option<Start> {
        let id = state.container;
        let text = &self.nodes[id].text;
        let (head, header) = match text.rsplit_once('\n') {
            Some((head, header)) => (Some(String::from(head)), String::from(header)),
            None => (None, text.clone()),
        };
        if split_row(&header).len() != alignments.len() {
            return None;
        }
        let parent = self.nodes[id].parent;
        if let Some(head) = head {
            self.nodes[id].text = head;
            self.close(id);
        } else {
            self.nodes[id].text.clear();
            self.nodes[id].kind = Kind::Removed;
            self.nodes[id].open = false;
        }
        let table = self.push_node(parent, Kind::Table { alignments, header }, true);
        state.container = table;
        state.all_matched = true;
        state.started = true;
        state.consumed = true;
        cursor.offset = cursor.line.len();
        Some(Start::Leaf)
    }

    fn list_item(
        &mut self,
        cursor: &mut Cursor<'_>,
        state: &mut LineState,
        indent: usize,
        rest: &str,
    ) -> Option<Start> {
        let (ordered, start, delim, marker_len) = parse_marker(rest)?;
        let after = &rest[marker_len..];
        let blank_after = after.bytes().all(|byte| matches!(byte, b' ' | b'\t'));
        if self.text_leaf(state.container) && (blank_after || (ordered && start != 1)) {
            return None;
        }
        cursor.advance_columns(indent);
        cursor.advance_bytes(marker_len);
        let spaces = cursor.indent();
        let (padding, consume) = if blank_after {
            (1, 0)
        } else if spaces >= 5 {
            (1, 1)
        } else {
            (spaces, spaces)
        };
        cursor.advance_columns(consume);
        let content_indent = indent + marker_len + padding;
        if !state.all_matched {
            self.close_unmatched(state.container);
            state.all_matched = true;
        }
        let mut parent = state.container;
        let reuse = matches!(
            self.nodes[parent].kind,
            Kind::List { ordered: o, delim: d, .. } if o == ordered && d == delim
        );
        if !reuse {
            parent = self.add_child(
                parent,
                Kind::List {
                    ordered,
                    start,
                    delim,
                },
                true,
            );
        }
        let item = self.add_child(parent, Kind::Item { content_indent }, true);
        state.container = item;
        state.started = true;
        Some(Start::Container)
    }

    fn add_text(&mut self, cursor: &mut Cursor<'_>, state: &mut LineState) {
        if state.consumed {
            return;
        }
        let blank = cursor.is_blank();
        self.note_blank(state, blank);
        let tip = self.tip();
        if !state.all_matched
            && !state.started
            && !blank
            && matches!(self.nodes[tip].kind, Kind::Paragraph)
        {
            let text = cursor.rest_text();
            self.add_line(tip, text.trim_start_matches([' ', '\t']));
            return;
        }
        if !state.all_matched {
            self.close_unmatched(state.container);
        }
        let container = state.container;
        match self.nodes[container].kind {
            Kind::FencedCode { .. } | Kind::IndentedCode => {
                let text = cursor.rest_text();
                self.add_code_line(container, &text);
            }
            Kind::Html(kind) => {
                let text = cursor.rest_text();
                self.add_line(container, &text);
                if html_block_ends(kind, &text) {
                    self.close(container);
                }
            }
            Kind::Paragraph => {
                let text = cursor.rest_text();
                self.add_line(container, text.trim_start_matches([' ', '\t']));
            }
            Kind::Table { .. } => {
                let text = cursor.rest_text();
                self.add_line(container, text.trim_matches([' ', '\t']));
            }
            _ => {
                if !blank {
                    let id = self.add_child(container, Kind::Paragraph, true);
                    let text = cursor.rest_text();
                    self.add_line(id, text.trim_start_matches([' ', '\t']));
                }
            }
        }
    }

    fn note_blank(&mut self, state: &LineState, blank: bool) {
        if blank && let Some(&last) = self.nodes[state.container].children.last() {
            self.nodes[last].ends_blank = true;
        }
    }

    fn finish(mut self) -> Document {
        self.close_unmatched(0);
        let root = self.nodes[0].children.clone();
        Document {
            blocks: self.convert_all(&root),
        }
    }

    fn convert_all(&self, ids: &[usize]) -> Vec<Block> {
        ids.iter().filter_map(|&id| self.convert(id)).collect()
    }

    fn inlines(&self, text: &str) -> Vec<Inline> {
        inline::parse(text, &self.refs, self.options)
    }

    fn convert(&self, id: usize) -> Option<Block> {
        let node = &self.nodes[id];
        Some(match &node.kind {
            Kind::Paragraph => Block::Paragraph(self.inlines(&node.text)),
            Kind::Heading(level) => Block::Heading {
                level: *level,
                content: self.inlines(&node.text),
            },
            Kind::ThematicBreak => Block::ThematicBreak,
            Kind::BlockQuote => Block::BlockQuote(self.convert_all(&node.children)),
            Kind::List { ordered, start, .. } => Block::List(List {
                start: ordered.then_some(*start),
                tight: self.list_is_tight(id),
                items: node.children.iter().map(|&item| self.item(item)).collect(),
            }),
            Kind::FencedCode { info, .. } => Block::CodeBlock {
                info: info.clone(),
                code: node.text.clone(),
            },
            Kind::IndentedCode => Block::CodeBlock {
                info: String::new(),
                code: node.text.clone(),
            },
            Kind::Html(_) => Block::Html(node.text.clone()),
            Kind::Table { alignments, header } => {
                Block::Table(self.table(node, alignments, header))
            }
            Kind::Item { .. } | Kind::Document | Kind::Removed => return None,
        })
    }

    fn item(&self, id: usize) -> Item {
        let children = &self.nodes[id].children;
        let mut checked = None;
        let mut blocks = Vec::new();
        for (index, &child) in children.iter().enumerate() {
            let node = &self.nodes[child];
            if index == 0
                && self.options.task_lists
                && matches!(node.kind, Kind::Paragraph)
                && let Some((state, rest)) = task_marker(&node.text)
            {
                checked = Some(state);
                blocks.push(Block::Paragraph(self.inlines(rest)));
                continue;
            }
            if let Some(block) = self.convert(child) {
                blocks.push(block);
            }
        }
        Item { checked, blocks }
    }

    fn table(&self, node: &Node, alignments: &[Alignment], header: &str) -> Table {
        let columns = alignments.len();
        let cells = |line: &str| -> Vec<Vec<Inline>> {
            let mut parts = split_row(line);
            parts.resize(columns, String::new());
            parts.iter().map(|cell| self.inlines(cell)).collect()
        };
        let rows = if node.lines == 0 {
            Vec::new()
        } else {
            node.text.split('\n').map(cells).collect()
        };
        Table {
            alignments: alignments.to_vec(),
            header: cells(header),
            rows,
        }
    }

    fn ends_with_blank(&self, id: usize) -> bool {
        let mut current = id;
        loop {
            if self.nodes[current].ends_blank {
                return true;
            }
            match self.nodes[current].kind {
                Kind::List { .. } | Kind::Item { .. } => {
                    match self.nodes[current].children.last() {
                        Some(&last) => current = last,
                        None => return false,
                    }
                }
                _ => return false,
            }
        }
    }

    fn list_is_tight(&self, id: usize) -> bool {
        let items = &self.nodes[id].children;
        for (index, &item) in items.iter().enumerate() {
            let has_next = index + 1 < items.len();
            if has_next && self.ends_with_blank(item) {
                return false;
            }
            let subs = &self.nodes[item].children;
            for (position, &sub) in subs.iter().enumerate() {
                if (has_next || position + 1 < subs.len()) && self.ends_with_blank(sub) {
                    return false;
                }
            }
        }
        true
    }
}

fn eat_quote_marker(cursor: &mut Cursor<'_>) -> bool {
    let indent = cursor.indent();
    if indent > 3 || cursor.rest_nonblank().as_bytes().first() != Some(&b'>') {
        return false;
    }
    cursor.advance_columns(indent);
    cursor.advance_bytes(1);
    if matches!(
        cursor.line.as_bytes().get(cursor.offset),
        Some(b' ' | b'\t')
    ) {
        cursor.advance_columns(1);
    }
    true
}

fn atx_content(after: &str) -> String {
    let trimmed = after.trim_matches([' ', '\t']);
    let without_hashes = trimmed.trim_end_matches('#');
    if without_hashes.is_empty() {
        return String::new();
    }
    if without_hashes.len() < trimmed.len() && without_hashes.ends_with([' ', '\t']) {
        return String::from(without_hashes.trim_end_matches([' ', '\t']));
    }
    String::from(trimmed)
}

fn is_thematic_break(rest: &str) -> bool {
    let first = rest.as_bytes()[0];
    if !matches!(first, b'-' | b'*' | b'_') {
        return false;
    }
    let mut count = 0;
    for byte in rest.bytes() {
        if byte == first {
            count += 1;
        } else if !matches!(byte, b' ' | b'\t') {
            return false;
        }
    }
    count >= 3
}

fn parse_marker(rest: &str) -> Option<(bool, u64, u8, usize)> {
    let bytes = rest.as_bytes();
    let first = bytes[0];
    if matches!(first, b'-' | b'+' | b'*') {
        return matches!(bytes.get(1), None | Some(b' ' | b'\t')).then_some((false, 0, first, 1));
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let delim = *bytes.get(digits)?;
    if !matches!(delim, b'.' | b')') || !matches!(bytes.get(digits + 1), None | Some(b' ' | b'\t'))
    {
        return None;
    }
    let start = rest[..digits].parse().ok()?;
    Some((true, start, delim, digits + 1))
}

fn task_marker(text: &str) -> Option<(bool, &str)> {
    let state = match text.as_bytes().get(..3)? {
        b"[ ]" => false,
        b"[x]" | b"[X]" => true,
        _ => return None,
    };
    let rest = &text[3..];
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some((state, rest.trim_start_matches([' ', '\t'])))
}

fn trim_blank_tail(text: &mut String) {
    loop {
        let Some(stripped) = text.strip_suffix('\n') else {
            return;
        };
        let line_start = stripped.rfind('\n').map_or(0, |at| at + 1);
        if stripped[line_start..]
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'))
        {
            text.truncate(line_start);
        } else {
            return;
        }
    }
}
