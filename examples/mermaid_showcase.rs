// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, MermaidDiagram, MermaidError, MermaidGlyphs, MermaidView, Paragraph,
    RenderCx, Widget,
};
use std::sync::OnceLock;

const FLOW: &str = "flowchart TD
  A([Commit]) --> B[Build]
  B --> C{Tests pass?}
  C -->|yes| D((Ship))
  C -->|no| E[Inspect logs]
  E -. retry .-> B";
const SEQUENCE: &str = "sequenceDiagram
  autonumber
  actor Dev as Developer
  participant API
  participant Store
  Dev->>API: Publish release
  API->>Store: Save artifact
  Store-->>API: Stored
  alt checks passed
    API-->>Dev: Release ready
  else retry needed
    API-xDev: Publish failed
  end
  Note over API,Store: Artifact and metadata saved";
static DIAGRAMS: OnceLock<[Result<MermaidDiagram, MermaidError>; 3]> = OnceLock::new();

pub(super) fn render(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    if area.width < 60 || area.height < 16 {
        return;
    }
    let index = if tick < 100 {
        0
    } else if tick < 170 {
        1
    } else {
        2
    };
    let horizontal = FLOW.replacen("TD", "LR", 1);
    let diagrams = DIAGRAMS.get_or_init(|| {
        [
            MermaidDiagram::parse(FLOW),
            MermaidDiagram::parse(&horizontal),
            MermaidDiagram::parse(SEQUENCE),
        ]
    });
    let mut cx = RenderCx::new(grid, clusters, theme);
    let base = Style::new()
        .fg(cx.token(Token::TextPrimary))
        .bg(cx.token(Token::Surface0));
    let accent = base
        .fg(cx.token(Token::Accent))
        .with_attrs(Attrs::new().set_bold(true));
    let dim = base.fg(cx.token(Token::TextMuted));
    let blank = (0..area.height)
        .map(|_| " ".repeat(usize::from(area.width)))
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    Paragraph::new(blank).style(base).render(area, &mut cx);
    let content = document_panels(&mut cx, area, index, &horizontal, accent, dim);
    if let Ok(diagram) = &diagrams[index] {
        mermaid_preview(&mut cx, area, content, diagram, index, tick);
    }
    let note = if tick >= 260 {
        "ASCII fallback · labels stay readable"
    } else if tick >= 220 {
        "Scrolled to the release result and note"
    } else if index == 2 {
        "Sequence messages · replies, failure and conditional frame"
    } else if index == 1 {
        "Horizontal layout · pan to the shipping decision"
    } else {
        "Flowchart · decision, retry cycle and labeled branches"
    };
    Paragraph::new(note).style(accent).render(
        Rect::new(area.x + 2, area.y + area.height - 2, area.width - 4, 1),
        &mut cx,
    );
}

fn mermaid_preview(
    cx: &mut RenderCx<'_>,
    area: Rect,
    content: Rect,
    diagram: &MermaidDiagram,
    index: usize,
    tick: u32,
) {
    let dim = Style::new()
        .fg(cx.token(Token::TextMuted))
        .bg(cx.token(Token::Surface0));
    let size = diagram.size();
    let viewport_width = if index == 1 {
        content.width.min(48)
    } else {
        content.width.min(size.width)
    };
    let bounds = Rect::new(
        content.x + (content.width - viewport_width) / 2,
        content.y,
        viewport_width,
        content.height,
    );
    let view = MermaidView::new(diagram);
    let (max_x, max_y) = view.max_scroll(bounds);
    let scroll_x = if index == 1 && tick >= 135 { max_x } else { 0 };
    let scroll_y = if index == 2 && tick >= 220 { max_y } else { 0 };
    let ascii = tick >= 260;
    view.scroll(scroll_x, scroll_y)
        .glyphs(if ascii {
            MermaidGlyphs::Ascii
        } else {
            MermaidGlyphs::Unicode
        })
        .render(bounds, cx);
    Paragraph::new(format!(
        "Preview   {} × {} cells   ·   scroll {},{}   ·   {}",
        size.width,
        size.height,
        scroll_x,
        scroll_y,
        if ascii {
            "ASCII lines"
        } else {
            "Unicode lines"
        }
    ))
    .style(dim)
    .render(
        Rect::new(area.x + 2, area.y + area.height - 3, area.width - 4, 1),
        cx,
    );
}

fn document_panels(
    cx: &mut RenderCx<'_>,
    area: Rect,
    index: usize,
    horizontal: &str,
    accent: Style,
    dim: Style,
) -> Rect {
    Paragraph::new("ORBIT DOCS   /   architecture")
        .style(accent)
        .render(Rect::new(area.x + 2, area.y + 1, area.width - 4, 1), cx);
    let name = if index == 2 {
        "release-sequence.mmd"
    } else {
        "delivery-flow.mmd"
    };
    Paragraph::new(format!(
        "docs/diagrams/{name}    ·    source and terminal preview"
    ))
    .style(dim)
    .render(Rect::new(area.x + 2, area.y + 2, area.width - 4, 1), cx);
    let source = Rect::new(area.x + 1, area.y + 4, 37, area.height - 8);
    let preview = Rect::new(area.x + 39, area.y + 4, area.width - 40, area.height - 8);
    Block::bordered()
        .style(dim.fg(cx.token(Token::TextPrimary)))
        .border_type(BorderType::Rounded)
        .title(" Mermaid source ")
        .border_style(dim)
        .render(source, cx);
    Paragraph::new(if index == 2 {
        SEQUENCE
    } else if index == 1 {
        horizontal
    } else {
        FLOW
    })
    .style(dim)
    .render(
        Rect::new(
            source.x + 2,
            source.y + 2,
            source.width - 4,
            source.height - 3,
        ),
        cx,
    );
    let title = if index == 2 {
        " Sequence preview "
    } else if index == 1 {
        " Flowchart / LR "
    } else {
        " Flowchart / TD "
    };
    Block::bordered()
        .style(dim.fg(cx.token(Token::TextPrimary)))
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(dim)
        .render(preview, cx);
    Rect::new(
        preview.x + 2,
        preview.y + 1,
        preview.width - 4,
        preview.height - 2,
    )
}
