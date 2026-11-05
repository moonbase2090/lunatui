// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::{GraphNode, NodeConnection, NodeGraphError};
use lunatui_core::{Rect, Size};
use std::collections::VecDeque;

pub(super) fn place(
    nodes: &[GraphNode<'_>],
    edges: &[NodeConnection],
) -> Result<(Vec<Rect>, Size), NodeGraphError> {
    if nodes.len() > 256 || edges.len() > 2048 {
        return Err(NodeGraphError::TooLarge);
    }
    let mut remaining = 65_536_usize;
    for node in nodes {
        remaining = remaining
            .checked_sub(node.title.len())
            .and_then(|left| left.checked_sub(node.body.len()))
            .ok_or(NodeGraphError::TooLarge)?;
    }
    let mut pending = vec![0_usize; nodes.len()];
    let mut incoming = vec![Vec::new(); nodes.len()];
    for edge in edges {
        let (Some(from), Some(to)) = (nodes.get(edge.from), nodes.get(edge.to)) else {
            return Err(NodeGraphError::InvalidNode);
        };
        if edge.output >= from.size.height - 2 || edge.input >= to.size.height - 2 {
            return Err(NodeGraphError::InvalidPort);
        }
        pending[edge.from] += 1;
        incoming[edge.to].push((edge.input, edge.from));
    }
    for entries in &mut incoming {
        entries.sort_by_key(|&(port, node)| (port, node));
    }
    let roots: Vec<_> = pending
        .iter()
        .enumerate()
        .filter_map(|(i, &n)| (n == 0).then_some(i))
        .collect();
    let mut queue: VecDeque<_> = roots.iter().copied().collect();
    let mut rank = vec![0_usize; nodes.len()];
    let mut count = 0;
    while let Some(node) = queue.pop_front() {
        count += 1;
        for &(_, parent) in &incoming[node] {
            rank[parent] = rank[parent].max(rank[node] + 1);
            pending[parent] -= 1;
            if pending[parent] == 0 {
                queue.push_back(parent);
            }
        }
    }
    if count != nodes.len() {
        return Err(NodeGraphError::Cycle);
    }
    if nodes.is_empty() {
        return Ok((Vec::new(), Size::new(0, 0)));
    }
    let columns = rank.iter().copied().max().unwrap_or(0) + 1;
    let mut widths = vec![0_u32; columns];
    for (node, &column) in nodes.iter().zip(&rank) {
        widths[column] = widths[column].max(u32::from(node.size.width));
    }
    let total = widths.iter().sum::<u32>() + 5 * u32::try_from(columns - 1).unwrap_or(0) + 2;
    let width = u16::try_from(total).map_err(|_| NodeGraphError::TooLarge)?;
    let mut right = vec![1_u32; columns];
    for column in 1..columns {
        right[column] = right[column - 1] + widths[column - 1] + 5;
    }
    let mut packing = Packing {
        nodes,
        incoming: &incoming,
        rank: &rank,
        right: &right,
        rows: vec![1_u32; columns],
        placements: vec![None; nodes.len()],
        width: total,
    };
    for root in roots {
        packing.branch(root, 1);
    }
    let height = packing.rows.iter().copied().max().unwrap_or(1) + 1;
    if u64::from(width) * u64::from(height) > 262_144 {
        return Err(NodeGraphError::TooLarge);
    }
    let height = u16::try_from(height).map_err(|_| NodeGraphError::TooLarge)?;
    let placements = packing
        .placements
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(NodeGraphError::Cycle)?;
    Ok((placements, Size::new(width, height)))
}

struct Packing<'a> {
    nodes: &'a [GraphNode<'a>],
    incoming: &'a [Vec<(u16, usize)>],
    rank: &'a [usize],
    right: &'a [u32],
    rows: Vec<u32>,
    placements: Vec<Option<Rect>>,
    width: u32,
}

impl Packing<'_> {
    fn branch(&mut self, node: usize, proposed: u32) {
        if self.placements[node].is_some() {
            return;
        }
        let column = self.rank[node];
        let top = proposed.max(self.rows[column]);
        let size = self.nodes[node].size;
        self.rows[column] = top + u32::from(size.height);
        let left = self.width - self.right[column] - u32::from(size.width);
        self.placements[node] = Some(Rect::new(
            u16::try_from(left).unwrap_or(0),
            u16::try_from(top).unwrap_or(u16::MAX),
            size.width,
            size.height,
        ));
        for index in 0..self.incoming[node].len() {
            let parent = self.incoming[node][index].1;
            self.branch(parent, top);
        }
    }
}
