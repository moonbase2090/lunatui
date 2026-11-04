// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::canvas::{Arms, Canvas, EAST, NORTH, Role, SOUTH, WEST, Weight, text_columns};
use super::flow_graph::count;
use super::seq_paint::{Painter, note_width};
use super::seq_parse::{Item, Place, Sequence};
use super::{MermaidError, fits};

const HEADER: i32 = 3;

fn item_rows(item: &Item) -> i32 {
    match item {
        Item::Message { from, to, text, .. } => {
            if from == to {
                3
            } else {
                2 + i32::from(!text.is_empty())
            }
        }
        Item::Note { lines, .. } => 1 + count(lines.len().max(1)) + 2,
        Item::Open { .. } | Item::Else { .. } | Item::Close => 2,
    }
}

fn frame_label(word: &str, text: &str) -> String {
    if text.is_empty() {
        word.to_owned()
    } else {
        format!("{word} [{text}]")
    }
}

struct Spans {
    gaps: Vec<i32>,
    needs: Vec<(usize, usize, i32)>,
    reach_left: i32,
    reach_right: i32,
}

fn constraints(sequence: &Sequence, halves: &[i32]) -> Spans {
    let count_of = sequence.participants.len();
    let mut spans = Spans {
        gaps: (1..count_of)
            .map(|at| halves[at - 1] + halves[at] + 3)
            .collect(),
        needs: Vec::new(),
        reach_left: 0,
        reach_right: 0,
    };
    for item in &sequence.items {
        match item {
            Item::Message { from, to, text, .. } => {
                let width = text_columns(text);
                if from == to {
                    if from + 1 < count_of {
                        spans.needs.push((*from, from + 1, width + 6));
                    } else {
                        spans.reach_right = spans.reach_right.max(width + 4);
                    }
                } else {
                    let (low, high) = (*from.min(to), *from.max(to));
                    spans.needs.push((low, high, (width + 4).max(5)));
                }
            }
            Item::Note {
                place,
                first,
                second,
                lines,
            } => {
                let width = note_width(lines);
                let half = width / 2;
                match place {
                    Place::Left if *first > 0 => spans.needs.push((first - 1, *first, width + 2)),
                    Place::Left => spans.reach_left = spans.reach_left.max(width + 1),
                    Place::Right if first + 1 < count_of => {
                        spans.needs.push((*first, first + 1, width + 3));
                    }
                    Place::Right => spans.reach_right = spans.reach_right.max(width + 2),
                    Place::Over if first == second => {
                        if *first > 0 {
                            spans.needs.push((first - 1, *first, half + 2));
                        } else {
                            spans.reach_left = spans.reach_left.max(half);
                        }
                        if first + 1 < count_of {
                            spans.needs.push((*first, first + 1, half + 2));
                        } else {
                            spans.reach_right = spans.reach_right.max(half);
                        }
                    }
                    Place::Over => spans.needs.push((*first, *second, width - 3)),
                }
            }
            _ => {}
        }
    }
    spans.needs.sort_by_key(|need| need.1 - need.0);
    for &(low, high, need) in &spans.needs {
        let have: i32 = spans.gaps[low..high].iter().sum();
        if have < need {
            spans.gaps[high - 1] += need - have;
        }
    }
    spans
}

pub(super) fn layout(sequence: &Sequence) -> Result<Canvas, MermaidError> {
    let people = &sequence.participants;
    if people.is_empty() {
        return Ok(Canvas::new(0, 0));
    }
    let boxes: Vec<i32> = people
        .iter()
        .map(|person| {
            let width = text_columns(&person.label) + 4;
            width + 1 - width % 2
        })
        .collect();
    let halves: Vec<i32> = boxes.iter().map(|width| (width - 1) / 2).collect();
    let spans = constraints(sequence, &halves);
    let mut centers = vec![0];
    for gap in &spans.gaps {
        centers.push(centers.last().copied().unwrap_or(0) + gap);
    }
    let mut depth = 0;
    let mut deepest = 0;
    let mut need_width = 0;
    for item in &sequence.items {
        match item {
            Item::Open { kind, text } => {
                need_width =
                    need_width.max(2 * depth + 6 + text_columns(&frame_label(kind.word(), text)));
                depth += 1;
                deepest = deepest.max(depth);
            }
            Item::Else { text } => {
                need_width =
                    need_width.max(2 * (depth - 1) + 6 + text_columns(&frame_label("else", text)));
            }
            Item::Close => depth -= 1,
            _ => {}
        }
    }
    let margin = if deepest > 0 { deepest + 1 } else { 0 };
    let leftmost = (0..people.len())
        .map(|at| centers[at] - halves[at])
        .min()
        .unwrap_or(0)
        .min(-spans.reach_left);
    let rightmost = (0..people.len())
        .map(|at| centers[at] + halves[at])
        .max()
        .unwrap_or(0)
        .max(centers[centers.len() - 1] + spans.reach_right);
    let shift = margin - leftmost;
    let width = (rightmost + shift + 1 + margin).max(need_width);
    let body: i32 = sequence.items.iter().map(item_rows).sum();
    let bottom = HEADER + body + 1;
    let height = bottom + HEADER;
    if !fits(width, height) {
        return Err(MermaidError::at(1, "the diagram is too large to draw"));
    }
    let centers: Vec<i32> = centers.iter().map(|center| center + shift).collect();
    let mut canvas = Canvas::new(width, height);
    for &center in &centers {
        for row in HEADER..bottom {
            let arms = Arms::new(NORTH | SOUTH, Weight::Dotted);
            canvas.line(center, row, arms, Role::Life);
        }
    }
    for (at, person) in people.iter().enumerate() {
        let left = centers[at] - halves[at];
        let round = person.actor;
        draw_box(&mut canvas, left, 0, boxes[at], &person.label, round);
        draw_box(&mut canvas, left, bottom, boxes[at], &person.label, round);
        let arms = Arms::new(SOUTH, Weight::Solid);
        canvas.line(centers[at], HEADER - 1, arms, Role::Node);
        canvas.line(
            centers[at],
            bottom,
            Arms::new(NORTH, Weight::Solid),
            Role::Node,
        );
    }
    let mut painter = Painter::new(&mut canvas, &centers, width);
    let mut row = HEADER;
    for item in &sequence.items {
        painter.item(item, row + 1);
        row += item_rows(item);
    }
    Ok(canvas)
}

fn draw_box(canvas: &mut Canvas, x: i32, y: i32, width: i32, label: &str, round: bool) {
    canvas.paint(x, y, width, 3, Role::Fill);
    for column in x + 1..x + width - 1 {
        canvas.line(column, y, Arms::border(EAST | WEST, round), Role::Node);
        canvas.line(column, y + 2, Arms::border(EAST | WEST, round), Role::Node);
    }
    let (right, low) = (x + width - 1, y + 2);
    canvas.line(x, y, Arms::border(EAST | SOUTH, round), Role::Node);
    canvas.line(right, y, Arms::border(WEST | SOUTH, round), Role::Node);
    canvas.line(x, low, Arms::border(EAST | NORTH, round), Role::Node);
    canvas.line(right, low, Arms::border(WEST | NORTH, round), Role::Node);
    canvas.line(x, y + 1, Arms::border(NORTH | SOUTH, round), Role::Node);
    canvas.line(right, y + 1, Arms::border(NORTH | SOUTH, round), Role::Node);
    let at = x + (width - text_columns(label)) / 2;
    canvas.text(at, y + 1, label, Role::Text);
}
