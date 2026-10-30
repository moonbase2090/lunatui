// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

use lunatui_core::{Rect, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{Block, BorderType, RenderCx, Widget};

const PANE_Z: u16 = 100;
const STATUS_Z: u16 = 1000;
const PANE_ROWS: u16 = 3;
const DEFAULT_LIMIT: usize = 4;

/// Identifies one toast for dismissal and hit testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToastId(
    /// Counter value, unique within a [`ToastStack`].
    pub u32,
);

/// What a toast is about, which picks its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToastKind {
    /// Neutral news, drawn in the accent color.
    Info,
    /// Something finished, drawn in the ok color.
    Success,
    /// Something needs attention, drawn in the warn color.
    Warning,
    /// Something failed, drawn in the fail color.
    Error,
}

impl ToastKind {
    pub(crate) fn token(self) -> Token {
        match self {
            Self::Info => Token::Accent,
            Self::Success => Token::Ok,
            Self::Warning => Token::Warn,
            Self::Error => Token::Fail,
        }
    }
}

/// Where a toast appears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToastScope {
    /// A boxed toast in the bottom-right corner of the pane with this id.
    Pane(u32),
    /// A one-row toast on the bottom rows of the whole area, right aligned.
    Status,
}

/// One message in a [`ToastStack`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    id: ToastId,
    text: String,
    kind: ToastKind,
    scope: ToastScope,
    expires: Option<Duration>,
}

impl Toast {
    /// The toast's id.
    #[must_use]
    pub fn id(&self) -> ToastId {
        self.id
    }

    /// The message.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The kind.
    #[must_use]
    pub fn kind(&self) -> ToastKind {
        self.kind
    }

    /// Where it appears.
    #[must_use]
    pub fn scope(&self) -> ToastScope {
        self.scope
    }

    /// The clock time at which it goes away, or `None` for a toast that stays until dismissed.
    #[must_use]
    pub fn expires(&self) -> Option<Duration> {
        self.expires
    }

    fn live(&self, now: Duration) -> bool {
        self.expires.is_none_or(|at| now < at)
    }
}

/// Where a toast is drawn this frame, for rendering and hit testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToastPlacement {
    /// The toast.
    pub id: ToastId,
    /// The cells it covers.
    pub rect: Rect,
    /// Layer for `HitMap::push`: newer toasts sit above older ones, and status toasts above pane
    /// toasts.
    pub z: u16,
}

/// A timed stack of toasts, driven by a clock the caller owns.
///
/// Times are [`Duration`]s since any fixed start, such as `lunatui_runtime::Clock::now`. Nothing
/// here reads the system clock, so a test that advances a virtual clock sees exactly when each
/// toast goes away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToastStack {
    toasts: Vec<Toast>,
    next: u32,
    limit: usize,
}

impl Default for ToastStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastStack {
    /// Starts empty, showing up to four toasts per pane and four status toasts.
    #[must_use]
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            next: 0,
            limit: DEFAULT_LIMIT,
        }
    }

    /// Most toasts shown at once per pane, and for the status row. Older ones wait their turn.
    #[must_use]
    pub fn max_visible(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Adds a toast at time `now` that lasts `ttl`, or until dismissed when `ttl` is `None`.
    pub fn push(
        &mut self,
        now: Duration,
        ttl: Option<Duration>,
        scope: ToastScope,
        kind: ToastKind,
        text: impl Into<String>,
    ) -> ToastId {
        let id = ToastId(self.next);
        self.next = self.next.wrapping_add(1);
        self.toasts.push(Toast {
            id,
            text: text.into(),
            kind,
            scope,
            expires: ttl.map(|ttl| now.saturating_add(ttl)),
        });
        id
    }

    /// Removes every toast whose time is up at `now` and returns their ids, oldest first.
    pub fn expire(&mut self, now: Duration) -> Vec<ToastId> {
        let mut gone = Vec::new();
        self.toasts.retain(|toast| {
            let keep = toast.live(now);
            if !keep {
                gone.push(toast.id);
            }
            keep
        });
        gone
    }

    /// Removes one toast. Returns whether it was there.
    pub fn dismiss(&mut self, id: ToastId) -> bool {
        let before = self.toasts.len();
        self.toasts.retain(|toast| toast.id != id);
        self.toasts.len() != before
    }

    /// Removes every toast.
    pub fn clear(&mut self) {
        self.toasts.clear();
    }

    /// Number of toasts held, shown or waiting.
    #[must_use]
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// Whether the stack holds no toasts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// The toast with this id.
    #[must_use]
    pub fn get(&self, id: ToastId) -> Option<&Toast> {
        self.toasts.iter().find(|toast| toast.id == id)
    }

    /// Where each shown toast sits at time `now`, oldest first.
    ///
    /// `area` is the whole screen, where status toasts go; `pane` gives the rectangle of a pane
    /// id, or `None` for a pane that is not on screen (its toasts wait). Within a scope the newest
    /// toast sits in the corner and older ones stack above it, so only the newest
    /// [`ToastStack::max_visible`] that fit are shown. Toasts past their time are skipped even if
    /// [`ToastStack::expire`] has not run yet.
    #[must_use]
    pub fn placements(
        &self,
        now: Duration,
        area: Rect,
        pane: impl Fn(u32) -> Option<Rect>,
    ) -> Vec<ToastPlacement> {
        let mut placed = Vec::new();
        let mut scopes: Vec<ToastScope> = Vec::new();
        for toast in self.toasts.iter().filter(|toast| toast.live(now)) {
            if !scopes.contains(&toast.scope) {
                scopes.push(toast.scope);
            }
        }
        for scope in scopes {
            let bounds = match scope {
                ToastScope::Pane(id) => pane(id),
                ToastScope::Status => Some(area),
            };
            let Some(bounds) = bounds else { continue };
            let members: Vec<&Toast> = self
                .toasts
                .iter()
                .filter(|toast| toast.scope == scope && toast.live(now))
                .collect();
            let (rows, base) = match scope {
                ToastScope::Pane(_) => (PANE_ROWS, PANE_Z),
                ToastScope::Status => (1, STATUS_Z),
            };
            let mut slots: Vec<ToastPlacement> = Vec::new();
            for (rank, toast) in members.iter().enumerate().rev().take(self.limit) {
                let depth = u16::try_from(slots.len()).unwrap_or(u16::MAX);
                let Some(rect) = slot(toast, bounds, rows, depth) else {
                    break;
                };
                let z = base.saturating_add(u16::try_from(rank).unwrap_or(u16::MAX));
                slots.push(ToastPlacement {
                    id: toast.id,
                    rect,
                    z,
                });
            }
            placed.extend(slots);
        }
        placed.sort_by_key(|placement| placement.id);
        placed
    }

    /// Draws the shown toasts at time `now`. See [`ToastStack::placements`].
    pub fn render(
        &self,
        now: Duration,
        area: Rect,
        pane: impl Fn(u32) -> Option<Rect>,
        cx: &mut RenderCx<'_>,
    ) {
        let mut placements = self.placements(now, area, pane);
        placements.sort_by_key(|placement| placement.z);
        for placement in placements {
            if let Some(toast) = self.get(placement.id) {
                draw(toast, placement.rect, cx);
            }
        }
    }
}

fn slot(toast: &Toast, bounds: Rect, rows: u16, depth: u16) -> Option<Rect> {
    let chrome: u16 = if rows == 1 { 2 } else { 4 };
    let wanted = u16::try_from(width(&toast.text))
        .unwrap_or(u16::MAX)
        .saturating_add(chrome);
    let wide = wanted.min(bounds.width);
    if wide < chrome || rows > bounds.height {
        return None;
    }
    let lift = depth.checked_mul(rows)?;
    let bottom = bounds.y + bounds.height;
    let y = bottom.checked_sub(rows)?.checked_sub(lift)?;
    if y < bounds.y {
        return None;
    }
    Some(Rect::new(bounds.x + bounds.width - wide, y, wide, rows))
}

fn draw(toast: &Toast, rect: Rect, cx: &mut RenderCx<'_>) {
    let accent = cx.token(toast.kind.token());
    let fill = Style::new()
        .fg(cx.token(Token::TextPrimary))
        .bg(cx.token(Token::Surface1));
    let blank = " ".repeat(usize::from(rect.width));
    for y in rect.y..rect.y + rect.height {
        cx.put_str(rect.x, y, &blank, fill);
    }
    if rect.height == 1 {
        let room = rect.width.saturating_sub(2);
        cx.put_str(
            rect.x + 1,
            rect.y,
            clip_display_prefix(&toast.text, room),
            fill.fg(accent),
        );
        return;
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(accent).bg(cx.token(Token::Surface1)))
        .style(fill)
        .render(rect, cx);
    let room = rect.width.saturating_sub(4);
    cx.put_str(
        rect.x + 2,
        rect.y + 1,
        clip_display_prefix(&toast.text, room),
        fill,
    );
}
