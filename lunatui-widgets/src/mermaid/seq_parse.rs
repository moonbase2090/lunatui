// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use super::label;
use super::{LIMIT, MermaidError};

pub(super) const MAX_DEPTH: usize = 8;
const MAX_OTHER_ITEMS: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum End {
    Head,
    Open,
    Cross,
    Async,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Place {
    Left,
    Right,
    Over,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrameKind {
    Loop,
    Alt,
    Opt,
}

impl FrameKind {
    pub(super) const fn word(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Alt => "alt",
            Self::Opt => "opt",
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Participant {
    pub(super) label: String,
    pub(super) actor: bool,
}

#[derive(Clone, Debug)]
pub(super) enum Item {
    Message {
        from: usize,
        to: usize,
        dotted: bool,
        end: End,
        text: String,
    },
    Note {
        place: Place,
        first: usize,
        second: usize,
        lines: Vec<String>,
    },
    Open {
        kind: FrameKind,
        text: String,
    },
    Else {
        text: String,
    },
    Close,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Sequence {
    pub(super) participants: Vec<Participant>,
    pub(super) items: Vec<Item>,
}

const ARROWS: [(&str, bool, End); 8] = [
    ("-->>", true, End::Head),
    ("--x", true, End::Cross),
    ("--)", true, End::Async),
    ("-->", true, End::Open),
    ("->>", false, End::Head),
    ("-x", false, End::Cross),
    ("-)", false, End::Async),
    ("->", false, End::Open),
];

struct Parser {
    sequence: Sequence,
    ids: HashMap<String, usize>,
    open: Vec<(FrameKind, usize)>,
    numbered: bool,
    count: u32,
    messages: usize,
    others: usize,
    line: usize,
}

pub(super) fn parse(lines: &[(usize, &str)]) -> Result<Sequence, MermaidError> {
    let mut parser = Parser {
        sequence: Sequence::default(),
        ids: HashMap::new(),
        open: Vec::new(),
        numbered: false,
        count: 0,
        messages: 0,
        others: 0,
        line: 0,
    };
    for &(line, text) in lines {
        parser.line = line;
        parser.statement(text)?;
    }
    if let Some(&(_, line)) = parser.open.last() {
        return Err(MermaidError::at(
            line,
            "this frame is never closed with end",
        ));
    }
    Ok(parser.sequence)
}

fn split_word(text: &str) -> (&str, &str) {
    match text.split_once(char::is_whitespace) {
        Some((word, rest)) => (word, rest.trim()),
        None => (text, ""),
    }
}

impl Parser {
    fn fail(&self, message: &str) -> MermaidError {
        MermaidError::at(self.line, message)
    }

    fn text(&self, raw: &str) -> Result<String, MermaidError> {
        label::single_line(raw).map_err(|message| self.fail(message))
    }

    fn statement(&mut self, text: &str) -> Result<(), MermaidError> {
        let (word, rest) = split_word(text);
        match word {
            "participant" => self.declare(rest, false),
            "actor" => self.declare(rest, true),
            "autonumber" if rest.is_empty() => {
                self.numbered = true;
                Ok(())
            }
            "Note" | "note" => self.note(rest),
            "loop" => self.open(FrameKind::Loop, rest),
            "alt" => self.open(FrameKind::Alt, rest),
            "opt" => self.open(FrameKind::Opt, rest),
            "else" => self.otherwise(rest),
            "end" if rest.is_empty() => self.close(),
            _ => self.message(text),
        }
    }

    fn participant(&mut self, id: &str) -> Result<usize, MermaidError> {
        if let Some(&index) = self.ids.get(id) {
            return Ok(index);
        }
        if self.sequence.participants.len() >= LIMIT {
            return Err(self.fail("too many participants (at most 256)"));
        }
        let label = self.text(id)?;
        self.sequence.participants.push(Participant {
            label,
            actor: false,
        });
        let index = self.sequence.participants.len() - 1;
        self.ids.insert(id.to_owned(), index);
        Ok(index)
    }

    fn declare(&mut self, rest: &str, actor: bool) -> Result<(), MermaidError> {
        let (id, tail) = split_word(rest);
        if id.is_empty() {
            return Err(self.fail("expected a participant name"));
        }
        let alias = match split_word(tail) {
            ("as", alias) if !alias.is_empty() => Some(self.text(alias)?),
            ("", _) => None,
            _ => return Err(self.fail("expected 'as' and a display name")),
        };
        let index = self.participant(id)?;
        if let Some(participant) = self.sequence.participants.get_mut(index) {
            participant.actor = actor;
            if let Some(alias) = alias.filter(|alias| !alias.is_empty()) {
                participant.label = alias;
            }
        }
        Ok(())
    }

    fn other(&mut self) -> Result<(), MermaidError> {
        self.others += 1;
        if self.others > MAX_OTHER_ITEMS {
            return Err(self.fail("too many notes and frames (at most 512)"));
        }
        Ok(())
    }

    fn note(&mut self, rest: &str) -> Result<(), MermaidError> {
        let (head, body) = rest
            .split_once(':')
            .ok_or_else(|| self.fail("a note needs ':' and its text"))?;
        let (place, names) = match split_word(head.trim()) {
            ("left", tail) => (Place::Left, tail.strip_prefix("of ")),
            ("right", tail) => (Place::Right, tail.strip_prefix("of ")),
            ("over", tail) => (Place::Over, Some(tail)),
            _ => return Err(self.fail("a note goes 'left of', 'right of' or 'over' a participant")),
        };
        let names = names.map(str::trim).filter(|names| !names.is_empty());
        let Some(names) = names else {
            return Err(self.fail("a note needs a participant"));
        };
        let mut ids = names.split(',').map(str::trim);
        let first = ids.next().unwrap_or_default();
        let second = ids.next();
        if ids.next().is_some() || first.is_empty() || second.is_some_and(str::is_empty) {
            return Err(self.fail("a note names one or two participants"));
        }
        if first.contains(char::is_whitespace) || second.is_some_and(|id| id.contains(' ')) {
            return Err(self.fail("a note names one or two participants"));
        }
        if place != Place::Over && second.is_some() {
            return Err(self.fail("only 'over' can name two participants"));
        }
        let first = self.participant(first)?;
        let second = match second {
            Some(id) => self.participant(id)?,
            None => first,
        };
        let lines = label::clean(body).map_err(|message| self.fail(message))?;
        self.other()?;
        self.sequence.items.push(Item::Note {
            place,
            first: first.min(second),
            second: first.max(second),
            lines,
        });
        Ok(())
    }

    fn open(&mut self, kind: FrameKind, rest: &str) -> Result<(), MermaidError> {
        if self.open.len() >= MAX_DEPTH {
            return Err(self.fail("frames are nested too deeply (at most 8)"));
        }
        let text = self.text(rest)?;
        self.other()?;
        self.open.push((kind, self.line));
        self.sequence.items.push(Item::Open { kind, text });
        Ok(())
    }

    fn otherwise(&mut self, rest: &str) -> Result<(), MermaidError> {
        if !matches!(self.open.last(), Some((FrameKind::Alt, _))) {
            return Err(self.fail("else belongs inside an alt frame"));
        }
        let text = self.text(rest)?;
        self.other()?;
        self.sequence.items.push(Item::Else { text });
        Ok(())
    }

    fn close(&mut self) -> Result<(), MermaidError> {
        if self.open.pop().is_none() {
            return Err(self.fail("end without a frame to close"));
        }
        self.other()?;
        self.sequence.items.push(Item::Close);
        Ok(())
    }

    fn message(&mut self, text: &str) -> Result<(), MermaidError> {
        let (head, body) = text.split_once(':').unwrap_or((text, ""));
        let found = head.char_indices().find_map(|(at, ch)| {
            if ch != '-' {
                return None;
            }
            let tail = head.get(at..)?;
            ARROWS
                .iter()
                .find(|(arrow, _, _)| tail.starts_with(arrow))
                .map(|&(arrow, dotted, end)| (at, arrow.len(), dotted, end))
        });
        let Some((at, length, dotted, end)) = found else {
            return Err(self.fail("unknown statement"));
        };
        let from = head.get(..at).unwrap_or_default().trim();
        let to = head.get(at + length..).unwrap_or_default().trim();
        if to.starts_with(['+', '-']) {
            return Err(self.fail("activation marks are not supported"));
        }
        let named = |id: &str| !id.is_empty() && !id.contains(char::is_whitespace);
        if !named(from) || !named(to) {
            return Err(self.fail("a message goes from one participant to another"));
        }
        if self.messages >= LIMIT {
            return Err(self.fail("too many messages (at most 256)"));
        }
        self.messages += 1;
        let text = self.text(body)?;
        let text = if self.numbered {
            self.count += 1;
            format!("{}. {text}", self.count).trim_end().to_owned()
        } else {
            text
        };
        let from = self.participant(from)?;
        let to = self.participant(to)?;
        self.sequence.items.push(Item::Message {
            from,
            to,
            dotted,
            end,
            text,
        });
        Ok(())
    }
}
