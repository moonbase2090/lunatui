// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::canvas::text_columns;
use super::flow_parse::Edge;

const SWEEPS: usize = 4;
const SHIFT_PASSES: usize = 4;

pub(super) fn count(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

pub(super) struct Chain {
    pub(super) edge: usize,
    pub(super) nodes: Vec<usize>,
    pub(super) reversed: bool,
    pub(super) label: Option<usize>,
    pub(super) first_port: i32,
    pub(super) last_port: i32,
}

pub(super) struct Graph {
    pub(super) real: usize,
    pub(super) rank: Vec<usize>,
    pub(super) left: Vec<i32>,
    pub(super) right: Vec<i32>,
    pub(super) thick: Vec<i32>,
    pub(super) up: Vec<Vec<usize>>,
    pub(super) down: Vec<Vec<usize>>,
    pub(super) ranks: Vec<Vec<usize>>,
    pub(super) place: Vec<i32>,
    pub(super) chains: Vec<Chain>,
    anchor: Vec<usize>,
}

pub(super) struct Spacing {
    pub(super) horizontal: bool,
}

pub(super) fn reversed_edges(nodes: usize, edges: &[Edge]) -> Vec<bool> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); nodes];
    for (at, edge) in edges.iter().enumerate() {
        if edge.from != edge.to {
            out[edge.from].push(at);
        }
    }
    let mut state = vec![0_u8; nodes];
    let mut reversed = vec![false; edges.len()];
    for root in 0..nodes {
        if state[root] != 0 {
            continue;
        }
        state[root] = 1;
        let mut stack = vec![(root, 0_usize)];
        while let Some(top) = stack.last_mut() {
            let (node, next) = *top;
            if let Some(&at) = out[node].get(next) {
                top.1 += 1;
                let target = edges[at].to;
                match state[target] {
                    0 => {
                        state[target] = 1;
                        stack.push((target, 0));
                    }
                    1 => reversed[at] = true,
                    _ => {}
                }
            } else {
                state[node] = 2;
                stack.pop();
            }
        }
    }
    reversed
}

fn layered(edge: &Edge, reversed: bool) -> (usize, usize) {
    if reversed {
        (edge.to, edge.from)
    } else {
        (edge.from, edge.to)
    }
}

fn longest_path(nodes: usize, edges: &[Edge], reversed: &[bool]) -> Vec<usize> {
    let mut indegree = vec![0_usize; nodes];
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); nodes];
    for (edge, &flipped) in edges.iter().zip(reversed) {
        if edge.from != edge.to {
            let (from, to) = layered(edge, flipped);
            out[from].push(to);
            indegree[to] += 1;
        }
    }
    let mut queue: Vec<usize> = (0..nodes).filter(|&node| indegree[node] == 0).collect();
    let mut rank = vec![0_usize; nodes];
    let mut head = 0;
    while let Some(&node) = queue.get(head) {
        head += 1;
        for &next in &out[node] {
            rank[next] = rank[next].max(rank[node] + 1);
            indegree[next] -= 1;
            if indegree[next] == 0 {
                queue.push(next);
            }
        }
    }
    rank
}

impl Graph {
    pub(super) fn build(
        edges: &[Edge],
        real_dims: &[(i32, i32, i32)],
        ports: &[i32],
        spacing: &Spacing,
    ) -> Self {
        let nodes = real_dims.len();
        let reversed = reversed_edges(nodes, edges);
        let stride = if edges.iter().any(|edge| edge.label.is_some()) {
            2
        } else {
            1
        };
        let mut rank: Vec<usize> = longest_path(nodes, edges, &reversed)
            .into_iter()
            .map(|level| level * stride)
            .collect();
        let mut graph = Self {
            real: nodes,
            rank: Vec::new(),
            left: real_dims.iter().map(|dims| dims.0).collect(),
            right: real_dims.iter().map(|dims| dims.1).collect(),
            thick: real_dims.iter().map(|dims| dims.2).collect(),
            up: vec![Vec::new(); nodes],
            down: vec![Vec::new(); nodes],
            ranks: Vec::new(),
            place: Vec::new(),
            chains: Vec::new(),
            anchor: (0..nodes).collect(),
        };
        for (at, edge) in edges.iter().enumerate() {
            if edge.from == edge.to {
                continue;
            }
            let (from, to) = layered(edge, reversed[at]);
            let span = rank[to] - rank[from];
            let label_rank = rank[from] + 2 * (span / 4) + 1;
            let columns = edge.label.as_deref().map_or(0, text_columns);
            let mut chain = vec![from];
            let mut label = None;
            for level in rank[from] + 1..rank[to] {
                let id = graph.left.len();
                let marked = edge.label.is_some() && level == label_rank;
                let (left, right, thick) = match (marked, spacing.horizontal) {
                    (false, _) => (0, 0, 0),
                    (true, false) => (0, columns + 1, 1),
                    (true, true) => (1, 0, columns + 2),
                };
                graph.left.push(left);
                graph.right.push(right);
                graph.thick.push(thick);
                graph.anchor.push(from);
                graph.up.push(Vec::new());
                graph.down.push(Vec::new());
                rank.push(level);
                label = if marked { Some(id) } else { label };
                chain.push(id);
            }
            chain.push(to);
            for pair in chain.windows(2) {
                graph.down[pair[0]].push(pair[1]);
                graph.up[pair[1]].push(pair[0]);
            }
            graph.chains.push(Chain {
                edge: at,
                nodes: chain,
                reversed: reversed[at],
                label,
                first_port: if reversed[at] { ports[from] } else { 0 },
                last_port: if reversed[at] { ports[to] } else { 0 },
            });
        }
        for chain in &graph.chains {
            let first = chain.nodes[0];
            let last = chain.nodes[chain.nodes.len() - 1];
            let target = if graph.down[first].len() > 1 {
                last
            } else {
                first
            };
            if chain.nodes.len() - 2 < stride {
                for &node in &chain.nodes[1..chain.nodes.len() - 1] {
                    graph.anchor[node] = target;
                }
            } else {
                for &node in &chain.nodes[1..chain.nodes.len() - 1] {
                    graph.anchor[node] = node;
                }
            }
        }
        graph.rank = rank;
        graph.order();
        graph.coordinates(spacing);
        graph
    }

    fn initial_order(&mut self) {
        let levels = self.rank.iter().max().map_or(0, |top| top + 1);
        self.ranks = vec![Vec::new(); levels];
        let mut seen = vec![false; self.rank.len()];
        for root in 0..self.real {
            let mut stack = vec![root];
            while let Some(node) = stack.pop() {
                if seen[node] {
                    continue;
                }
                seen[node] = true;
                self.ranks[self.rank[node]].push(node);
                stack.extend(self.down[node].iter().rev());
            }
        }
    }

    fn positions(&self) -> Vec<usize> {
        let mut pos = vec![0; self.rank.len()];
        for members in &self.ranks {
            for (at, &node) in members.iter().enumerate() {
                pos[node] = at;
            }
        }
        pos
    }

    fn crossings(&self) -> usize {
        let pos = self.positions();
        let mut total = 0;
        for members in &self.ranks {
            let segments: Vec<(usize, usize)> = members
                .iter()
                .flat_map(|&from| self.down[from].iter().map(move |&to| (from, to)))
                .map(|(from, to)| (pos[from], pos[to]))
                .collect();
            for (at, first) in segments.iter().enumerate() {
                for second in segments.iter().skip(at + 1) {
                    if (first.0 < second.0 && first.1 > second.1)
                        || (first.0 > second.0 && first.1 < second.1)
                    {
                        total += 1;
                    }
                }
            }
        }
        total
    }

    fn sweep(&mut self, downward: bool) {
        let levels = self.ranks.len();
        let order: Vec<usize> = if downward {
            (1..levels).collect()
        } else {
            (0..levels.saturating_sub(1)).rev().collect()
        };
        for level in order {
            let pos = self.positions();
            let neighbors = if downward { &self.up } else { &self.down };
            let mut keyed: Vec<(usize, i64, i64)> = self.ranks[level]
                .iter()
                .map(|&node| {
                    let list = &neighbors[node];
                    if list.is_empty() {
                        (node, i64::try_from(pos[node]).unwrap_or(0), 1)
                    } else {
                        let sum: usize = list.iter().map(|&other| pos[other]).sum();
                        (
                            node,
                            i64::try_from(sum).unwrap_or(0),
                            i64::try_from(list.len()).unwrap_or(1),
                        )
                    }
                })
                .collect();
            keyed.sort_by(|first, second| (first.1 * second.2).cmp(&(second.1 * first.2)));
            self.ranks[level] = keyed.into_iter().map(|entry| entry.0).collect();
        }
    }

    fn order(&mut self) {
        self.initial_order();
        let mut best = self.ranks.clone();
        let mut fewest = self.crossings();
        for _ in 0..SWEEPS {
            if fewest == 0 {
                break;
            }
            self.sweep(true);
            self.sweep(false);
            let found = self.crossings();
            if found < fewest {
                fewest = found;
                best.clone_from(&self.ranks);
            }
        }
        self.ranks = best;
    }

    fn separation(&self, first: usize, second: usize, spacing: &Spacing) -> i32 {
        let both_real = first < self.real && second < self.real;
        let gap = match (both_real, spacing.horizontal) {
            (true, false) => 3,
            (true, true) | (false, _) => 1,
        };
        self.right[first] + self.left[second] + 1 + gap
    }

    fn coordinates(&mut self, spacing: &Spacing) {
        self.place = vec![0; self.rank.len()];
        for level in 0..self.ranks.len() {
            let members = self.ranks[level].clone();
            let mut at = 0;
            for (index, &node) in members.iter().enumerate() {
                if let Some(&before) = index.checked_sub(1).and_then(|prior| members.get(prior)) {
                    at += self.separation(before, node, spacing);
                }
                self.place[node] = at;
            }
        }
        for _ in 0..SHIFT_PASSES {
            for level in 1..self.ranks.len() {
                self.shift(level, true, spacing);
            }
            for level in (0..self.ranks.len().saturating_sub(1)).rev() {
                self.shift(level, false, spacing);
            }
        }
        let low = (0..self.rank.len())
            .map(|node| self.place[node] - self.left[node])
            .min()
            .unwrap_or(0);
        for place in &mut self.place {
            *place -= low;
        }
    }

    fn shift(&mut self, level: usize, downward: bool, spacing: &Spacing) {
        let members = self.ranks[level].clone();
        let neighbors = if downward { &self.up } else { &self.down };
        let wanted: Vec<i32> = members
            .iter()
            .map(|&node| {
                let list = &neighbors[node];
                if node >= self.real && self.anchor[node] != node {
                    return self.place[self.anchor[node]];
                }
                if list.is_empty() {
                    return self.place[node];
                }
                let sum: i32 = list.iter().map(|&other| self.place[other]).sum();
                let length = count(list.len());
                (sum + length / 2).div_euclid(length)
            })
            .collect();
        let mut offsets = vec![0];
        for pair in members.windows(2) {
            let last = offsets.last().copied().unwrap_or(0);
            offsets.push(last + self.separation(pair[0], pair[1], spacing));
        }
        let mut blocks: Vec<(i64, i64)> = Vec::new();
        for (want, offset) in wanted.iter().zip(&offsets) {
            blocks.push((i64::from(want - offset), 1));
            while let [.., before, after] = blocks.as_slice() {
                if before.0 * after.1 <= after.0 * before.1 {
                    break;
                }
                let merged = (before.0 + after.0, before.1 + after.1);
                blocks.truncate(blocks.len() - 2);
                blocks.push(merged);
            }
        }
        let mut node = 0;
        for (sum, size) in blocks {
            let mean = i32::try_from(sum.div_euclid(size)).unwrap_or(0);
            for _ in 0..size {
                self.place[members[node]] = mean + offsets[node];
                node += 1;
            }
        }
    }
}
