// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_text::markdown::{Alignment, Block, Inline, Options, parse};
use proptest::prelude::*;

fn text(value: &str) -> Inline {
    Inline::Text(value.into())
}

fn paragraph(source: &str) -> Vec<Inline> {
    match parse(source, Options::gfm()).blocks.into_iter().next() {
        Some(Block::Paragraph(content)) => content,
        other => panic!("expected a paragraph, got {other:?}"),
    }
}

#[test]
fn empty_input_has_no_blocks() {
    assert!(parse("", Options::gfm()).blocks.is_empty());
    assert!(parse("\n\n  \n", Options::gfm()).blocks.is_empty());
}

#[test]
fn heading_levels_and_closing_hashes() {
    let doc = parse("# One\n###### Six ##\n####### seven", Options::gfm());
    assert_eq!(doc.blocks.len(), 3);
    assert_eq!(
        doc.blocks[0],
        Block::Heading {
            level: 1,
            content: vec![text("One")]
        }
    );
    assert_eq!(
        doc.blocks[1],
        Block::Heading {
            level: 6,
            content: vec![text("Six")]
        }
    );
    assert!(matches!(doc.blocks[2], Block::Paragraph(_)));
}

#[test]
fn setext_heading_wins_over_thematic_break() {
    let doc = parse("Title\n---\n", Options::gfm());
    assert_eq!(
        doc.blocks,
        vec![Block::Heading {
            level: 2,
            content: vec![text("Title")]
        }]
    );
}

#[test]
fn emphasis_and_strong_nest() {
    assert_eq!(
        paragraph("*a **b** c*"),
        vec![Inline::Emphasis(vec![
            text("a "),
            Inline::Strong(vec![text("b")]),
            text(" c")
        ])]
    );
}

#[test]
fn intraword_underscore_is_literal() {
    assert_eq!(paragraph("snake_case_name"), vec![text("snake_case_name")]);
}

#[test]
fn code_span_strips_one_edge_space() {
    assert_eq!(paragraph("`` a ` b ``"), vec![Inline::Code("a ` b".into())]);
}

#[test]
fn escapes_and_entities_resolve() {
    assert_eq!(paragraph(r"\*x\* &amp; &copy;"), vec![text("*x* & \u{a9}")]);
}

#[test]
fn inline_link_with_title() {
    assert_eq!(
        paragraph("[a](/u \"t\")"),
        vec![Inline::Link {
            dest: "/u".into(),
            title: "t".into(),
            content: vec![text("a")]
        }]
    );
}

#[test]
fn reference_link_resolves_across_the_document() {
    let doc = parse("[x]\n\n[x]: /where \"T\"\n", Options::gfm());
    assert_eq!(doc.blocks.len(), 1);
    assert_eq!(
        doc.blocks[0],
        Block::Paragraph(vec![Inline::Link {
            dest: "/where".into(),
            title: "T".into(),
            content: vec![text("x")]
        }])
    );
}

#[test]
fn links_do_not_nest() {
    let content = paragraph("[a [b](/1) c](/2)");
    let links = content
        .iter()
        .filter(|node| matches!(node, Inline::Link { .. }))
        .count();
    assert_eq!(links, 1);
}

#[test]
fn image_keeps_alt_inlines() {
    assert_eq!(
        paragraph("![*a*](/i)"),
        vec![Inline::Image {
            dest: "/i".into(),
            title: String::new(),
            alt: vec![Inline::Emphasis(vec![text("a")])]
        }]
    );
}

#[test]
fn hard_and_soft_breaks() {
    assert_eq!(
        paragraph("a  \nb\nc"),
        vec![
            text("a"),
            Inline::HardBreak,
            text("b"),
            Inline::SoftBreak,
            text("c")
        ]
    );
    assert_eq!(
        paragraph("a\\\nb"),
        vec![text("a"), Inline::HardBreak, text("b")]
    );
}

#[test]
fn fenced_code_keeps_info_and_content() {
    let doc = parse("```rust ignore\nlet x = 1;\n\n```\n", Options::gfm());
    assert_eq!(
        doc.blocks,
        vec![Block::CodeBlock {
            info: "rust ignore".into(),
            code: "let x = 1;\n\n".into()
        }]
    );
}

#[test]
fn unclosed_fence_runs_to_the_end() {
    let doc = parse("```\na\nb", Options::gfm());
    assert_eq!(
        doc.blocks,
        vec![Block::CodeBlock {
            info: String::new(),
            code: "a\nb\n".into()
        }]
    );
}

#[test]
fn tight_and_loose_lists() {
    let tight = parse("- a\n- b\n", Options::gfm());
    let loose = parse("- a\n\n- b\n", Options::gfm());
    let Block::List(tight) = &tight.blocks[0] else {
        panic!("expected a list")
    };
    let Block::List(loose) = &loose.blocks[0] else {
        panic!("expected a list")
    };
    assert!(tight.tight);
    assert!(!loose.tight);
}

#[test]
fn ordered_list_keeps_its_start() {
    let doc = parse("7. a\n8. b\n", Options::gfm());
    let Block::List(list) = &doc.blocks[0] else {
        panic!("expected a list")
    };
    assert_eq!(list.start, Some(7));
    assert_eq!(list.items.len(), 2);
}

#[test]
fn nested_quote_and_list() {
    let doc = parse("> - a\n>   - b\n", Options::gfm());
    let Block::BlockQuote(inner) = &doc.blocks[0] else {
        panic!("expected a quote")
    };
    let Block::List(list) = &inner[0] else {
        panic!("expected a list")
    };
    assert!(matches!(list.items[0].blocks[1], Block::List(_)));
}

#[test]
fn table_alignment_and_ragged_rows() {
    let doc = parse("| a | b | c |\n|:-|:-:|-:|\n| 1 | 2 |\n", Options::gfm());
    let Block::Table(table) = &doc.blocks[0] else {
        panic!("expected a table")
    };
    assert_eq!(
        table.alignments,
        vec![Alignment::Left, Alignment::Center, Alignment::Right]
    );
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0].len(), 3);
    assert!(table.rows[0][2].is_empty());
}

#[test]
fn escaped_pipe_stays_in_its_cell() {
    let doc = parse("| a |\n|---|\n| x \\| y |\n", Options::gfm());
    let Block::Table(table) = &doc.blocks[0] else {
        panic!("expected a table")
    };
    assert_eq!(table.rows[0][0], vec![text("x | y")]);
}

#[test]
fn tables_are_off_in_commonmark_mode() {
    let doc = parse("| a |\n|---|\n| b |\n", Options::commonmark());
    assert!(matches!(doc.blocks[0], Block::Paragraph(_)));
}

#[test]
fn task_items_report_their_state() {
    let doc = parse("- [ ] a\n- [x] b\n- c\n", Options::gfm());
    let Block::List(list) = &doc.blocks[0] else {
        panic!("expected a list")
    };
    let states: Vec<_> = list.items.iter().map(|item| item.checked).collect();
    assert_eq!(states, vec![Some(false), Some(true), None]);
}

#[test]
fn strikethrough_and_autolink() {
    assert_eq!(
        paragraph("~~gone~~"),
        vec![Inline::Strikethrough(vec![text("gone")])]
    );
    assert_eq!(
        paragraph("see www.example.com."),
        vec![
            text("see "),
            Inline::Link {
                dest: "http://www.example.com".into(),
                title: String::new(),
                content: vec![text("www.example.com")]
            },
            text(".")
        ]
    );
    assert_eq!(
        parse("~~x~~", Options::commonmark()).blocks,
        vec![Block::Paragraph(vec![text("~~x~~")])]
    );
}

#[test]
fn html_block_is_kept_verbatim() {
    let doc = parse("<div>\n*not emphasis*\n</div>\n\nafter", Options::gfm());
    assert_eq!(
        doc.blocks[0],
        Block::Html("<div>\n*not emphasis*\n</div>".into())
    );
    assert!(matches!(doc.blocks[1], Block::Paragraph(_)));
}

#[test]
fn crlf_and_nul_are_normalised() {
    assert_eq!(
        parse("a\r\nb\r\n", Options::gfm()),
        parse("a\nb\n", Options::gfm())
    );
    assert_eq!(paragraph("a\0b"), vec![text("a\u{fffd}b")]);
}

#[test]
fn deep_nesting_is_bounded() {
    let quotes = ">".repeat(10_000);
    let lists = "- ".repeat(10_000);
    let _ = parse(&format!("{quotes} x"), Options::gfm());
    let _ = parse(&format!("{lists}x"), Options::gfm());
    let _ = parse(&"[".repeat(20_000), Options::gfm());
    let _ = parse(&"*a ".repeat(20_000), Options::gfm());
    let _ = parse(&"`a".repeat(20_000), Options::gfm());
}

const FRAGMENTS: &[&str] = &[
    "*",
    "**",
    "_",
    "__",
    "~~",
    "`",
    "``",
    "[",
    "]",
    "(",
    ")",
    "![",
    "<",
    ">",
    "&",
    "&amp;",
    "\\",
    "\n",
    "\n\n",
    "  \n",
    "\t",
    " ",
    "- ",
    "1. ",
    "# ",
    "> ",
    "---",
    "===",
    "|",
    "|-|",
    ":-:",
    "```",
    "~~~",
    "<div>",
    "</div>",
    "<!--",
    "-->",
    "www.a.b",
    "a@b.c",
    "http://x.y",
    "[x]: /u",
    "é",
    "ẞ",
    "日本",
    "🙂",
    "\u{200d}",
    "\u{301}",
    "x",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arbitrary_text_never_panics(source in ".{0,200}") {
        let _ = parse(&source, Options::gfm());
        let _ = parse(&source, Options::commonmark());
    }

    #[test]
    fn markdown_shaped_text_never_panics(
        parts in proptest::collection::vec(proptest::sample::select(FRAGMENTS), 0..60)
    ) {
        let source: String = parts.concat();
        let _ = parse(&source, Options::gfm());
        let _ = parse(&source, Options::commonmark());
    }

    #[test]
    fn parsing_is_deterministic(
        parts in proptest::collection::vec(proptest::sample::select(FRAGMENTS), 0..40)
    ) {
        let source: String = parts.concat();
        prop_assert_eq!(parse(&source, Options::gfm()), parse(&source, Options::gfm()));
    }
}
