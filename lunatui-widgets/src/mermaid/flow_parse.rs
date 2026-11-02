// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use super::canvas::Weight;
use super::label;
use super::{LIMIT, MermaidError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    TopDown,
    BottomUp,
    LeftRight,
    RightLeft,
}

impl Direction {
    pub(super) fn from_word(word: &str) -> Option<Self> {
        match word {
            "TD" | "TB" => Some(Self::TopDown),
            "BT" => Some(Self::BottomUp),
            "LR" => Some(Self::LeftRight),
            "RL" => Some(Self::RightLeft),
            _ => None,
        }
    }

    pub(super) const fn horizontal(self) -> bool {
        matches!(self, Self::LeftRight | Self::RightLeft)
    }

    pub(super) const fn flipped(self) -> bool {
        matches!(self, Self::BottomUp | Self::RightLeft)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    Rect,
    Round,
    Stadium,
    Diamond,
    Circle,
}

#[derive(Clone, Debug)]
pub(super) struct Node {
    pub(super) lines: Vec<String>,
    pub(super) shape: Shape,
}

#[derive(Clone, Debug)]
pub(super) struct Edge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) weight: Weight,
    pub(super) head: bool,
    pub(super) label: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct Flow {
    pub(super) direction: Direction,
    pub(super) nodes: Vec<Node>,
    pub(super) edges: Vec<Edge>,
}

struct Link {
    weight: Weight,
    head: bool,
    label: Option<String>,
}

struct Cursor {
    chars: Vec<char>,
    at: usize,
}

impl Cursor {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.at + offset).copied()
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.at += 1;
        }
    }

    fn starts_with(&self, text: &str) -> bool {
        text.chars()
            .enumerate()
            .all(|(offset, ch)| self.peek_at(offset) == Some(ch))
    }

    fn word(&mut self) -> String {
        let mut word = String::new();
        while let Some(ch) = self.peek().filter(|ch| ch.is_alphanumeric() || *ch == '_') {
            word.push(ch);
            self.at += 1;
        }
        word
    }
}

pub(super) struct Parser {
    flow: Flow,
    ids: HashMap<String, usize>,
    line: usize,
}

impl Parser {
    pub(super) fn new(direction: Direction) -> Self {
        Self {
            flow: Flow {
                direction,
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            ids: HashMap::new(),
            line: 0,
        }
    }

    pub(super) fn finish(self) -> Flow {
        self.flow
    }

    pub(super) fn statement(&mut self, line: usize, text: &str) -> Result<(), MermaidError> {
        self.line = line;
        let mut cur = Cursor {
            chars: text.chars().collect(),
            at: 0,
        };
        loop {
            cur.skip_space();
            match cur.peek() {
                None => return Ok(()),
                Some(';') => cur.at += 1,
                Some(_) => {
                    if self.keyword(&mut cur)? {
                        return Ok(());
                    }
                    self.chain(&mut cur)?;
                }
            }
        }
    }

    fn fail(&self, message: &str) -> MermaidError {
        MermaidError::at(self.line, message)
    }

    fn keyword(&self, cur: &mut Cursor) -> Result<bool, MermaidError> {
        let start = cur.at;
        let word = cur.word();
        let spaced = cur.peek().is_none_or(char::is_whitespace);
        cur.at = start;
        match word.as_str() {
            "style" | "classDef" | "class" | "linkStyle" | "click" if spaced => Ok(true),
            "subgraph" | "end" | "direction" => Err(self.fail("subgraph is not supported")),
            _ => Ok(false),
        }
    }

    fn chain(&mut self, cur: &mut Cursor) -> Result<(), MermaidError> {
        let mut previous = self.node(cur)?;
        loop {
            cur.skip_space();
            if cur.peek().is_none_or(|ch| ch == ';') {
                return Ok(());
            }
            let link = self.link(cur)?;
            let next = self.node(cur)?;
            if self.flow.edges.len() >= LIMIT {
                return Err(self.fail("too many links (at most 256)"));
            }
            self.flow.edges.push(Edge {
                from: previous,
                to: next,
                weight: link.weight,
                head: link.head,
                label: link.label,
            });
            previous = next;
        }
    }

    fn node(&mut self, cur: &mut Cursor) -> Result<usize, MermaidError> {
        cur.skip_space();
        let id = cur.word();
        if id.is_empty() {
            return Err(self.fail(match cur.peek() {
                Some('&') => "'&' is not supported",
                Some(_) => "expected a node id",
                None => "a link must end in a node",
            }));
        }
        let shape = self.shape(cur)?;
        if cur.starts_with(":::") {
            cur.at += 3;
            cur.word();
        }
        let known = self.ids.get(&id).copied();
        let index = if let Some(index) = known {
            index
        } else {
            if self.flow.nodes.len() >= LIMIT {
                return Err(self.fail("too many nodes (at most 256)"));
            }
            self.flow.nodes.push(Node {
                lines: vec![id.clone()],
                shape: Shape::Rect,
            });
            self.ids.insert(id, self.flow.nodes.len() - 1);
            self.flow.nodes.len() - 1
        };
        if let (Some((shape, lines)), Some(node)) = (shape, self.flow.nodes.get_mut(index)) {
            node.shape = shape;
            if !lines.is_empty() {
                node.lines = lines;
            }
        }
        Ok(index)
    }

    fn shape(&self, cur: &mut Cursor) -> Result<Option<(Shape, Vec<String>)>, MermaidError> {
        let unsupported = || self.fail("this node shape is not supported");
        let (shape, close) = match (cur.peek(), cur.peek_at(1), cur.peek_at(2)) {
            (Some('['), Some('[' | '(' | '/' | '\\'), _) | (Some('{'), Some('{'), _) => {
                return Err(unsupported());
            }
            (Some('('), Some('('), Some('(')) | (Some('>'), _, _) => return Err(unsupported()),
            (Some('['), _, _) => {
                cur.at += 1;
                (Shape::Rect, "]")
            }
            (Some('('), Some('('), _) => {
                cur.at += 2;
                (Shape::Circle, "))")
            }
            (Some('('), Some('['), _) => {
                cur.at += 2;
                (Shape::Stadium, "])")
            }
            (Some('('), _, _) => {
                cur.at += 1;
                (Shape::Round, ")")
            }
            (Some('{'), _, _) => {
                cur.at += 1;
                (Shape::Diamond, "}")
            }
            _ => return Ok(None),
        };
        let raw = self.delimited(cur, close)?;
        let lines = label::clean(&raw).map_err(|message| self.fail(message))?;
        Ok(Some((shape, lines)))
    }

    fn delimited(&self, cur: &mut Cursor, close: &str) -> Result<String, MermaidError> {
        cur.skip_space();
        let mut text = String::new();
        if cur.peek() == Some('"') {
            cur.at += 1;
            loop {
                match cur.peek() {
                    None => return Err(self.fail("unterminated quote")),
                    Some('"') => break,
                    Some(ch) => text.push(ch),
                }
                cur.at += 1;
            }
            cur.at += 1;
            cur.skip_space();
            if !cur.starts_with(close) {
                return Err(self.fail("expected the closing bracket after the quoted label"));
            }
        } else {
            while !cur.starts_with(close) {
                let Some(ch) = cur.peek() else {
                    return Err(self.fail("a node label is not closed"));
                };
                text.push(ch);
                cur.at += 1;
            }
        }
        cur.at += close.chars().count();
        Ok(text)
    }

    fn link(&self, cur: &mut Cursor) -> Result<Link, MermaidError> {
        cur.skip_space();
        let (end, weight, head, mut text) =
            if let Some((end, weight, head)) = complete(&cur.chars, cur.at) {
                (end, weight, head, None)
            } else {
                let (end, weight, head, text) = self.spaced_text(&cur.chars, cur.at)?;
                (end, weight, head, Some(text))
            };
        cur.at = end;
        cur.skip_space();
        if cur.peek() == Some('|') {
            if text.is_some() {
                return Err(self.fail("a link has two labels"));
            }
            cur.at += 1;
            text = Some(self.delimited(cur, "|")?);
        }
        let label = match text {
            Some(raw) => {
                let joined = label::single_line(&raw).map_err(|message| self.fail(message))?;
                (!joined.is_empty()).then_some(joined)
            }
            None => None,
        };
        Ok(Link {
            weight,
            head,
            label,
        })
    }

    fn spaced_text(
        &self,
        chars: &[char],
        at: usize,
    ) -> Result<(usize, Weight, bool, String), MermaidError> {
        let prefix = (chars.get(at), chars.get(at + 1), chars.get(at + 2));
        let start = at + 2;
        let found = match prefix {
            (Some('-'), Some('-'), third) if third != Some(&'-') => {
                Self::scan(chars, start, |index| complete(chars, index))
            }
            (Some('='), Some('='), third) if third != Some(&'=') => {
                Self::scan(chars, start, |index| complete(chars, index))
            }
            (Some('-'), Some('.'), _) => Self::scan(chars, start, |index| {
                if chars.get(index) == Some(&'.') && chars.get(index + 1) == Some(&'-') {
                    let head = chars.get(index + 2) == Some(&'>');
                    Some((index + 2 + usize::from(head), Weight::Dotted, head))
                } else {
                    None
                }
            }),
            _ => return Err(self.fail("expected a link such as -->")),
        };
        let Some((from, end, head)) = found else {
            return Err(self.fail("a link with text is not closed"));
        };
        let weight = match prefix.0 {
            Some('=') => Weight::Thick,
            Some(_) if prefix.1 == Some(&'.') => Weight::Dotted,
            _ => Weight::Solid,
        };
        let text: String = chars.get(start..from).unwrap_or_default().iter().collect();
        Ok((end, weight, head, text))
    }

    fn scan(
        chars: &[char],
        start: usize,
        finish: impl Fn(usize) -> Option<(usize, Weight, bool)>,
    ) -> Option<(usize, usize, bool)> {
        let mut index = start;
        while index < chars.len() {
            if let Some((end, _, head)) = finish(index) {
                return Some((index, end, head));
            }
            let mark = chars.get(index).copied();
            index += 1;
            if matches!(mark, Some('-' | '=')) {
                while chars.get(index) == mark.as_ref() {
                    index += 1;
                }
            }
        }
        None
    }
}

fn complete(chars: &[char], at: usize) -> Option<(usize, Weight, bool)> {
    let run = |mark: char, weight: Weight| {
        let mut end = at;
        while chars.get(end) == Some(&mark) {
            end += 1;
        }
        let count = end - at;
        if chars.get(end) == Some(&'>') && count >= 2 {
            Some((end + 1, weight, true))
        } else if count >= 3 {
            Some((end, weight, false))
        } else {
            None
        }
    };
    match (chars.get(at), chars.get(at + 1)) {
        (Some('-'), Some('.')) => {
            let mut end = at + 1;
            while chars.get(end) == Some(&'.') {
                end += 1;
            }
            if chars.get(end) != Some(&'-') {
                return None;
            }
            let head = chars.get(end + 1) == Some(&'>');
            Some((end + 1 + usize::from(head), Weight::Dotted, head))
        }
        (Some('-'), _) => run('-', Weight::Solid),
        (Some('='), _) => run('=', Weight::Thick),
        _ => None,
    }
}
