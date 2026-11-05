// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(feature = "node-graph")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    GraphNode, NodeConnection, NodeGraph, NodeGraphError, RenderCx, Widget, WidgetState,
};

fn paint(graph: &NodeGraph) -> (Grid, ClusterTable) {
    let size = graph.size();
    let mut grid = Grid::new(size.width, size.height);
    let mut clusters = ClusterTable::new();
    let theme = Theme::builtin(Variant::Dark);
    graph.view().render(
        Rect::new(0, 0, size.width, size.height),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme),
    );
    (grid, clusters)
}

#[test]
fn draws_real_borders_labels_ports_and_a_connected_route() -> Result<(), Box<dyn core::error::Error>>
{
    let nodes = [
        GraphNode::new("A", Size::new(5, 3)).body("one"),
        GraphNode::new("B", Size::new(5, 3)).body("two"),
    ];
    let edges = [NodeConnection::new(0, 0, 1, 0)];
    let graph = NodeGraph::new(&nodes, &edges)?;
    assert_eq!(graph.size(), Size::new(17, 5));
    assert_eq!(graph.node_area(0), Some(Rect::new(1, 1, 5, 3)));
    assert_eq!(graph.node_area(1), Some(Rect::new(11, 1, 5, 3)));
    let (grid, clusters) = paint(&graph);
    let rows: Vec<_> = (0..5).map(|row| grid.row_display(row, &clusters)).collect();
    assert_eq!(
        rows,
        [
            "                 ",
            " ╭A──╮     ╭B──╮ ",
            " │one├─────┤two│ ",
            " ╰───╯     ╰───╯ ",
            "                 "
        ]
    );
    Ok(())
}

#[test]
fn orders_fan_in_by_input_port() -> Result<(), Box<dyn core::error::Error>> {
    let nodes = [
        GraphNode::new("A", Size::new(8, 4)),
        GraphNode::new("B", Size::new(8, 4)),
        GraphNode::new("Merge", Size::new(8, 8)),
    ];
    let edges = [
        NodeConnection::new(1, 0, 2, 4),
        NodeConnection::new(0, 0, 2, 0),
    ];
    let graph = NodeGraph::new(&nodes, &edges)?;
    assert_eq!(graph.node_area(0), Some(Rect::new(1, 1, 8, 4)));
    assert_eq!(graph.node_area(1), Some(Rect::new(1, 5, 8, 4)));
    assert_eq!(graph.node_area(2), Some(Rect::new(14, 1, 8, 8)));
    let (grid, clusters) = paint(&graph);
    assert!(grid.row_display(2, &clusters).contains("├─────┤"));
    assert!(grid.row_display(6, &clusters).contains("├─────┤"));
    Ok(())
}

#[test]
fn rejects_missing_nodes_invalid_ports_cycles_and_excessive_extents() {
    let nodes = [
        GraphNode::new("A", Size::new(5, 3)),
        GraphNode::new("B", Size::new(5, 3)),
    ];
    assert_eq!(
        NodeGraph::new(&nodes, &[NodeConnection::new(0, 0, 2, 0)]).err(),
        Some(NodeGraphError::InvalidNode)
    );
    assert_eq!(
        NodeGraph::new(&nodes, &[NodeConnection::new(0, 1, 1, 0)]).err(),
        Some(NodeGraphError::InvalidPort)
    );
    assert_eq!(
        NodeGraph::new(&nodes, &[NodeConnection::new(0, 0, 0, 0)]).err(),
        Some(NodeGraphError::Cycle)
    );
    assert_eq!(
        NodeGraph::new(
            &nodes,
            &[
                NodeConnection::new(0, 0, 1, 0),
                NodeConnection::new(1, 0, 0, 0)
            ]
        )
        .err(),
        Some(NodeGraphError::Cycle)
    );
    let huge = [GraphNode::new("A", Size::new(u16::MAX, u16::MAX))];
    assert_eq!(
        NodeGraph::new(&huge, &[]).err(),
        Some(NodeGraphError::TooLarge)
    );
    let many = [GraphNode::new("A", Size::new(3, 3)); 257];
    assert_eq!(
        NodeGraph::new(&many, &[]).err(),
        Some(NodeGraphError::TooLarge)
    );
    let oversized_label = "x".repeat(65_537);
    let label_node = [GraphNode::new(&oversized_label, Size::new(3, 3))];
    assert_eq!(
        NodeGraph::new(&label_node, &[]).err(),
        Some(NodeGraphError::TooLarge)
    );
    let deep = [GraphNode::new("A", Size::new(3, u16::MAX)); 256];
    assert_eq!(
        NodeGraph::new(&deep, &[]).err(),
        Some(NodeGraphError::TooLarge)
    );
}

#[test]
fn clips_wide_labels_and_preserves_cells_outside_a_scrolled_viewport()
-> Result<(), Box<dyn core::error::Error>> {
    let nodes = [GraphNode::new("界A", Size::new(5, 3)).body("好Z")];
    let graph = NodeGraph::new(&nodes, &[])?;
    let mut grid = Grid::new(7, 7);
    let mut clusters = ClusterTable::new();
    let theme = Theme::builtin(Variant::Dark);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.fill(Rect::new(0, 0, 7, 7), Style::new(), '.');
    graph
        .view()
        .scroll(3, 1)
        .render(Rect::new(2, 2, 3, 3), &mut cx);
    assert_eq!(grid.row_display(1, &clusters), ".......");
    assert_eq!(grid.row_display(2, &clusters), ".. A╮..");
    assert_eq!(grid.row_display(3, &clusters), ".. Z│..");
    assert_eq!(grid.row_display(4, &clusters), "..──╯..");
    assert_eq!(grid.row_display(5, &clusters), ".......");
    Ok(())
}

#[test]
fn selection_and_disabled_state_resolve_the_current_theme()
-> Result<(), Box<dyn core::error::Error>> {
    let nodes = [GraphNode::new("A", Size::new(5, 3))];
    let graph = NodeGraph::new(&nodes, &[])?;
    for variant in [Variant::Dark, Variant::Light] {
        let mut theme = Theme::builtin(variant);
        theme.set_token(Token::FocusRing, Color::Rgb(1, 2, 3));
        theme.set_token(Token::TextMuted, Color::Rgb(4, 5, 6));
        let mut grid = Grid::new(7, 5);
        let mut clusters = ClusterTable::new();
        graph.view().selected(Some(0)).render(
            Rect::new(0, 0, 7, 5),
            &mut RenderCx::new(&mut grid, &mut clusters, &theme),
        );
        let style = grid.get(1, 1).ok_or("missing border")?.style();
        assert_eq!(style.fg, Color::Rgb(1, 2, 3));
        assert!(style.attrs.bold());
        graph
            .view()
            .selected(Some(0))
            .state(WidgetState::Disabled)
            .render(
                Rect::new(0, 0, 7, 5),
                &mut RenderCx::new(&mut grid, &mut clusters, &theme),
            );
        let style = grid.get(1, 1).ok_or("missing border")?.style();
        assert_eq!(style.fg, Color::Rgb(4, 5, 6));
        assert!(style.attrs.dim());
        assert!(!style.attrs.bold());
    }
    Ok(())
}

#[test]
fn empty_and_zero_sized_viewports_are_supported() -> Result<(), Box<dyn core::error::Error>> {
    let graph = NodeGraph::new(&[], &[])?;
    assert_eq!(graph.size(), Size::new(0, 0));
    assert_eq!(graph.node_area(0), None);
    let mut grid = Grid::new(0, 0);
    let mut clusters = ClusterTable::new();
    graph.view().render(
        Rect::new(u16::MAX, u16::MAX, 2, 2),
        &mut RenderCx::new(&mut grid, &mut clusters, &Theme::builtin(Variant::Dark)),
    );
    assert_eq!(graph.view().measure(Size::new(8, 8)), Size::new(0, 0));
    Ok(())
}

#[test]
fn routes_a_bend_without_covering_node_contents() -> Result<(), NodeGraphError> {
    let nodes = [
        GraphNode::new("A", Size::new(5, 3)).body("x"),
        GraphNode::new("B", Size::new(5, 5)).body("one\ntwo\nend"),
    ];
    let graph = NodeGraph::new(&nodes, &[NodeConnection::new(0, 0, 1, 2)])?;
    let (grid, clusters) = paint(&graph);
    assert_eq!(grid.row_display(2, &clusters), " │x  ├────╮│one│ ");
    assert_eq!(grid.row_display(3, &clusters), " ╰───╯    ││two│ ");
    assert_eq!(grid.row_display(4, &clusters), "          ╰┤end│ ");
    Ok(())
}

#[test]
fn hit_testing_accounts_for_viewport_origin_scroll_and_disabled_state() -> Result<(), NodeGraphError>
{
    let graph = NodeGraph::new(
        &[
            GraphNode::new("A", Size::new(5, 3)),
            GraphNode::new("B", Size::new(5, 3)),
        ],
        &[NodeConnection::new(0, 0, 1, 0)],
    )?;
    let area = Rect::new(3, 2, 7, 5);
    assert_eq!(graph.view().node_at(area, 4, 3), Some(0));
    assert_eq!(graph.view().scroll(10, 0).node_at(area, 4, 3), Some(1));
    assert_eq!(
        graph.view().scroll(u16::MAX, 0).node_at(area, 4, 3),
        Some(1)
    );
    assert_eq!(graph.view().node_at(area, 3, 2), None);
    assert_eq!(graph.view().node_at(area, 2, 3), None);
    assert_eq!(
        graph
            .view()
            .state(WidgetState::Disabled)
            .node_at(area, 4, 3),
        None
    );
    Ok(())
}

#[test]
fn a_long_connection_detours_around_an_intermediate_node() -> Result<(), NodeGraphError> {
    let nodes = [
        GraphNode::new("A", Size::new(5, 5)),
        GraphNode::new("B", Size::new(5, 5)).body("mid\nfit\nbox"),
        GraphNode::new("C", Size::new(5, 5)),
    ];
    let edges = [
        NodeConnection::new(0, 0, 1, 0),
        NodeConnection::new(1, 0, 2, 0),
        NodeConnection::new(0, 1, 2, 2),
    ];
    let graph = NodeGraph::new(&nodes, &edges)?;
    let (grid, clusters) = paint(&graph);
    assert!(grid.row_display(2, &clusters).contains("┤mid├"));
    assert!(grid.row_display(3, &clusters).contains("│fit│"));
    assert!(grid.row_display(4, &clusters).contains("│box│"));
    assert_eq!(
        grid.row_display(6, &clusters),
        "          ╰─────╯          "
    );
    Ok(())
}
