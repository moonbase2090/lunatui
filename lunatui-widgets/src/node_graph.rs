// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod canvas;
mod layout;
#[cfg(feature = "pixel-node-graph")]
mod pixel;
mod view;

use core::fmt;
use lunatui_core::{Rect, Size};

pub use view::NodeGraphView;

/// A labelled node with a requested outer size, including its border.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphNode<'a> {
    title: &'a str,
    body: &'a str,
    size: Size,
}

impl<'a> GraphNode<'a> {
    /// Creates a node. Each dimension is at least three cells, leaving room for ports.
    ///
    /// ```
    /// use lunatui_core::Size;
    /// use lunatui_widgets::GraphNode;
    /// let node = GraphNode::new("Build", Size::new(16, 5)).body("24 checks passed");
    /// ```
    #[must_use]
    pub const fn new(title: &'a str, size: Size) -> Self {
        Self {
            title,
            body: "",
            size: Size::new(
                if size.width < 3 { 3 } else { size.width },
                if size.height < 3 { 3 } else { size.height },
            ),
        }
    }

    /// Sets the text inside the border. Lines and wide graphemes are clipped to the interior.
    #[must_use]
    pub const fn body(mut self, text: &'a str) -> Self {
        self.body = text;
        self
    }
}

/// An output port connected to an input port. Port zero is the first interior row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeConnection {
    from: usize,
    output: u16,
    to: usize,
    input: u16,
}

impl NodeConnection {
    /// Connects the indexed nodes at the given zero-based interior rows.
    /// Endpoints and port rows are checked by [`NodeGraph::new`].
    #[must_use]
    pub const fn new(from: usize, output: u16, to: usize, input: u16) -> Self {
        Self {
            from,
            output,
            to,
            input,
        }
    }
}

/// Why a node graph cannot be laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeGraphError {
    /// A connection refers to a node outside the supplied slice.
    InvalidNode,
    /// A port row lies outside its node's interior.
    InvalidPort,
    /// The graph contains a directed cycle, including a self-connection.
    Cycle,
    /// The graph exceeds 256 nodes, 2048 connections, or 262144 layout cells, 64 KiB of labels, or the routing budget.
    TooLarge,
    /// An orthogonal route cannot reach its destination without crossing a node.
    NoRoute,
}

impl fmt::Display for NodeGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidNode => "connection refers to a missing node",
            Self::InvalidPort => "connection port is outside the node interior",
            Self::Cycle => "node graph contains a directed cycle",
            Self::TooLarge => "node graph exceeds its layout budget",
            Self::NoRoute => "connection cannot be routed around the nodes",
        })
    }
}

impl core::error::Error for NodeGraphError {}

/// A native, automatically laid-out directed acyclic graph with input and output ports.
///
/// Nodes are packed into columns from inputs on the left to outputs on the right. Incoming
/// connections are ordered by their input port, and orthogonal routes avoid node interiors.
/// Layout and Unicode segmentation happen once here; [`NodeGraphView`] resolves theme colors
/// and draws the visible cells. Rebuild the graph when its data changes.
///
/// ```
/// use lunatui_core::Size;
/// use lunatui_widgets::{GraphNode, NodeConnection, NodeGraph};
/// let nodes = [GraphNode::new("Source", Size::new(12, 4)), GraphNode::new("Build", Size::new(12, 4))];
/// let edges = [NodeConnection::new(0, 0, 1, 0)];
/// let graph = NodeGraph::new(&nodes, &edges)?;
/// assert_eq!(graph.size(), Size::new(31, 6));
/// # Ok::<(), lunatui_widgets::NodeGraphError>(())
/// ```
#[derive(Debug)]
pub struct NodeGraph {
    placements: Vec<Rect>,
    size: Size,
    canvas: canvas::Canvas,
}

impl NodeGraph {
    /// Validates and lays out a graph. Empty graphs are valid and have zero intrinsic size.
    ///
    /// # Errors
    /// Returns [`NodeGraphError`] for missing endpoints, invalid ports, cycles or exhausted
    /// layout/routing space. At most 256 nodes, 2048 edges, 262144 cells and 64 KiB of labels
    /// are accepted. Routing may inspect at most 16 million cells across all connections.
    pub fn new(
        nodes: &[GraphNode<'_>],
        connections: &[NodeConnection],
    ) -> Result<Self, NodeGraphError> {
        let (placements, size) = layout::place(nodes, connections)?;
        let canvas = canvas::Canvas::build(nodes, connections, &placements, size)?;
        Ok(Self {
            placements,
            size,
            canvas,
        })
    }

    /// The complete drawing size, including one cell of routing space around the nodes.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// A node's outer rectangle in graph coordinates, or `None` for a missing node.
    #[must_use]
    pub fn node_area(&self, node: usize) -> Option<Rect> {
        self.placements.get(node).copied()
    }

    /// Creates a view. Its selection and scroll position belong to the application.
    #[must_use]
    pub const fn view(&self) -> NodeGraphView<'_> {
        NodeGraphView::new(self)
    }
}
