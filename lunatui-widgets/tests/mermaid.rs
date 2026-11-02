// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    MermaidDiagram, MermaidError, MermaidGlyphs, MermaidView, RenderCx, Widget, WidgetState,
};

type Test = Result<(), MermaidError>;

fn paint(view: &MermaidView<'_>, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    view.render(area, &mut cx);
    (grid, clusters)
}

fn rows(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..grid.height())
        .map(|y| grid.row_display(y, clusters).trim_end().to_owned())
        .collect()
}

fn natural(source: &str) -> Result<(Grid, ClusterTable), MermaidError> {
    let diagram = MermaidDiagram::parse(source)?;
    let size = diagram.size();
    Ok(paint(
        &MermaidView::new(&diagram),
        Rect::new(0, 0, size.width, size.height),
        size.width,
        size.height,
    ))
}

fn drawn(source: &str) -> Result<Vec<String>, MermaidError> {
    let (grid, clusters) = natural(source)?;
    Ok(rows(&grid, &clusters))
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn token(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn failure(source: &str) -> (usize, String) {
    match MermaidDiagram::parse(source) {
        Ok(_) => (0, String::from("parsed")),
        Err(error) => (error.line(), error.message().to_owned()),
    }
}

fn rejects(source: &str, line: usize, message: &str) {
    assert_eq!(failure(source), (line, message.to_owned()), "{source:?}");
}

#[test]
fn draws_each_node_shape() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A[Hi]")?,
        ["┌─────┐", "│ Hi  │", "└─────┘"]
    );
    assert_eq!(
        drawn("flowchart TD\n  A(Hi)")?,
        ["╭─────╮", "│ Hi  │", "╰─────╯"]
    );
    assert_eq!(
        drawn("flowchart TD\n  A([Hi])")?,
        ["╭─────╮", "( Hi  )", "╰─────╯"]
    );
    assert_eq!(
        drawn("flowchart TD\n  A{Hi}")?,
        [" ╱─────╲", "<  Hi   >", " ╲─────╱"]
    );
    assert_eq!(
        drawn("flowchart TD\n  A((Hi))")?,
        ["╭───────╮", "(( Hi  ))", "╰───────╯"]
    );
    Ok(())
}

#[test]
fn a_bare_id_is_a_rectangle_labeled_with_the_id() -> Test {
    assert_eq!(
        drawn("graph TD\n  Node")?,
        ["┌───────┐", "│ Node  │", "└───────┘"]
    );
    Ok(())
}

#[test]
fn quoted_labels_keep_brackets_and_commas() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  Q[\"a, [b]\"]")?,
        ["┌─────────┐", "│ a, [b]  │", "└─────────┘"]
    );
    Ok(())
}

#[test]
fn a_later_label_replaces_an_earlier_one() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A[One]\n  A[Two]")?,
        ["┌─────┐", "│ Two │", "└─────┘"]
    );
    assert_eq!(
        drawn("flowchart TD\n  A[One]\n  A")?,
        ["┌─────┐", "│ One │", "└─────┘"]
    );
    Ok(())
}

#[test]
fn line_breaks_in_a_label_stack_the_lines() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A[x<br/>yyy<BR>z]")?,
        ["┌─────┐", "│  x  │", "│ yyy │", "│  z  │", "└─────┘"]
    );
    Ok(())
}

#[test]
fn draws_each_edge_kind_left_to_right() -> Test {
    assert_eq!(
        drawn("flowchart LR\n  A --> B")?,
        ["┌───┐  ┌───┐", "│ A ├─▶│ B │", "└───┘  └───┘"]
    );
    assert_eq!(
        drawn("flowchart LR\n  A --- B")?,
        ["┌───┐  ┌───┐", "│ A ├──┤ B │", "└───┘  └───┘"]
    );
    assert_eq!(
        drawn("flowchart LR\n  A -.-> B")?,
        ["┌───┐  ┌───┐", "│ A ├┄▶│ B │", "└───┘  └───┘"]
    );
    assert_eq!(
        drawn("flowchart LR\n  A ==> B")?,
        ["┌───┐  ┌───┐", "│ A ├━▶│ B │", "└───┘  └───┘"]
    );
    Ok(())
}

#[test]
fn open_dotted_and_thick_edges_run_top_down() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A -.- B\n  B === C")?,
        [
            "┌───┐",
            "│ A │",
            "└─┬─┘",
            "  ┆",
            "  ┆",
            "┌─┴─┐",
            "│ B │",
            "└─┬─┘",
            "  ┃",
            "  ┃",
            "┌─┴─┐",
            "│ C │",
            "└───┘",
        ]
    );
    Ok(())
}

#[test]
fn edge_labels_sit_beside_the_line_in_both_syntaxes() -> Test {
    let top_down = [
        "┌───┐",
        "│ A │",
        "└─┬─┘",
        "  │",
        "  │ yes",
        "  ▼",
        "┌───┐",
        "│ B │",
        "└───┘",
    ];
    assert_eq!(drawn("flowchart TD\n  A -->|yes| B")?, top_down);
    assert_eq!(drawn("flowchart TD\n  A -- yes --> B")?, top_down);
    let left_right = [
        "┌───┐  yes  ┌───┐",
        "│ A ├──────▶│ B │",
        "└───┘       └───┘",
    ];
    assert_eq!(drawn("flowchart LR\n  A -- yes --> B")?, left_right);
    assert_eq!(drawn("flowchart LR\n  A -->|yes| B")?, left_right);
    assert_eq!(drawn("flowchart LR\n  A -->|yes|B")?, left_right);
    assert_eq!(drawn("flowchart LR\n  A--yes-->B")?, left_right);
    Ok(())
}

#[test]
fn dotted_and_thick_edges_take_their_labels_too() -> Test {
    assert_eq!(
        drawn("flowchart LR\n  A -. hi .-> B")?,
        ["┌───┐  hi  ┌───┐", "│ A ├┄┄┄┄┄▶│ B │", "└───┘      └───┘"]
    );
    assert_eq!(
        drawn("flowchart LR\n  A == hi ==> B")?,
        ["┌───┐  hi  ┌───┐", "│ A ├━━━━━▶│ B │", "└───┘      └───┘"]
    );
    Ok(())
}

#[test]
fn a_chain_declares_every_link_in_one_statement() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A --> B --> C")?,
        [
            "┌───┐",
            "│ A │",
            "└─┬─┘",
            "  │",
            "  ▼",
            "┌───┐",
            "│ B │",
            "└─┬─┘",
            "  │",
            "  ▼",
            "┌───┐",
            "│ C │",
            "└───┘",
        ]
    );
    Ok(())
}

#[test]
fn a_chain_can_mix_link_kinds_and_labels() -> Test {
    assert_eq!(
        drawn("flowchart LR\n  A -->|one| B -->|two| C")?,
        [
            "┌───┐  one  ┌───┐  two  ┌───┐",
            "│ A ├──────▶│ B ├──────▶│ C │",
            "└───┘       └───┘       └───┘",
        ]
    );
    Ok(())
}

#[test]
fn a_cycle_draws_the_back_edge_pointing_at_its_original_target() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A --> B\n  B --> A")?,
        [
            "┌───┐",
            "│ A │",
            "└─┬─┘",
            "  │▲",
            "  ▼│",
            "┌──┴┐",
            "│ B │",
            "└───┘"
        ]
    );
    Ok(())
}

#[test]
fn a_self_loop_marks_the_node() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A --> A")?,
        ["┌─────┐", "│ A ↺ │", "└─────┘"]
    );
    Ok(())
}

#[test]
fn all_five_directions() -> Test {
    let top_down = [
        "┌───┐",
        "│ A │",
        "└─┬─┘",
        "  │",
        "  ▼",
        "┌───┐",
        "│ B │",
        "└───┘",
    ];
    assert_eq!(drawn("flowchart TD\n  A --> B")?, top_down);
    assert_eq!(drawn("flowchart TB\n  A --> B")?, top_down);
    assert_eq!(drawn("graph TD\n  A --> B")?, top_down);
    assert_eq!(drawn("graph\n  A --> B")?, top_down);
    assert_eq!(
        drawn("flowchart BT\n  A --> B")?,
        [
            "┌───┐",
            "│ B │",
            "└───┘",
            "  ▲",
            "  │",
            "┌─┴─┐",
            "│ A │",
            "└───┘"
        ]
    );
    assert_eq!(
        drawn("flowchart LR\n  A --> B")?,
        ["┌───┐  ┌───┐", "│ A ├─▶│ B │", "└───┘  └───┘"]
    );
    assert_eq!(
        drawn("flowchart RL\n  A --> B")?,
        ["┌───┐  ┌───┐", "│ B │◀─┤ A │", "└───┘  └───┘"]
    );
    Ok(())
}

#[test]
fn bottom_up_labels_and_arrows_follow_the_flip() -> Test {
    assert_eq!(
        drawn("flowchart BT\n  A -->|up| B")?,
        [
            "┌───┐",
            "│ B │",
            "└───┘",
            "  ▲",
            "  │ up",
            "  │",
            "┌─┴─┐",
            "│ A │",
            "└───┘"
        ]
    );
    assert_eq!(
        drawn("flowchart RL\n  A -->|go| B")?,
        ["┌───┐  go  ┌───┐", "│ B │◀─────┤ A │", "└───┘      └───┘"]
    );
    Ok(())
}

#[test]
fn siblings_share_one_bus_under_their_parent() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A --> B\n  A --> C")?,
        [
            "    ┌───┐",
            "    │ A │",
            "    └─┬─┘",
            "      │",
            "  ╭───┴───╮",
            "  ▼       ▼",
            "┌───┐   ┌───┐",
            "│ B │   │ C │",
            "└───┘   └───┘",
        ]
    );
    Ok(())
}

#[test]
fn a_long_edge_runs_beside_the_nodes_it_skips() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A --> B --> C\n  A --> C")?,
        [
            "  ┌───┐",
            "  │ A │",
            "  └─┬─┘",
            "    │",
            "  ╭─┴─╮",
            "  ▼   │",
            "┌───┐ │",
            "│ B │ │",
            "└─┬─┘ │",
            "  │   │",
            "  ╰─┬─╯",
            "    ▼",
            "  ┌───┐",
            "  │ C │",
            "  └───┘",
        ]
    );
    Ok(())
}

#[test]
fn comments_style_lines_and_semicolons_are_skipped() -> Test {
    let source = "flowchart TD\n%% a comment\n  A --> B %% trailing\n  style A fill:#f00\n  \
                  classDef c fill:#fff;\n  class A c\n  linkStyle 0 stroke:red\n  \
                  click A call cb\n  A:::c --> B; B --> C";
    assert_eq!(
        drawn(source)?,
        drawn("flowchart TD\n  A --> B\n  A --> B\n  B --> C")?
    );
    Ok(())
}

#[test]
fn a_comment_marker_inside_quotes_is_text() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A[\"50%%\"]")?,
        ["┌───────┐", "│ 50%%  │", "└───────┘"]
    );
    Ok(())
}

#[test]
fn horizontal_flows_with_mixed_shapes_stay_aligned() -> Test {
    assert_eq!(
        drawn("flowchart LR\n  A([Round]) --> B{Pick} --> C((Out))\n  B -->|alt| D[Other]")?,
        [
            "╭───────╮                     ╭───────╮",
            "( Round )─╮            ╭─────▶(( Out ))",
            "╰───────╯ │  ╱──────╲  │      ╰───────╯",
            "          ╰▶<  Pick  >─┤",
            "             ╲──────╱  │ alt  ┌───────┐",
            "                       ╰─────▶│ Other │",
            "                              └───────┘",
        ]
    );
    Ok(())
}

#[test]
fn a_fan_in_joins_one_bus_above_the_target() -> Test {
    assert_eq!(
        drawn(
            "flowchart TD\n  D{Go?} -->|yes| Y[Yes]\n  D -->|no| N[No]\n  Y --> E[End]\n  N --> E"
        )?,
        [
            "     ╱─────╲",
            "    <  Go?  >",
            "     ╲──┬──╱",
            "        │",
            "   ╭────┴────╮",
            "   │ yes     │ no",
            "   ▼         ▼",
            "┌─────┐   ┌─────┐",
            "│ Yes │   │ No  │",
            "└──┬──┘   └──┬──┘",
            "   │         │",
            "   ╰────┬────╯",
            "        ▼",
            "     ┌─────┐",
            "     │ End │",
            "     └─────┘",
        ]
    );
    Ok(())
}

#[test]
fn wide_characters_take_two_columns() -> Test {
    let (grid, clusters) = natural("flowchart TD\n  A[日本]")?;
    assert_eq!(
        rows(&grid, &clusters),
        ["┌───────┐", "│ 日本  │", "└───────┘"]
    );
    assert_eq!(grid.width(), 9);
    Ok(())
}

#[test]
fn sequence_messages_in_all_eight_arrow_styles() -> Test {
    let source = "sequenceDiagram\n  A->>B: solid head\n  A-->>B: dotted head\n  \
                  A->B: solid open\n  A-->B: dotted open\n  A-xB: solid cross\n  \
                  A--xB: dotted cross\n  A-)B: solid async\n  A--)B: dotted async";
    assert_eq!(
        drawn(source)?,
        [
            "┌───┐           ┌───┐",
            "│ A │           │ B │",
            "└─┬─┘           └─┬─┘",
            "  ┆               ┆",
            "  ┆  solid head   ┆",
            "  ├──────────────▶┆",
            "  ┆               ┆",
            "  ┆  dotted head  ┆",
            "  ├┄┄┄┄┄┄┄┄┄┄┄┄┄┄▶┆",
            "  ┆               ┆",
            "  ┆  solid open   ┆",
            "  ├───────────────┤",
            "  ┆               ┆",
            "  ┆  dotted open  ┆",
            "  ├┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┤",
            "  ┆               ┆",
            "  ┆  solid cross  ┆",
            "  ├──────────────✕┆",
            "  ┆               ┆",
            "  ┆ dotted cross  ┆",
            "  ├┄┄┄┄┄┄┄┄┄┄┄┄┄┄✕┆",
            "  ┆               ┆",
            "  ┆  solid async  ┆",
            "  ├──────────────⟩┆",
            "  ┆               ┆",
            "  ┆ dotted async  ┆",
            "  ├┄┄┄┄┄┄┄┄┄┄┄┄┄┄⟩┆",
            "  ┆               ┆",
            "┌─┴─┐           ┌─┴─┐",
            "│ A │           │ B │",
            "└───┘           └───┘",
        ]
    );
    Ok(())
}

#[test]
fn replies_point_back_and_self_messages_loop() -> Test {
    assert_eq!(
        drawn("sequenceDiagram\n  participant A\n  participant B\n  B->>A: left\n  A->>A: me")?,
        [
            "┌───┐   ┌───┐",
            "│ A │   │ B │",
            "└─┬─┘   └─┬─┘",
            "  ┆       ┆",
            "  ┆ left  ┆",
            "  ┆◀──────┤",
            "  ┆       ┆",
            "  ├──╮ me ┆",
            "  ┆◀─╯    ┆",
            "  ┆       ┆",
            "┌─┴─┐   ┌─┴─┐",
            "│ A │   │ B │",
            "└───┘   └───┘",
        ]
    );
    Ok(())
}

#[test]
fn a_message_crosses_the_lifeline_between_its_ends() -> Test {
    assert_eq!(
        drawn("sequenceDiagram\n  participant A\n  participant B\n  participant C\n  A->>C: skip")?,
        [
            "┌───┐  ┌───┐  ┌───┐",
            "│ A │  │ B │  │ C │",
            "└─┬─┘  └─┬─┘  └─┬─┘",
            "  ┆      ┆      ┆",
            "  ┆    skip     ┆",
            "  ├──────┼─────▶┆",
            "  ┆      ┆      ┆",
            "┌─┴─┐  ┌─┴─┐  ┌─┴─┐",
            "│ A │  │ B │  │ C │",
            "└───┘  └───┘  └───┘",
        ]
    );
    Ok(())
}

#[test]
fn actors_get_rounded_boxes_and_aliases_replace_the_name() -> Test {
    assert_eq!(
        drawn("sequenceDiagram\n  participant W as Web\n  actor U as User\n  U->>W: go")?,
        [
            "┌─────┐  ╭───────╮",
            "│ Web │  │ User  │",
            "└──┬──┘  ╰───┬───╯",
            "   ┆         ┆",
            "   ┆   go    ┆",
            "   ┆◀────────┤",
            "   ┆         ┆",
            "┌──┴──┐  ╭───┴───╮",
            "│ Web │  │ User  │",
            "└─────┘  ╰───────╯",
        ]
    );
    Ok(())
}

#[test]
fn autonumber_prefixes_each_message() -> Test {
    assert_eq!(
        drawn("sequenceDiagram\n  autonumber\n  A->>B: first\n  B->>A: second")?,
        [
            "┌───┐        ┌───┐",
            "│ A │        │ B │",
            "└─┬─┘        └─┬─┘",
            "  ┆            ┆",
            "  ┆  1. first  ┆",
            "  ├───────────▶┆",
            "  ┆            ┆",
            "  ┆ 2. second  ┆",
            "  ┆◀───────────┤",
            "  ┆            ┆",
            "┌─┴─┐        ┌─┴─┐",
            "│ A │        │ B │",
            "└───┘        └───┘",
        ]
    );
    Ok(())
}

#[test]
fn autonumber_only_numbers_messages_after_it() -> Test {
    let rows = drawn("sequenceDiagram\n  A->>B: before\n  autonumber\n  B->>A: after")?;
    assert!(rows.iter().any(|row| row.contains(" before ")));
    assert!(rows.iter().any(|row| row.contains(" 1. after ")));
    Ok(())
}

#[test]
fn notes_sit_left_right_and_over_participants() -> Test {
    let source = "sequenceDiagram\n  A->>B: hi\n  Note left of A: L\n  Note right of B: R\n  \
                  Note over A: one\n  Note over A,B: two";
    assert_eq!(
        drawn(source)?,
        [
            "    ┌───┐  ┌───┐",
            "    │ A │  │ B │",
            "    └─┬─┘  └─┬─┘",
            "      ┆      ┆",
            "      ┆  hi  ┆",
            "      ├─────▶┆",
            "      ┆      ┆",
            "╭───╮ ┆      ┆",
            "│ L │ ┆      ┆",
            "╰───╯ ┆      ┆",
            "      ┆      ┆",
            "      ┆      ┆ ╭───╮",
            "      ┆      ┆ │ R │",
            "      ┆      ┆ ╰───╯",
            "      ┆      ┆",
            "   ╭─────╮   ┆",
            "   │ one │   ┆",
            "   ╰─────╯   ┆",
            "      ┆      ┆",
            "     ╭────────╮",
            "     │  two   │",
            "     ╰────────╯",
            "      ┆      ┆",
            "    ┌─┴─┐  ┌─┴─┐",
            "    │ A │  │ B │",
            "    └───┘  └───┘",
        ]
    );
    Ok(())
}

#[test]
fn a_multi_line_note_stacks_its_lines() -> Test {
    let rows = drawn("sequenceDiagram\n  A->>B: hi\n  Note over A,B: top<br/>bottom")?;
    assert!(
        rows.iter().any(|row| row.contains("│  top   │")),
        "{rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("│ bottom │")),
        "{rows:?}"
    );
    Ok(())
}

#[test]
fn frames_nest_with_labeled_borders_and_an_else_divider() -> Test {
    let source = "sequenceDiagram\n  A->>B: hi\n  opt maybe\n    B->>A: yes\n  end\n  \
                  loop forever\n    alt a\n      A->>B: x\n    else b\n      B->>A: y\n    \
                  end\n  end";
    assert_eq!(
        drawn(source)?,
        [
            "   ┌───┐  ┌───┐",
            "   │ A │  │ B │",
            "   └─┬─┘  └─┬─┘",
            "     ┆      ┆",
            "     ┆  hi  ┆",
            "     ├─────▶┆",
            "     ┆      ┆",
            "╭─ opt [maybe] ────╮",
            "│    ┆      ┆      │",
            "│    ┆ yes  ┆      │",
            "│    ┆◀─────┤      │",
            "│    ┆      ┆      │",
            "╰────┼──────┼──────╯",
            "     ┆      ┆",
            "╭─ loop [forever] ─╮",
            "│    ┆      ┆      │",
            "│╭─ alt [a] ┼─────╮│",
            "││   ┆      ┆     ││",
            "││   ┆  x   ┆     ││",
            "││   ├─────▶┆     ││",
            "││   ┆      ┆     ││",
            "│├┄ else [b] ┄┄┄┄┄┤│",
            "││   ┆      ┆     ││",
            "││   ┆  y   ┆     ││",
            "││   ┆◀─────┤     ││",
            "││   ┆      ┆     ││",
            "│╰───┼──────┼─────╯│",
            "│    ┆      ┆      │",
            "╰────┼──────┼──────╯",
            "     ┆      ┆",
            "   ┌─┴─┐  ┌─┴─┐",
            "   │ A │  │ B │",
            "   └───┘  └───┘",
        ]
    );
    Ok(())
}

#[test]
fn an_unlabeled_frame_shows_only_its_keyword() -> Test {
    let rows = drawn("sequenceDiagram\n  A->>B: hi\n  loop\n    B->>A: again\n  end")?;
    assert!(rows.iter().any(|row| row.starts_with("╭─ loop ─")));
    Ok(())
}

#[test]
fn a_frame_wider_than_the_participants_widens_the_diagram() -> Test {
    let rows = drawn(
        "sequenceDiagram\n  A->>B: hi\n  loop a very long condition for this frame here\n  end",
    )?;
    let frame = rows
        .iter()
        .find(|row| row.contains("a very long condition"))
        .map(|row| row.chars().count());
    assert_eq!(frame, Some(54));
    Ok(())
}

#[test]
fn a_wide_note_pushes_neighbours_apart() -> Test {
    let narrow = MermaidDiagram::parse("sequenceDiagram\n  A->>B: hi")?.size();
    let wide = MermaidDiagram::parse(
        "sequenceDiagram\n  A->>B: hi\n  Note over A,B: a note that is quite wide indeed",
    )?
    .size();
    assert_eq!(narrow.width, 12);
    assert_eq!(wide.width, 38);
    Ok(())
}

#[test]
fn a_long_message_label_widens_the_gap() -> Test {
    let size =
        MermaidDiagram::parse("sequenceDiagram\n  A->>B: a label of thirty characters!!")?.size();
    assert_eq!(size.width, 39);
    Ok(())
}

#[test]
fn rejects_malformed_flowcharts_with_a_line_number() {
    rejects("", 1, "the diagram is empty");
    rejects("  \n %% only a comment\n", 1, "the diagram is empty");
    rejects(
        "pie\n  A",
        1,
        "expected flowchart, graph or sequenceDiagram",
    );
    rejects(
        "\n\nclassDiagram",
        3,
        "expected flowchart, graph or sequenceDiagram",
    );
    rejects(
        "flowchart XY\n  A",
        1,
        "the direction is TD, TB, BT, LR or RL",
    );
    rejects(
        "flowchart TD A\n  A",
        1,
        "put statements on the lines below",
    );
    rejects(
        "flowchart TD;x\n  A",
        1,
        "the direction is TD, TB, BT, LR or RL",
    );
    rejects("flowchart TD;\n  A -->", 2, "a link must end in a node");
    rejects(
        "flowchart TD\n  subgraph one",
        2,
        "subgraph is not supported",
    );
    rejects(
        "flowchart TD\n  A --> B\n  end",
        3,
        "subgraph is not supported",
    );
    rejects(
        "flowchart TD\n  direction LR",
        2,
        "subgraph is not supported",
    );
    rejects("flowchart TD\n\n  A[open", 3, "a node label is not closed");
    rejects("flowchart TD\n  A(open", 2, "a node label is not closed");
    rejects("flowchart TD\n  A{open", 2, "a node label is not closed");
    rejects("flowchart TD\n  A((open)", 2, "a node label is not closed");
    rejects("flowchart TD\n  A([open)", 2, "a node label is not closed");
    rejects("flowchart TD\n  A[\"open", 2, "unterminated quote");
    rejects(
        "flowchart TD\n  A[\"x\" y]",
        2,
        "expected the closing bracket after the quoted label",
    );
    rejects(
        "flowchart TD\n  A[[sub]]",
        2,
        "this node shape is not supported",
    );
    rejects(
        "flowchart TD\n  A[(db)]",
        2,
        "this node shape is not supported",
    );
    rejects(
        "flowchart TD\n  A[/par/]",
        2,
        "this node shape is not supported",
    );
    rejects(
        "flowchart TD\n  A[\\par\\]",
        2,
        "this node shape is not supported",
    );
    rejects(
        "flowchart TD\n  A{{hex}}",
        2,
        "this node shape is not supported",
    );
    rejects(
        "flowchart TD\n  A(((two)))",
        2,
        "this node shape is not supported",
    );
    rejects(
        "flowchart TD\n  A>flag]",
        2,
        "this node shape is not supported",
    );
    rejects("flowchart TD\n  A -->", 2, "a link must end in a node");
    rejects("flowchart TD\n  --> B", 2, "expected a node id");
    rejects("flowchart TD\n  A --> ?", 2, "expected a node id");
    rejects(
        "flowchart TD\n  A -- x",
        2,
        "a link with text is not closed",
    );
    rejects(
        "flowchart TD\n  A -. x",
        2,
        "a link with text is not closed",
    );
    rejects(
        "flowchart TD\n  A == x",
        2,
        "a link with text is not closed",
    );
    rejects(
        "flowchart TD\n  A --o B",
        2,
        "circle and cross link ends are not supported",
    );
    rejects(
        "flowchart TD\n  A --x B",
        2,
        "circle and cross link ends are not supported",
    );
    rejects(
        "flowchart TD\n  A--o",
        2,
        "circle and cross link ends are not supported",
    );
    rejects("flowchart TD\n  A - B", 2, "expected a link such as -->");
    rejects("flowchart TD\n  A & B --> C", 2, "'&' is not supported");
    rejects("flowchart TD\n  A --> B & C", 2, "'&' is not supported");
    rejects(
        "flowchart TD\n  A -- a --> |b| B",
        2,
        "a link has two labels",
    );
    rejects("flowchart TD\n  A -->|b B", 2, "a node label is not closed");
}

#[test]
fn rejects_labels_that_are_too_wide_or_too_tall() {
    let wide = "x".repeat(101);
    rejects(
        &format!("flowchart TD\n  A[{wide}]"),
        2,
        "label is too wide",
    );
    let tall = "<br/>y".repeat(9);
    rejects(
        &format!("flowchart TD\n  A[x{tall}]"),
        2,
        "label has too many lines",
    );
    rejects(
        &format!("flowchart TD\n  A -->|{wide}| B"),
        2,
        "label is too wide",
    );
    let fits = "x".repeat(100);
    assert!(MermaidDiagram::parse(&format!("flowchart TD\n  A[{fits}]")).is_ok());
    let lines = "<br/>y".repeat(7);
    assert!(MermaidDiagram::parse(&format!("flowchart TD\n  A[x{lines}]")).is_ok());
}

#[test]
fn rejects_sequence_diagrams_with_a_line_number() {
    rejects(
        "sequenceDiagram foo",
        1,
        "expected flowchart, graph or sequenceDiagram",
    );
    rejects("sequenceDiagram\n  hello world", 2, "unknown statement");
    rejects("sequenceDiagram\n  autonumber 5", 2, "unknown statement");
    rejects(
        "sequenceDiagram\n  participant",
        2,
        "expected a participant name",
    );
    rejects(
        "sequenceDiagram\n  participant A as",
        2,
        "expected 'as' and a display name",
    );
    rejects(
        "sequenceDiagram\n  participant A like B",
        2,
        "expected 'as' and a display name",
    );
    rejects(
        "sequenceDiagram\n  A->>+B: x",
        2,
        "activation marks are not supported",
    );
    rejects(
        "sequenceDiagram\n  A->>: x",
        2,
        "a message goes from one participant to another",
    );
    rejects(
        "sequenceDiagram\n  ->>B: x",
        2,
        "a message goes from one participant to another",
    );
    rejects(
        "sequenceDiagram\n  big name->>B: x",
        2,
        "a message goes from one participant to another",
    );
    rejects(
        "sequenceDiagram\n  Note over A",
        2,
        "a note needs ':' and its text",
    );
    rejects(
        "sequenceDiagram\n  Note sideways of A: x",
        2,
        "a note goes 'left of', 'right of' or 'over' a participant",
    );
    rejects(
        "sequenceDiagram\n  Note over : x",
        2,
        "a note needs a participant",
    );
    rejects(
        "sequenceDiagram\n  Note left of : x",
        2,
        "a note needs a participant",
    );
    rejects(
        "sequenceDiagram\n  Note over A,B,C: x",
        2,
        "a note names one or two participants",
    );
    rejects(
        "sequenceDiagram\n  Note over A,: x",
        2,
        "a note names one or two participants",
    );
    rejects(
        "sequenceDiagram\n  Note over A B: x",
        2,
        "a note names one or two participants",
    );
    rejects(
        "sequenceDiagram\n  Note left of A,B: x",
        2,
        "only 'over' can name two participants",
    );
    rejects(
        "sequenceDiagram\n  opt x\n  else y\n  end",
        3,
        "else belongs inside an alt frame",
    );
    rejects(
        "sequenceDiagram\n  else y",
        2,
        "else belongs inside an alt frame",
    );
    rejects("sequenceDiagram\n  end", 2, "end without a frame to close");
    rejects(
        "sequenceDiagram\n  A->>B: x\n  loop y\n  A->>B: z",
        3,
        "this frame is never closed with end",
    );
    rejects(
        "sequenceDiagram\n  alt a\n    loop b\n  end",
        2,
        "this frame is never closed with end",
    );
    rejects("sequenceDiagram\n  par x\n  end", 2, "unknown statement");
}

#[test]
fn sequence_input_that_is_allowed_parses() {
    let ok = |source: &str| assert!(MermaidDiagram::parse(source).is_ok(), "{source:?}");
    ok("sequenceDiagram");
    ok("sequenceDiagram\n  participant A as Alice Smith\n  A->>A: hm");
    ok("sequenceDiagram\n  A->>B");
    ok("sequenceDiagram\n  A->>B: a: b: c");
    ok("sequenceDiagram\n  My-Service->>B: dashes in a name");
    ok("sequenceDiagram\n  alt x\n  else y\n  else z\n  end");
    ok("sequenceDiagram\n  note over A: lowercase");
    ok("sequenceDiagram\n  %% comment\n  A-->>B: x %% trailing");
}

#[test]
fn caps_the_size_of_the_source() {
    let big = format!("flowchart TD\n  A[{}]", "x".repeat(70_000));
    rejects(&big, 1, "the diagram is larger than 64 KiB");
    let edge = format!("flowchart TD\n%%{}\n  A --> B", "x".repeat(65_000));
    assert!(MermaidDiagram::parse(&edge).is_ok());
    let over = format!("flowchart TD\n%%{}\n  A --> B", "x".repeat(65_536));
    rejects(&over, 1, "the diagram is larger than 64 KiB");
}

fn chain(count: usize) -> String {
    let mut source = String::from("flowchart LR\n");
    for index in 0..count {
        source.push_str(&format!("  N{index} --> N{}\n", index + 1));
    }
    source
}

#[test]
fn caps_nodes_and_links_at_256() -> Test {
    assert!(MermaidDiagram::parse(&chain(255)).is_ok());
    rejects(&chain(256), 257, "too many nodes (at most 256)");
    let mut links = String::from("flowchart TD\n");
    for _ in 0..256 {
        links.push_str("  A --> B\n");
    }
    assert!(MermaidDiagram::parse(&links).is_ok());
    links.push_str("  A --> B\n");
    rejects(&links, 258, "too many links (at most 256)");
    let mut nodes = String::from("flowchart TD\n");
    for index in 0..256 {
        nodes.push_str(&format!("  N{index}\n"));
    }
    assert!(MermaidDiagram::parse(&nodes).is_ok());
    nodes.push_str("  Extra\n");
    rejects(&nodes, 258, "too many nodes (at most 256)");
    Ok(())
}

#[test]
fn caps_participants_messages_notes_and_frame_depth() {
    let mut people = String::from("sequenceDiagram\n");
    for index in 0..256 {
        people.push_str(&format!("  participant P{index}\n"));
    }
    assert!(MermaidDiagram::parse(&people).is_ok());
    people.push_str("  participant Extra\n");
    rejects(&people, 258, "too many participants (at most 256)");

    let mut messages = String::from("sequenceDiagram\n");
    for _ in 0..256 {
        messages.push_str("  A->>B: x\n");
    }
    assert!(MermaidDiagram::parse(&messages).is_ok());
    messages.push_str("  A->>B: x\n");
    rejects(&messages, 258, "too many messages (at most 256)");

    let mut notes = String::from("sequenceDiagram\n");
    for _ in 0..512 {
        notes.push_str("  Note over A: x\n");
    }
    assert!(MermaidDiagram::parse(&notes).is_ok());
    notes.push_str("  Note over A: x\n");
    rejects(&notes, 514, "too many notes and frames (at most 512)");

    let mut depth = String::from("sequenceDiagram\n");
    for _ in 0..8 {
        depth.push_str("  loop x\n");
    }
    for _ in 0..8 {
        depth.push_str("  end\n");
    }
    assert!(MermaidDiagram::parse(&depth).is_ok());
    let deeper = depth.replacen("  loop x\n", "  loop x\n  loop x\n", 1);
    rejects(&deeper, 10, "frames are nested too deeply (at most 8)");
}

#[test]
fn refuses_a_diagram_that_would_draw_past_a_million_cells() {
    let label = "w".repeat(99);
    let mut source = String::from("sequenceDiagram\n");
    for index in 0..256 {
        source.push_str(&format!("  participant P{index} as {label}\n"));
    }
    for _ in 0..255 {
        source.push_str("  P0->>P1: x\n");
    }
    rejects(&source, 1, "the diagram is too large to draw");
}

#[test]
fn size_reports_the_whole_diagram() -> Test {
    assert_eq!(
        MermaidDiagram::parse("flowchart TD\n  A --> B")?.size(),
        Size::new(5, 8)
    );
    assert_eq!(
        MermaidDiagram::parse("flowchart LR\n  A --> B")?.size(),
        Size::new(12, 3)
    );
    assert_eq!(
        MermaidDiagram::parse("flowchart TD")?.size(),
        Size::new(0, 0)
    );
    assert_eq!(
        MermaidDiagram::parse("sequenceDiagram")?.size(),
        Size::new(0, 0)
    );
    Ok(())
}

#[test]
fn measure_is_the_natural_size_clamped_to_the_space() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A --> B")?;
    let view = MermaidView::new(&diagram);
    assert_eq!(view.measure(Size::new(80, 24)), Size::new(5, 8));
    assert_eq!(view.measure(Size::new(3, 100)), Size::new(3, 8));
    assert_eq!(view.measure(Size::new(100, 2)), Size::new(5, 2));
    Ok(())
}

#[test]
fn clips_to_a_smaller_area() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A --> B")?;
    let (grid, clusters) = paint(&MermaidView::new(&diagram), Rect::new(0, 0, 5, 4), 5, 4);
    assert_eq!(rows(&grid, &clusters), ["┌───┐", "│ A │", "└─┬─┘", "  │"]);
    let (grid, clusters) = paint(&MermaidView::new(&diagram), Rect::new(1, 1, 3, 2), 6, 5);
    assert_eq!(rows(&grid, &clusters), ["", " ┌──", " │ A", "", ""]);
    Ok(())
}

#[test]
fn scrolls_by_columns_and_rows() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A --> B")?;
    let view = MermaidView::new(&diagram).scroll(0, 3);
    let (grid, clusters) = paint(&view, Rect::new(0, 0, 5, 4), 5, 4);
    assert_eq!(rows(&grid, &clusters), ["  │", "  ▼", "┌───┐", "│ B │"]);
    let view = MermaidView::new(&diagram).scroll(2, 0);
    let (grid, clusters) = paint(&view, Rect::new(0, 0, 3, 3), 3, 3);
    assert_eq!(rows(&grid, &clusters), ["──┐", "A │", "┬─┘"]);
    Ok(())
}

#[test]
fn scrolling_stops_at_the_far_edges() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A --> B")?;
    let view = MermaidView::new(&diagram).scroll(99, 99);
    let (grid, clusters) = paint(&view, Rect::new(0, 0, 5, 4), 5, 4);
    assert_eq!(rows(&grid, &clusters), ["  ▼", "┌───┐", "│ B │", "└───┘"]);
    let view = MermaidView::new(&diagram).scroll(99, 0);
    let (grid, clusters) = paint(&view, Rect::new(0, 0, 3, 2), 3, 2);
    assert_eq!(
        rows(&grid, &clusters),
        ["──┐", "A │"].map(|row| row.replace("A ", "A "))
    );
    assert_eq!(view.max_scroll(Rect::new(0, 0, 3, 2)), (2, 6));
    assert_eq!(view.max_scroll(Rect::new(0, 0, 5, 8)), (0, 0));
    assert_eq!(view.max_scroll(Rect::new(0, 0, 50, 80)), (0, 0));
    Ok(())
}

#[test]
fn a_larger_area_leaves_the_rest_as_background() -> Test {
    let diagram = MermaidDiagram::parse("flowchart LR\n  A --> B")?;
    let (grid, clusters) = paint(&MermaidView::new(&diagram), Rect::new(1, 1, 16, 5), 18, 7);
    assert_eq!(
        rows(&grid, &clusters),
        [
            "",
            " ┌───┐  ┌───┐",
            " │ A ├─▶│ B │",
            " └───┘  └───┘",
            "",
            "",
            "",
        ]
    );
    let ground = token(Token::Surface0);
    assert_eq!(style_at(&grid, 1, 5).bg, ground);
    assert_eq!(style_at(&grid, 16, 1).bg, ground);
    assert_eq!(style_at(&grid, 0, 0), Style::default());
    assert_eq!(style_at(&grid, 17, 6), Style::default());
    Ok(())
}

#[test]
fn an_empty_area_draws_nothing() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A")?;
    for area in [
        Rect::new(0, 0, 0, 5),
        Rect::new(0, 0, 5, 0),
        Rect::new(2, 2, 0, 0),
    ] {
        let (grid, clusters) = paint(&MermaidView::new(&diagram), area, 8, 8);
        assert!(
            rows(&grid, &clusters).iter().all(String::is_empty),
            "{area:?}"
        );
    }
    Ok(())
}

#[test]
fn an_area_past_the_grid_is_clipped_without_panicking() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A")?;
    let (grid, clusters) = paint(&MermaidView::new(&diagram), Rect::new(2, 1, 40, 40), 6, 4);
    assert_eq!(rows(&grid, &clusters), ["", "  ┌───", "  │ A", "  └───"]);
    Ok(())
}

#[test]
fn a_wide_character_cut_by_the_edge_is_blanked() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A[日本]")?;
    let (grid, clusters) = paint(
        &MermaidView::new(&diagram).scroll(3, 1),
        Rect::new(0, 0, 4, 1),
        4,
        1,
    );
    assert_eq!(rows(&grid, &clusters), [" 本"]);
    let (grid, clusters) = paint(
        &MermaidView::new(&diagram).scroll(2, 1),
        Rect::new(0, 0, 3, 1),
        3,
        1,
    );
    assert_eq!(rows(&grid, &clusters), ["日"]);
    Ok(())
}

#[test]
fn ascii_glyphs_replace_box_drawing() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A --> B\n  B -.-> C\n  C ==> D")?;
    let size = diagram.size();
    let view = MermaidView::new(&diagram).glyphs(MermaidGlyphs::Ascii);
    let (grid, clusters) = paint(
        &view,
        Rect::new(0, 0, size.width, size.height),
        size.width,
        size.height,
    );
    assert_eq!(
        rows(&grid, &clusters),
        [
            "+---+", "| A |", "+-+-+", "  |", "  v", "+---+", "| B |", "+-+-+", "  :", "  v",
            "+---+", "| C |", "+-+-+", "  #", "  v", "+---+", "| D |", "+---+",
        ]
    );
    let diagram = MermaidDiagram::parse("flowchart LR\n  A ==> B\n  B -.-> C\n  C --> D")?;
    let size = diagram.size();
    let view = MermaidView::new(&diagram).glyphs(MermaidGlyphs::Ascii);
    let (grid, clusters) = paint(
        &view,
        Rect::new(0, 0, size.width, size.height),
        size.width,
        size.height,
    );
    assert_eq!(
        rows(&grid, &clusters),
        [
            "+---+  +---+  +---+  +---+",
            "| A +=>| B +.>| C +->| D |",
            "+---+  +---+  +---+  +---+",
        ]
    );
    Ok(())
}

#[test]
fn ascii_glyphs_cover_diamonds_arrows_and_sequences() -> Test {
    let diagram = MermaidDiagram::parse("flowchart BT\n  A{x} --> B")?;
    let size = diagram.size();
    let view = MermaidView::new(&diagram).glyphs(MermaidGlyphs::Ascii);
    let area = Rect::new(0, 0, size.width, size.height);
    let (grid, clusters) = paint(&view, area, size.width, size.height);
    assert_eq!(
        rows(&grid, &clusters),
        [
            " +---+", " | B |", " +---+", "   ^", "   |", " /-+-\\", "<  x  >", " \\---/"
        ]
    );
    let diagram = MermaidDiagram::parse("sequenceDiagram\n  A->>B: a\n  B-xA: b\n  A-)B: c")?;
    let size = diagram.size();
    let view = MermaidView::new(&diagram).glyphs(MermaidGlyphs::Ascii);
    let area = Rect::new(0, 0, size.width, size.height);
    let (grid, clusters) = paint(&view, area, size.width, size.height);
    assert_eq!(
        rows(&grid, &clusters),
        [
            "+---+  +---+",
            "| A |  | B |",
            "+-+-+  +-+-+",
            "  :      :",
            "  :  a   :",
            "  +----->:",
            "  :      :",
            "  :  b   :",
            "  :x-----+",
            "  :      :",
            "  :  c   :",
            "  +-----):",
            "  :      :",
            "+-+-+  +-+-+",
            "| A |  | B |",
            "+---+  +---+",
        ]
    );
    Ok(())
}

#[test]
fn colors_come_from_theme_tokens() -> Test {
    let (grid, _) = natural("flowchart TD\n  A -->|yes| B")?;
    let accent = token(Token::Accent);
    let raised = token(Token::Surface1);
    let ground = token(Token::Surface0);
    let muted = token(Token::TextMuted);
    let text = token(Token::TextPrimary);
    assert_eq!(style_at(&grid, 0, 0), Style::new().fg(accent).bg(raised));
    assert_eq!(style_at(&grid, 2, 1), Style::new().fg(text).bg(raised));
    assert_eq!(style_at(&grid, 1, 1), Style::new().fg(text).bg(raised));
    assert_eq!(style_at(&grid, 2, 3), Style::new().fg(muted).bg(ground));
    assert_eq!(style_at(&grid, 2, 5), Style::new().fg(muted).bg(ground));
    assert_eq!(style_at(&grid, 4, 4), Style::new().fg(muted).bg(ground));
    assert_eq!(style_at(&grid, 4, 7), Style::new().fg(accent).bg(raised));
    assert_eq!(style_at(&grid, 0, 3), Style::new().fg(text).bg(ground));
    Ok(())
}

#[test]
fn sequence_colors_come_from_theme_tokens() -> Test {
    let (grid, _) = natural("sequenceDiagram\n  A-xB: boom\n  Note over A: n\n  opt x\n  end")?;
    let border = token(Token::Border);
    let text = token(Token::TextPrimary);
    let ground = token(Token::Surface0);
    let raised = token(Token::Surface1);
    let warn = token(Token::Warn);
    let fail = token(Token::Fail);
    let accent = token(Token::Accent);
    assert_eq!(style_at(&grid, 4, 3), Style::new().fg(border).bg(ground));
    assert_eq!(style_at(&grid, 6, 4), Style::new().fg(text).bg(ground));
    let cross = style_at(&grid, 11, 5);
    assert_eq!((cross.fg, cross.bg), (fail, ground));
    assert!(cross.attrs.bold());
    assert_eq!(style_at(&grid, 2, 7), Style::new().fg(warn).bg(raised));
    assert_eq!(style_at(&grid, 3, 8), Style::new().fg(text).bg(raised));
    let tag = style_at(&grid, 3, 11);
    assert_eq!((tag.fg, tag.bg), (accent, ground));
    assert!(tag.attrs.bold());
    assert_eq!(style_at(&grid, 0, 11), Style::new().fg(border).bg(ground));
    Ok(())
}

#[test]
fn the_background_style_replaces_the_default_ground() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A --> B")?;
    let panel = Style::new().fg(Color::Rgb(1, 2, 3)).bg(Color::Rgb(4, 5, 6));
    let (grid, _) = paint(
        &MermaidView::new(&diagram).background(panel),
        Rect::new(0, 0, 8, 9),
        8,
        9,
    );
    assert_eq!(style_at(&grid, 7, 8), panel);
    assert_eq!(style_at(&grid, 2, 3).bg, Color::Rgb(4, 5, 6));
    assert_eq!(style_at(&grid, 0, 0).bg, token(Token::Surface1));
    Ok(())
}

#[test]
fn disabled_dims_every_cell() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A --> B")?;
    let (grid, _) = paint(
        &MermaidView::new(&diagram).state(WidgetState::Disabled),
        Rect::new(0, 0, 5, 8),
        5,
        8,
    );
    for (x, y) in [(0, 0), (2, 1), (2, 3), (2, 4), (4, 7)] {
        assert!(style_at(&grid, x, y).attrs.dim(), "{x},{y}");
    }
    let (grid, _) = natural("flowchart TD\n  A --> B")?;
    assert!(!style_at(&grid, 2, 3).attrs.dim());
    Ok(())
}

#[test]
fn drawing_twice_gives_the_same_cells() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n  A -->|x| B --> C\n  C --> A\n  A -.-> C")?;
    let size = diagram.size();
    let view = MermaidView::new(&diagram);
    let area = Rect::new(0, 0, size.width, size.height);
    let (first, _) = paint(&view, area, size.width, size.height);
    let (second, _) = paint(&view, area, size.width, size.height);
    for y in 0..size.height {
        for x in 0..size.width {
            assert_eq!(first.get(x, y), second.get(x, y));
        }
    }
    Ok(())
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap_or(0) % below.max(1)
    }
}

const SAMPLES: [&str; 3] = [
    "flowchart TD\n  A[Start] -->|go| B{Ok?}\n  B -- no --> C([Fix]) -.-> A\n  B ==> D((Done)) --- E\n",
    "graph LR\n  X --> Y --> X\n  Y -->|a| Z\n  Z --> Z\n  style X fill:#f00\n",
    "sequenceDiagram\n  autonumber\n  participant A as Alice\n  actor B\n  A->>B: hi\n  \
     loop l\n  alt a\n  B-->>A: x\n  else b\n  A-xA: y\n  end\n  end\n  Note over A,B: n\n",
];

const PIECES: [&str; 32] = [
    "-->",
    "---",
    "-.->",
    "==>",
    "|",
    "[",
    "]",
    "(",
    ")",
    "((",
    "))",
    "{",
    "}",
    "\"",
    ";",
    "&",
    "%%",
    "\n",
    " ",
    "A",
    "B",
    "->>",
    "-x",
    "Note over ",
    ":",
    "loop ",
    "end",
    "else ",
    "<br/>",
    "subgraph ",
    "日",
    "\u{301}",
];

fn check_parse(source: &str) {
    let lines = source.lines().count().max(1);
    match MermaidDiagram::parse(source) {
        Ok(diagram) => {
            let size = diagram.size();
            let (grid, clusters) = paint(
                &MermaidView::new(&diagram),
                Rect::new(0, 0, size.width, size.height),
                size.width,
                size.height,
            );
            assert!(u32::from(size.width) * u32::from(size.height) <= 1_000_000);
            for y in 0..size.height {
                assert_eq!(
                    lunatui_core::width(&grid.row_display(y, &clusters)),
                    usize::from(size.width),
                    "{source:?}"
                );
            }
        }
        Err(error) => {
            assert!(
                error.line() >= 1 && error.line() <= lines,
                "{source:?} {error}"
            );
            assert!(!error.message().is_empty());
        }
    }
}

#[test]
fn truncated_input_never_panics_and_errors_stay_inside_the_source() {
    for sample in SAMPLES {
        let mut cut = 0;
        while cut <= sample.len() {
            if sample.is_char_boundary(cut) {
                check_parse(&sample[..cut]);
            }
            cut += 1;
        }
    }
}

#[test]
fn mutated_input_never_panics() {
    let mut random = Lcg(7);
    for round in 0..3000 {
        let mut source = String::from(SAMPLES[round % SAMPLES.len()]);
        for _ in 0..=random.next(6) {
            let at = random.next(source.chars().count() + 1);
            let byte = source
                .char_indices()
                .nth(at)
                .map_or(source.len(), |(index, _)| index);
            match random.next(3) {
                0 => source.insert_str(byte, PIECES[random.next(PIECES.len())]),
                1 => {
                    if let Some(ch) = source[byte..].chars().next() {
                        source.replace_range(byte..byte + ch.len_utf8(), "");
                    }
                }
                _ => source.truncate(byte),
            }
        }
        check_parse(&source);
    }
}

#[test]
fn random_input_never_panics() {
    let mut random = Lcg(99);
    for _ in 0..2000 {
        let mut source =
            String::from(["flowchart TD\n", "sequenceDiagram\n", "graph RL\n"][random.next(3)]);
        for _ in 0..random.next(40) {
            source.push_str(PIECES[random.next(PIECES.len())]);
        }
        check_parse(&source);
    }
}

#[test]
fn random_flowcharts_draw_every_label_in_every_direction() -> Test {
    let mut random = Lcg(2026);
    let directions = ["TD", "TB", "BT", "LR", "RL"];
    let links = ["-->", "---", "-.->", "==>", "-->|lab|", "-- txt -->"];
    let shapes = [
        ("[", "]"),
        ("(", ")"),
        ("([", "])"),
        ("{", "}"),
        ("((", "))"),
    ];
    for round in 0..300 {
        let nodes = 1 + random.next(9);
        let mut source = format!("flowchart {}\n", directions[round % directions.len()]);
        for node in 0..nodes {
            let (open, close) = shapes[random.next(shapes.len())];
            source.push_str(&format!("  N{node}{open}Label{node}{close}\n"));
        }
        for _ in 0..random.next(14) {
            let (from, to) = (random.next(nodes), random.next(nodes));
            source.push_str(&format!(
                "  N{from} {} N{to}\n",
                links[random.next(links.len())]
            ));
        }
        let drawn = drawn(&source)?.join("\n");
        for node in 0..nodes {
            assert!(drawn.contains(&format!("Label{node}")), "{source}\n{drawn}");
        }
    }
    Ok(())
}

#[test]
fn combining_marks_and_joined_emoji_keep_their_graphemes_and_widths() -> Test {
    assert_eq!(
        drawn("flowchart TD\n  A[Cafe\u{301}]")?,
        ["┌───────┐", "│ Cafe\u{301}  │", "└───────┘"]
    );
    assert_eq!(
        drawn("flowchart TD\n  A[👩‍💻]")?,
        ["┌─────┐", "│ 👩‍💻  │", "└─────┘"]
    );
    assert_eq!(
        drawn("flowchart TD\n  A[🇨🇦]")?,
        ["┌─────┐", "│ 🇨🇦  │", "└─────┘"]
    );
    let out = drawn("sequenceDiagram\n participant A as Cafe\u{301}\n A->>B: 👩‍💻 ready")?.join("\n");
    assert!(
        out.contains("Cafe\u{301}") && out.contains("👩‍💻 ready"),
        "{out}"
    );
    Ok(())
}

#[test]
fn horizontal_scroll_never_draws_half_of_a_joined_emoji() -> Test {
    let diagram = MermaidDiagram::parse("flowchart TD\n A[👩‍💻]")?;
    let view = MermaidView::new(&diagram).scroll(3, 1);
    let (grid, clusters) = paint(&view, Rect::new(0, 0, 2, 1), 2, 1);
    assert_eq!(grid.row_display(0, &clusters), "  ");
    let view = MermaidView::new(&diagram).scroll(2, 1);
    let (grid, clusters) = paint(&view, Rect::new(0, 0, 1, 1), 1, 1);
    assert_eq!(grid.row_display(0, &clusters), " ");
    Ok(())
}
