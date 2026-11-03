// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, Tree, TreeItem, TreeState, Widget};

fn items() -> Vec<TreeItem> {
    vec![
        TreeItem::node(
            "src",
            vec![
                TreeItem::leaf("main.rs"),
                TreeItem::node("widgets", vec![TreeItem::leaf("tree.rs")]),
            ],
        ),
        TreeItem::leaf("README.md").badge("12"),
    ]
}

fn state() -> TreeState {
    TreeState::new(items())
}

fn draw(state: &TreeState, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Tree::new(state).render(area, &mut cx);
    (grid, clusters)
}

fn rows(state: &TreeState, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(state, Rect::new(0, 0, width, height), width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn visible_labels(state: &TreeState) -> Vec<&str> {
    state.visible().iter().map(|row| row.label).collect()
}

#[test]
fn closed_nodes_hide_their_children_until_opened() {
    let mut tree = state();
    assert_eq!(visible_labels(&tree), ["src", "README.md"]);
    assert!(tree.open(&[0]));
    assert_eq!(
        visible_labels(&tree),
        ["src", "main.rs", "widgets", "README.md"]
    );
    assert!(tree.open(&[0, 1]));
    assert_eq!(
        visible_labels(&tree),
        ["src", "main.rs", "widgets", "tree.rs", "README.md"]
    );
    assert!(tree.close(&[0]));
    assert_eq!(visible_labels(&tree), ["src", "README.md"]);
    assert!(!tree.is_open(&[0]));
}

#[test]
fn visible_rows_carry_path_depth_and_open_state() {
    let mut tree = state();
    tree.expand_all();
    let visible = tree.visible();
    let paths: Vec<&[usize]> = visible.iter().map(|row| row.path.as_slice()).collect();
    assert_eq!(
        paths,
        [&[0][..], &[0, 0][..], &[0, 1][..], &[0, 1, 0][..], &[1][..]]
    );
    let depths: Vec<usize> = visible.iter().map(|row| row.depth).collect();
    assert_eq!(depths, [0, 1, 1, 2, 0]);
    assert!(visible[0].has_children && visible[0].open);
    assert!(!visible[1].has_children && !visible[1].open);
    tree.collapse_all();
    assert_eq!(visible_labels(&tree), ["src", "README.md"]);
    assert!(!tree.is_open(&[0, 1]));
}

#[test]
fn toggling_leaves_disabled_and_unknown_paths_changes_nothing() {
    let mut tree = TreeState::new(vec![
        TreeItem::leaf("plain"),
        TreeItem::node("locked", vec![TreeItem::leaf("inner")]).disabled(),
    ]);
    assert!(!tree.toggle(&[0]));
    assert!(!tree.toggle(&[1]));
    assert!(!tree.open(&[1]));
    assert!(!tree.toggle(&[2]));
    assert!(!tree.toggle(&[0, 0]));
    assert!(!tree.toggle(&[]));
    assert_eq!(visible_labels(&tree), ["plain", "locked"]);
}

#[test]
fn closing_a_node_moves_a_descendant_selection_to_that_node() {
    let mut tree = state();
    tree.expand_all();
    assert!(tree.select(&[0, 1, 0]));
    assert!(tree.close(&[0]));
    assert_eq!(tree.selected_path(), Some([0].as_slice()));
    assert_eq!(tree.selected().map(TreeItem::label), Some("src"));
}

#[test]
fn hidden_and_disabled_nodes_cannot_be_selected_until_revealed() {
    let mut tree = state();
    assert!(!tree.select(&[0, 1, 0]));
    assert_eq!(tree.selected_path(), None);
    assert!(tree.reveal(&[0, 1, 0]));
    assert_eq!(tree.selected_path(), Some([0, 1, 0].as_slice()));
    assert!(tree.is_open(&[0]) && tree.is_open(&[0, 1]));
    assert!(!tree.reveal(&[9]));
    let mut disabled = TreeState::new(vec![TreeItem::leaf("off").disabled()]);
    assert!(!disabled.select(&[0]));
    assert!(!disabled.reveal(&[0]));
    assert_eq!(disabled.selected_path(), None);
}

#[test]
fn up_and_down_walk_visible_rows_skipping_disabled_ones() {
    let mut tree = TreeState::new(vec![
        TreeItem::leaf("a"),
        TreeItem::leaf("b").disabled(),
        TreeItem::node("c", vec![TreeItem::leaf("c1")]),
    ]);
    assert!(tree.move_down());
    assert_eq!(tree.selected_path(), Some([0].as_slice()));
    assert!(tree.move_down());
    assert_eq!(tree.selected_path(), Some([2].as_slice()));
    assert!(!tree.move_down());
    assert!(tree.move_up());
    assert_eq!(tree.selected_path(), Some([0].as_slice()));
    assert!(!tree.move_up());
    let mut fresh = state();
    assert!(fresh.move_up());
    assert_eq!(fresh.selected_path(), Some([1].as_slice()));
    assert!(fresh.select_first());
    assert_eq!(fresh.selected_path(), Some([0].as_slice()));
    assert!(fresh.select_last());
    assert_eq!(fresh.selected_path(), Some([1].as_slice()));
}

#[test]
fn right_opens_then_steps_in_and_left_steps_out_then_closes() {
    let mut tree = state();
    assert!(tree.select(&[0]));
    assert!(tree.move_right());
    assert!(tree.is_open(&[0]));
    assert_eq!(tree.selected_path(), Some([0].as_slice()));
    assert!(tree.move_right());
    assert_eq!(tree.selected_path(), Some([0, 0].as_slice()));
    assert!(!tree.move_right());
    assert!(tree.move_left());
    assert_eq!(tree.selected_path(), Some([0].as_slice()));
    assert!(tree.move_left());
    assert!(!tree.is_open(&[0]));
    assert!(!tree.move_left());
    let mut none = state();
    assert!(!none.move_left());
    assert!(!none.move_right());
}

#[test]
fn rows_draw_carets_guides_and_a_right_aligned_badge() {
    let mut tree = state();
    assert_eq!(
        rows(&tree, 20, 3),
        [
            "▸ src               ",
            "  README.md       12",
            "                    "
        ]
    );
    tree.expand_all();
    assert_eq!(
        rows(&tree, 20, 5),
        [
            "▾ src               ",
            "│   main.rs         ",
            "│ ▾ widgets         ",
            "│     tree.rs       ",
            "  README.md       12",
        ]
    );
}

#[test]
fn long_labels_are_clipped_before_the_badge() {
    let tree = TreeState::new(vec![TreeItem::leaf("configuration").badge("42")]);
    assert_eq!(rows(&tree, 10, 1), ["  confi 42"]);
    assert_eq!(rows(&tree, 3, 1), ["  c"]);
}

#[test]
fn the_selected_row_is_accented_and_bold_and_disabled_rows_are_dim() {
    let mut tree = TreeState::new(vec![
        TreeItem::leaf("a"),
        TreeItem::leaf("b").disabled(),
        TreeItem::leaf("c"),
    ]);
    assert!(tree.select(&[2]));
    let (grid, _) = draw(&tree, Rect::new(0, 0, 8, 3), 8, 3);
    assert_eq!(style_at(&grid, 2, 0).fg, color(Token::TextPrimary));
    assert!(!style_at(&grid, 2, 0).attrs.bold());
    assert!(style_at(&grid, 2, 1).attrs.dim());
    assert_eq!(style_at(&grid, 2, 2).fg, color(Token::Accent));
    assert!(style_at(&grid, 2, 2).attrs.bold());
}

#[test]
fn rendering_clears_stale_glyphs_from_every_row_of_the_area() {
    let tree = state();
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(12, 4);
    let mut clusters = ClusterTable::new();
    grid.fill(Rect::new(0, 0, 12, 4), Style::new(), 'X', &mut clusters);
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        Tree::new(&tree).render(Rect::new(0, 0, 12, 4), &mut cx);
    }
    assert_eq!(grid.row_display(0, &clusters), "▸ src       ");
    assert_eq!(grid.row_display(2, &clusters), "            ");
    assert_eq!(grid.row_display(3, &clusters), "            ");
}

#[test]
fn the_view_scrolls_so_the_selection_stays_on_screen() {
    let mut tree = state();
    tree.expand_all();
    assert!(tree.select(&[1]));
    assert_eq!(
        rows(&tree, 20, 2),
        ["│     tree.rs       ", "  README.md       12"]
    );
    assert!(tree.select(&[0]));
    tree.set_offset(3);
    assert_eq!(rows(&tree, 20, 2)[0], "▾ src               ");
}

#[test]
fn row_areas_match_the_rendered_rows_for_hit_testing() {
    let mut tree = state();
    tree.open(&[0]);
    let areas = Tree::new(&tree).row_areas(Rect::new(2, 3, 20, 3));
    assert_eq!(
        areas,
        [
            (vec![0], Rect::new(2, 3, 20, 1)),
            (vec![0, 0], Rect::new(2, 4, 20, 1)),
            (vec![0, 1], Rect::new(2, 5, 20, 1)),
        ]
    );
    assert_eq!(Tree::new(&tree).row_areas(Rect::new(0, 0, 0, 3)), []);
}

#[test]
fn measure_counts_the_visible_rows() {
    let mut tree = state();
    assert_eq!(
        Tree::new(&tree).measure(Size::new(20, 12)),
        Size::new(20, 2)
    );
    tree.expand_all();
    assert_eq!(
        Tree::new(&tree).measure(Size::new(20, 12)),
        Size::new(20, 5)
    );
    assert_eq!(Tree::new(&tree).measure(Size::new(20, 3)), Size::new(20, 3));
    assert_eq!(Tree::new(&tree).measure(Size::new(0, 5)), Size::new(0, 0));
    let empty = TreeState::new(Vec::new());
    assert_eq!(Tree::new(&empty).measure(Size::new(9, 9)), Size::new(0, 0));
    assert_eq!(rows(&empty, 6, 2), ["      ", "      "]);
}

#[test]
fn an_explicit_style_and_indent_replace_the_defaults() {
    let mut tree = state();
    tree.open(&[0]);
    let style = Style::new().fg(Color::Rgb(1, 2, 3));
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 3);
    let mut clusters = ClusterTable::new();
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        Tree::new(&tree)
            .style(style)
            .indent(4)
            .render(Rect::new(0, 0, 20, 3), &mut cx);
    }
    assert_eq!(style_at(&grid, 2, 0), style);
    assert_eq!(grid.row_display(1, &clusters), "│     main.rs       ");
}
