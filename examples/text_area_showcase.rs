// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{Event, KeyCode, KeyEvent, Modifiers};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{Block, Paragraph, RenderCx, TextArea, TextPosition, Widget, WidgetState};

pub const DOC: &str = "\
# Release notes

## Multi-line editor

The editor keeps your document in one place.
Select a passage, find a phrase, then keep writing.
Every edit can be undone without losing your place.

## Navigation

Arrow keys move across graphemes and line breaks.
Shift extends a selection from its original anchor.
The gutter follows the document as the editor scrolls.

## Find and revise

Search highlights the next matching phrase.
The editor keeps the current match in view.
Paste inserts a complete passage as one undo step.

## Keyboard

Emacs bindings are ready when the editor opens.
Vim mode adds normal and insert keymaps.

## Before publishing

- [ ] Final review and release notes
- [ ] Read the notes once more before publishing.
";

fn editor_at(tick: u32) -> TextArea {
    let mut editor = TextArea::new(DOC).line_numbers(true);
    editor.set_widget_state(WidgetState::Focused);
    editor.move_to(
        TextPosition {
            line: 27,
            grapheme: 6,
        },
        false,
    );
    editor.handle_event(&Event::Key(
        KeyEvent::press(KeyCode::End).modifiers(Modifiers::new().shift()),
    ));
    if tick >= 90 {
        editor.move_to(
            TextPosition {
                line: 0,
                grapheme: 0,
            },
            false,
        );
        editor.start_search();
        editor.handle_event(&Event::Key(
            KeyEvent::press(KeyCode::Char('e')).text("editor"),
        ));
    }
    if tick >= 180 {
        editor.cancel_search();
        editor.move_to(
            TextPosition {
                line: 27,
                grapheme: 3,
            },
            false,
        );
        editor.move_to(
            TextPosition {
                line: 27,
                grapheme: 4,
            },
            true,
        );
        editor.handle_event(&Event::Key(KeyEvent::press(KeyCode::Char('x')).text("x")));
    }
    editor
}

pub fn render(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let ground = Style::new()
        .fg(theme.resolve(Token::TextPrimary))
        .bg(theme.resolve(Token::Surface0));
    let dim = Style::new()
        .fg(theme.resolve(Token::TextMuted))
        .bg(theme.resolve(Token::Surface1));
    let strong = Style::new()
        .fg(theme.resolve(Token::TextStrong))
        .bg(theme.resolve(Token::Surface1));
    let blank_rows = vec![" ".repeat(usize::from(area.width)); usize::from(area.height)].join("\n");
    Paragraph::new(blank_rows)
        .style(ground)
        .render(area, &mut cx);
    Paragraph::new(
        "  LUNATUI    File   Edit   Find                              notes / release-notes.md",
    )
    .style(strong)
    .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let body = Rect::new(
        area.x,
        area.y.saturating_add(2),
        area.width,
        area.height.saturating_sub(4),
    );
    let side_width = 25.min(body.width / 3);
    let sidebar = Rect::new(body.x, body.y, side_width, body.height);
    let side = Block::bordered()
        .title(" PROJECT ")
        .style(ground)
        .border_style(dim);
    let side_inner = side.inner(sidebar);
    side.render(sidebar, &mut cx);
    Paragraph::new(
        "\
 notes/
   release-notes.md
   getting-started.md
   architecture.md

 src/
   main.rs
 Cargo.toml
 README.md

 OUTLINE

 Multi-line editor
 Navigation
 Find and revise
 Keyboard
 Before publishing

 OPEN DOCUMENT

 Markdown
 UTF-8
 LF line endings",
    )
    .style(ground)
    .render(side_inner, &mut cx);
    let pane = Rect::new(
        body.x + side_width,
        body.y,
        body.width.saturating_sub(side_width),
        body.height,
    );
    let editor = editor_at(tick);
    let edited = editor.text() != DOC;
    let block = Block::bordered()
        .title(if edited {
            " release-notes.md [+] "
        } else {
            " release-notes.md "
        })
        .style(ground)
        .border_style(dim);
    let inner = block.inner(pane);
    block.render(pane, &mut cx);
    editor.render(inner, &mut cx);
    let detail = if let Some(query) = editor.search_query() {
        format!("Find: {query}    |    Enter accept    Esc cancel")
    } else if editor.selection().is_some() {
        "Selection    |    Shift+arrows extend    Type to replace".to_owned()
    } else {
        "Modified    |    Ctrl+Z undo    Ctrl+S find".to_owned()
    };
    let cursor = editor.cursor();
    Paragraph::new(format!(
        "  {detail}    Ln {}, Col {}",
        cursor.line + 1,
        cursor.grapheme + 1
    ))
    .style(strong)
    .render(
        Rect::new(
            area.x,
            area.y + area.height.saturating_sub(2),
            area.width,
            1,
        ),
        &mut cx,
    );
    Paragraph::new("  EMACS    Markdown    UTF-8                          Shift+End select    Ctrl+S find    Ctrl+Z undo")
        .style(dim).render(Rect::new(area.x, area.y + area.height.saturating_sub(1), area.width, 1), &mut cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_replaces_the_longer_selection_status() {
        let theme = Theme::builtin(lunatui_theme::Variant::Dark);
        let mut grid = Grid::new(120, 36);
        let mut clusters = ClusterTable::new();
        render(
            &mut grid,
            &mut clusters,
            &theme,
            Rect::new(0, 0, 120, 36),
            0,
        );
        render(
            &mut grid,
            &mut clusters,
            &theme,
            Rect::new(0, 0, 120, 36),
            90,
        );
        assert_eq!(
            grid.row_display(34, &clusters).trim_end(),
            "  Find: editor    |    Enter accept    Esc cancel    Ln 3, Col 15"
        );
    }

    #[test]
    fn recording_holds_selection_search_and_an_undoable_edit() {
        for tick in [0, 89] {
            let editor = editor_at(tick);
            assert_eq!(
                editor
                    .selection()
                    .map(|selection| (selection.anchor, selection.head)),
                Some((
                    TextPosition {
                        line: 27,
                        grapheme: 6
                    },
                    TextPosition {
                        line: 27,
                        grapheme: 36
                    }
                ))
            );
            assert_eq!(editor.search_query(), None);
        }
        for tick in [90, 179] {
            let editor = editor_at(tick);
            assert_eq!(editor.search_query(), Some("editor"));
            assert_eq!(
                editor.cursor(),
                TextPosition {
                    line: 2,
                    grapheme: 14
                }
            );
            assert_eq!(editor.selection(), None);
        }
        for tick in [180, 269] {
            let mut editor = editor_at(tick);
            assert!(
                editor
                    .text()
                    .contains("- [x] Final review and release notes")
            );
            assert_eq!(editor.search_query(), None);
            assert_eq!(editor.selection(), None);
            editor.undo();
            assert_eq!(editor.text(), DOC);
        }
    }
}
