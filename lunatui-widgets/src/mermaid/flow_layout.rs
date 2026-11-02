// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::canvas::{Canvas, Role, text_columns};
use super::flow_draw::{Mapper, NodeBox, draw_node, draw_path, expand};
use super::flow_graph::{Chain, Graph, Spacing, count, reversed_edges};
use super::flow_parse::{Flow, Shape};
use super::{MermaidError, fits};

struct Group {
    stubs: Vec<i32>,
    drops: Vec<i32>,
    hops: Vec<(usize, usize)>,
    target: (usize, i32),
    low: i32,
    high: i32,
    row: usize,
}

impl Group {
    fn clashes(&self, other: &Self) -> bool {
        if self.row == other.row {
            return self.low <= other.high && other.low <= self.high;
        }
        let meets = |stubs: &[i32], drops: &[i32]| stubs.iter().any(|stub| drops.contains(stub));
        (meets(&self.stubs, &other.drops) && self.row >= other.row)
            || (meets(&other.stubs, &self.drops) && other.row >= self.row)
    }

    fn fan_in(&self) -> bool {
        self.hops.len() == 1 || self.stubs.len() == self.hops.len()
    }

    fn absorb(&mut self, other: Self) {
        self.stubs.extend(other.stubs);
        self.drops.extend(other.drops);
        self.hops.extend(other.hops);
        self.low = self.low.min(other.low);
        self.high = self.high.max(other.high);
    }
}

fn columns(graph: &Graph, chain: &Chain) -> Vec<i32> {
    let last = chain.nodes.len() - 1;
    chain
        .nodes
        .iter()
        .enumerate()
        .map(|(at, &node)| {
            let port = match at {
                0 => chain.first_port,
                _ if at == last => chain.last_port,
                _ => 0,
            };
            graph.place[node] + port
        })
        .collect()
}

struct Tracks {
    hop_row: Vec<Vec<Option<usize>>>,
    rows: Vec<usize>,
}

fn assign_tracks(graph: &Graph, cols: &[Vec<i32>]) -> Tracks {
    let levels = graph.ranks.len();
    let mut hop_row: Vec<Vec<Option<usize>>> = graph
        .chains
        .iter()
        .map(|chain| vec![None; chain.nodes.len() - 1])
        .collect();
    let mut by_level: Vec<Vec<Group>> = (0..levels).map(|_| Vec::new()).collect();
    let mut owners: Vec<Vec<(usize, i32)>> = (0..levels).map(|_| Vec::new()).collect();
    for (at, chain) in graph.chains.iter().enumerate() {
        for (index, pair) in chain.nodes.windows(2).enumerate() {
            let level = graph.rank[pair[0]];
            let (source, drop) = (cols[at][index], cols[at][index + 1]);
            let key = (pair[0], source);
            let found = owners[level].iter().position(|owner| *owner == key);
            let slot = found.unwrap_or_else(|| {
                owners[level].push(key);
                by_level[level].push(Group {
                    stubs: vec![source],
                    drops: Vec::new(),
                    hops: Vec::new(),
                    target: (pair[1], drop),
                    low: source,
                    high: source,
                    row: 0,
                });
                owners[level].len() - 1
            });
            let group = &mut by_level[level][slot];
            group.drops.push(drop);
            group.hops.push((at, index));
            group.low = group.low.min(drop);
            group.high = group.high.max(drop);
        }
    }
    let mut rows = vec![0; levels];
    for (level, groups) in by_level.into_iter().enumerate() {
        let mut joined: Vec<Group> = Vec::new();
        for group in groups {
            let sibling = (group.hops.len() == 1)
                .then(|| {
                    joined
                        .iter()
                        .position(|other| other.target == group.target && other.fan_in())
                })
                .flatten();
            match sibling {
                Some(at) => joined[at].absorb(group),
                None => joined.push(group),
            }
        }
        let mut bent: Vec<Group> = joined
            .into_iter()
            .filter(|group| group.low != group.high)
            .collect();
        bent.sort_by_key(|group| (group.low, group.high, group.target.1));
        let mut placed: Vec<Group> = Vec::new();
        for mut group in bent {
            let used = placed.iter().map(|other| other.row + 1).max().unwrap_or(0);
            group.row = (0..=used)
                .find(|&row| {
                    group.row = row;
                    !placed.iter().any(|other| group.clashes(other))
                })
                .unwrap_or(used);
            for &(at, index) in &group.hops {
                if cols[at][index] != cols[at][index + 1] {
                    hop_row[at][index] = Some(group.row);
                }
            }
            placed.push(group);
        }
        rows[level] = placed.iter().map(|group| group.row + 1).max().unwrap_or(0);
    }
    Tracks { hop_row, rows }
}

fn lines_of(flow: &Flow, at: usize) -> Vec<String> {
    let mut lines = flow
        .nodes
        .get(at)
        .map(|node| node.lines.clone())
        .unwrap_or_default();
    for edge in flow
        .edges
        .iter()
        .filter(|edge| edge.from == at && edge.to == at)
    {
        let mark = edge
            .label
            .as_ref()
            .map_or_else(|| String::from("↺"), |label| format!("↺ {label}"));
        match lines.last_mut() {
            Some(last) => {
                last.push(' ');
                last.push_str(&mark);
            }
            None => lines.push(mark),
        }
    }
    lines
}

struct Plan {
    graph: Graph,
    cols: Vec<Vec<i32>>,
    tracks: Tracks,
    start: Vec<i32>,
    stub: Vec<i32>,
    thickness: Vec<i32>,
    low: i32,
    total: i32,
    across: i32,
}

fn plan(graph: Graph) -> Plan {
    let levels = graph.ranks.len();
    let thickness: Vec<i32> = graph
        .ranks
        .iter()
        .map(|members| {
            members
                .iter()
                .map(|&node| graph.thick[node])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let has_real: Vec<bool> = graph
        .ranks
        .iter()
        .map(|members| members.iter().any(|&node| node < graph.real))
        .collect();
    let cols: Vec<Vec<i32>> = graph
        .chains
        .iter()
        .map(|chain| columns(&graph, chain))
        .collect();
    let tracks = assign_tracks(&graph, &cols);
    let mut start = vec![0_i32; levels];
    let mut stub = vec![0_i32; levels];
    let mut total = 0;
    for level in 0..levels {
        stub[level] = i32::from(has_real[level]);
        start[level] = total;
        let arrow = i32::from(has_real.get(level + 1).copied().unwrap_or(false));
        total += thickness[level];
        if level + 1 < levels {
            total += stub[level] + count(tracks.rows[level]) + arrow;
        }
    }
    let nodes = 0..graph.rank.len();
    let low = nodes
        .clone()
        .map(|node| graph.place[node] - graph.left[node])
        .min()
        .unwrap_or(0);
    let high = nodes
        .map(|node| graph.place[node] + graph.right[node])
        .max()
        .unwrap_or(-1);
    Plan {
        graph,
        cols,
        tracks,
        start,
        stub,
        thickness,
        low,
        total,
        across: high - low + 1,
    }
}

fn draw_edges(canvas: &mut Canvas, flow: &Flow, plan: &Plan, mapper: &Mapper) {
    let graph = &plan.graph;
    for (at, chain) in graph.chains.iter().enumerate() {
        let Some(edge) = flow.edges.get(chain.edge) else {
            continue;
        };
        let (first, last) = (chain.nodes[0], chain.nodes[chain.nodes.len() - 1]);
        let head_end = edge.head && !chain.reversed;
        let head_start = edge.head && chain.reversed;
        let col = |index: usize| plan.cols[at][index] - plan.low;
        let begin = plan.start[graph.rank[first]] + graph.thick[first] - 1;
        let mut corners = vec![(begin, col(0))];
        for (index, pair) in chain.nodes.windows(2).enumerate() {
            if let Some(row) = plan.tracks.hop_row[at][index] {
                let level = graph.rank[pair[0]];
                let along =
                    plan.start[level] + plan.thickness[level] + plan.stub[level] + count(row);
                corners.push((along, col(index)));
                corners.push((along, col(index + 1)));
            }
        }
        let landing = plan.start[graph.rank[last]];
        corners.push((
            if head_end { landing - 1 } else { landing },
            col(chain.nodes.len() - 1),
        ));
        let cells: Vec<(i32, i32)> = expand(&corners)
            .into_iter()
            .map(|(along, across)| mapper.point(along, across))
            .collect();
        draw_path(canvas, &cells, edge.weight, head_end, head_start);
    }
}

fn draw_labels(canvas: &mut Canvas, flow: &Flow, plan: &Plan, mapper: &Mapper) {
    let graph = &plan.graph;
    for chain in &graph.chains {
        let (Some(node), Some(edge)) = (chain.label, flow.edges.get(chain.edge)) else {
            continue;
        };
        let Some(label) = edge.label.as_deref() else {
            continue;
        };
        let begin = plan.start[graph.rank[node]];
        let line = graph.place[node] - plan.low;
        let (x, y) = if mapper.horizontal {
            let first = if mapper.flipped {
                begin + text_columns(label)
            } else {
                begin + 1
            };
            mapper.point(first, line - 1)
        } else {
            mapper.point(begin, line + 2)
        };
        canvas.text(x, y, label, Role::Label);
    }
}

fn port_offsets(boxes: &[NodeBox], horizontal: bool) -> Vec<i32> {
    boxes
        .iter()
        .map(|node| {
            let diamond = node.shape == Shape::Diamond;
            let extent = if horizontal { node.height } else { node.width };
            let reach = (extent - 1) / 2 - 1 - i32::from(diamond && !horizontal);
            if diamond && horizontal {
                0
            } else {
                reach.clamp(0, 2)
            }
        })
        .collect()
}

pub(super) fn layout(flow: &Flow) -> Result<Canvas, MermaidError> {
    let horizontal = flow.direction.horizontal();
    let reversed = reversed_edges(flow.nodes.len(), &flow.edges);
    let mut roomy = vec![false; flow.nodes.len()];
    for (edge, _) in flow
        .edges
        .iter()
        .zip(&reversed)
        .filter(|(_, flipped)| **flipped)
    {
        roomy[edge.from] = true;
        roomy[edge.to] = true;
    }
    let boxes: Vec<NodeBox> = flow
        .nodes
        .iter()
        .enumerate()
        .map(|(at, node)| NodeBox::new(lines_of(flow, at), node.shape, horizontal, roomy[at]))
        .collect();
    let dims: Vec<(i32, i32, i32)> = boxes
        .iter()
        .map(|node| {
            if horizontal {
                ((node.height - 1) / 2, (node.height - 1) / 2, node.width)
            } else {
                ((node.width - 1) / 2, (node.width - 1) / 2, node.height)
            }
        })
        .collect();
    let ports = port_offsets(&boxes, horizontal);
    let plan = plan(Graph::build(
        &flow.edges,
        &dims,
        &ports,
        &Spacing { horizontal },
    ));
    let (width, height) = if horizontal {
        (plan.total, plan.across)
    } else {
        (plan.across, plan.total)
    };
    if !fits(width, height) {
        return Err(MermaidError::at(1, "the diagram is too large to draw"));
    }
    let mapper = Mapper {
        horizontal,
        flipped: flow.direction.flipped(),
        total: plan.total,
    };
    let mut canvas = Canvas::new(width, height);
    for (index, node) in boxes.iter().enumerate() {
        let (length, breadth) = if horizontal {
            (node.width, node.height)
        } else {
            (node.height, node.width)
        };
        let across = plan.graph.place[index] - plan.low - plan.graph.left[index];
        let start = plan.start[plan.graph.rank[index]];
        let (x, y, ..) = mapper.rect(start, length, across, breadth);
        draw_node(&mut canvas, x, y, node);
    }
    draw_edges(&mut canvas, flow, &plan, &mapper);
    draw_labels(&mut canvas, flow, &plan, &mapper);
    Ok(canvas)
}
