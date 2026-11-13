// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_runtime::{FocusManager, HitId, LayerEvent, LayerStack};
use lunatui_term::{Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseKind};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    AnchorSide, Paragraph, Popover, PopoverState, RenderCx, Widget, WidgetState,
};

const ZOOM: Rect = Rect::new(2, 1, 4, 1);
fn background() -> Style {
    Style::new().fg(Color::Rgb(200, 200, 200))
}

fn zoom(anchor: Rect) -> Popover<Paragraph> {
    Popover::new(Paragraph::new("Fit\nFill"), anchor)
        .title("Zoom")
        .size(Size::new(6, 2))
}

fn draw(popover: &Popover<Paragraph>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let dots = ".".repeat(usize::from(width));
    for y in 0..height {
        cx.put_str(0, y, &dots, background());
    }
    popover.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(popover: &Popover<Paragraph>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(popover, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn token(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

#[test]
fn opens_under_its_anchor_as_a_titled_panel() {
    let popover = zoom(ZOOM);
    assert_eq!(
        popover.area(Rect::new(0, 0, 20, 8)),
        Some(Rect::new(2, 2, 8, 4))
    );
    assert_eq!(
        rows(&popover, 20, 8),
        [
            "....................",
            "....................",
            "..╭Zoom──╮..........",
            "..│Fit   │..........",
            "..│Fill  │..........",
            "..╰──────╯..........",
            "....................",
            "....................",
        ]
    );
}

#[test]
fn leaves_the_rest_of_the_screen_as_it_was() {
    let (grid, _) = draw(&zoom(ZOOM), 20, 8);
    for (x, y) in [(0, 0), (12, 3), (2, 1), (10, 2), (5, 7)] {
        assert_eq!(style_at(&grid, x, y), background(), "cell {x},{y}");
    }
}

#[test]
fn takes_its_colors_from_theme_tokens_and_its_state() {
    let (grid, _) = draw(&zoom(ZOOM), 20, 8);
    assert_eq!(style_at(&grid, 2, 2).fg, token(Token::Accent));
    assert_eq!(style_at(&grid, 3, 3).bg, token(Token::Surface1));
    let (focused, _) = draw(&zoom(ZOOM).state(WidgetState::Focused), 20, 8);
    assert_eq!(style_at(&focused, 2, 2).fg, token(Token::FocusRing));
}

#[test]
fn flips_above_when_there_is_no_room_below() {
    let popover = zoom(Rect::new(2, 6, 4, 1));
    assert_eq!(
        popover.area(Rect::new(0, 0, 20, 8)),
        Some(Rect::new(2, 2, 8, 4))
    );
    assert_eq!(rows(&popover, 20, 8)[2], "..╭Zoom──╮..........");
}

#[test]
fn goes_beside_the_anchor_when_neither_row_has_room() {
    let area = Rect::new(0, 0, 20, 4);
    assert_eq!(zoom(ZOOM).area(area), Some(Rect::new(6, 0, 8, 4)));
    let left = zoom(Rect::new(14, 1, 4, 1));
    assert_eq!(left.area(area), Some(Rect::new(6, 0, 8, 4)));
}

#[test]
fn prefers_the_side_it_is_given() {
    let popover = zoom(Rect::new(2, 2, 4, 1)).side(AnchorSide::Right);
    assert_eq!(
        popover.area(Rect::new(0, 0, 20, 8)),
        Some(Rect::new(6, 2, 8, 4))
    );
}

#[test]
fn slides_left_to_stay_inside_the_area() {
    let popover = zoom(Rect::new(16, 1, 4, 1));
    assert_eq!(
        popover.area(Rect::new(0, 0, 20, 8)),
        Some(Rect::new(12, 2, 8, 4))
    );
}

#[test]
fn is_not_drawn_when_no_side_has_room() {
    let popover = zoom(Rect::new(1, 1, 18, 2));
    assert_eq!(popover.area(Rect::new(0, 0, 20, 4)), None);
    assert!(
        rows(&popover, 20, 4)
            .iter()
            .all(|row| row == "....................")
    );
    let wide = Popover::new(Paragraph::new("Fit"), ZOOM);
    assert_eq!(
        wide.area(Rect::new(0, 0, 20, 8)),
        Some(Rect::new(0, 2, 20, 3)),
        "content that measures to the full width spans it"
    );
}

#[test]
fn needs_a_cell_inside_its_border() {
    let corner = Rect::new(0, 0, 1, 1);
    assert_eq!(
        zoom(corner).area(Rect::new(0, 0, 2, 8)),
        None,
        "two columns"
    );
    assert_eq!(
        zoom(corner).area(Rect::new(0, 0, 3, 8)),
        Some(Rect::new(0, 1, 3, 4))
    );
    assert_eq!(zoom(corner).area(Rect::new(0, 0, 20, 2)), None, "two rows");
    assert_eq!(
        zoom(corner).area(Rect::new(0, 0, 20, 3)),
        Some(Rect::new(1, 0, 8, 3))
    );
}

#[test]
fn never_covers_its_anchor_and_stays_inside_the_area() {
    let area = Rect::new(0, 0, 20, 8);
    let mut drawn = 0;
    for side in [
        AnchorSide::Below,
        AnchorSide::Above,
        AnchorSide::Right,
        AnchorSide::Left,
    ] {
        for y in 0..8 {
            for x in 0..20 {
                for (width, height) in [(1, 1), (4, 1), (3, 2), (6, 3)] {
                    let anchor = Rect::new(x, y, width, height);
                    let Some(panel) = zoom(anchor).side(side).area(area) else {
                        continue;
                    };
                    drawn += 1;
                    assert!(!overlaps(panel, anchor), "{side:?} {anchor:?} -> {panel:?}");
                    assert!(panel.x + panel.width <= 20 && panel.y + panel.height <= 8);
                }
            }
        }
    }
    assert!(drawn > 1500, "only {drawn} placements fit");
}

const SCREEN: Rect = Rect::new(0, 0, 20, 8);
const ANCHOR_ID: HitId = HitId(1);
const SAVE_ID: HitId = HitId(2);
const SAVE: Rect = Rect::new(14, 7, 4, 1);

struct Screen {
    layers: LayerStack,
    focus: FocusManager,
    state: PopoverState,
    saved: u32,
}

impl Screen {
    fn new() -> Self {
        let mut screen = Self {
            layers: LayerStack::new(),
            focus: FocusManager::new(),
            state: PopoverState::new(),
            saved: 0,
        };
        screen.frame();
        screen
    }

    fn frame(&mut self) {
        let panel = self
            .state
            .is_open()
            .then(|| zoom(ZOOM).area(SCREEN))
            .flatten();
        self.state.place(&mut self.layers, panel);
        let base = vec![(ANCHOR_ID, ZOOM), (SAVE_ID, SAVE)];
        self.focus.sync(self.layers.focus_entries(base));
    }

    fn event(&mut self, event: &Event) {
        match self.layers.handle(event, &mut self.focus) {
            LayerEvent::Consumed | LayerEvent::Dismissed { deliver: false, .. } => {
                self.state.sync(&self.layers);
                self.frame();
                return;
            }
            LayerEvent::Dismissed { deliver: true, .. } => {
                self.state.sync(&self.layers);
            }
            LayerEvent::Pass => {}
        }
        if let Event::Mouse(mouse) = event {
            let at = (mouse.column, mouse.row);
            if contains(ZOOM, at) {
                self.state
                    .toggle(&mut self.layers, &mut self.focus, ZOOM, Some(ANCHOR_ID));
            } else if contains(SAVE, at) {
                self.saved += 1;
            }
        }
        self.frame();
    }

    fn press(&mut self, column: u16, row: u16) {
        self.event(&Event::Mouse(MouseEvent::new(
            MouseKind::Down(MouseButton::Left),
            column,
            row,
        )));
    }
}

fn contains(area: Rect, (column, row): (u16, u16)) -> bool {
    column >= area.x && column - area.x < area.width && row >= area.y && row - area.y < area.height
}

#[test]
fn a_press_on_the_anchor_opens_it_and_a_second_closes_it() {
    assert_eq!(PopoverState::new(), PopoverState::default());
    let mut screen = Screen::new();
    assert!(!screen.state.is_open());
    screen.press(3, 1);
    assert!(screen.state.is_open());
    assert_eq!(screen.layers.len(), 1);
    screen.press(5, 1);
    assert!(
        !screen.state.is_open(),
        "the anchor closes it rather than reopening it"
    );
    assert!(screen.layers.is_empty());
}

#[test]
fn a_press_inside_keeps_it_open_and_one_outside_closes_it_and_still_lands()
-> Result<(), &'static str> {
    let mut screen = Screen::new();
    screen.press(3, 1);
    let panel = zoom(ZOOM).area(SCREEN).ok_or("the popover fits")?;
    screen.press(panel.x + 1, panel.y + 1);
    assert!(screen.state.is_open());
    screen.press(15, 7);
    assert!(!screen.state.is_open());
    assert_eq!(
        screen.saved, 1,
        "the press that closed it also pressed Save"
    );
    Ok(())
}

#[test]
fn esc_closes_it_and_gives_focus_back_to_the_anchor() {
    let mut screen = Screen::new();
    screen.focus.focus(SAVE_ID);
    screen.press(3, 1);
    assert!(screen.state.is_open());
    screen.event(&Event::Key(KeyEvent::press(KeyCode::Esc)));
    assert!(!screen.state.is_open());
    assert!(screen.layers.is_empty());
    assert_eq!(screen.focus.focused(), Some(ANCHOR_ID));
}
