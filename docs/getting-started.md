# Getting started (v0.1.15 preview)

lunatui **v0.1.15** is the current preview: a packed cell grid, an input and session layer, theme tokens, and the widget set. APIs can still change before 1.0.

## Add the dependency

From a new binary crate:

```sh
cargo add lunatui --features app
cargo add lunatui-core lunatui-term lunatui-theme lunatui-widgets
```

`app` turns on `lunatui::run`. The other crates are the grid, events, theme tokens, and widgets. Templates that use a sparkline, a form, a popover, or the layout engine also need:

```sh
cargo add lunatui-widgets --features all-widgets
cargo add lunatui-layout
```

## A first app

This program draws one bordered box and quits when you press `q`. It is under 30 lines.

```rust
use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::Event;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget};

struct Hello;

impl App for Hello {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) { ControlFlow::Break } else { ControlFlow::Continue }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        let text = Style::new().fg(cx.token(Token::TextPrimary)).bg(cx.token(Token::Surface0));
        let accent = Style::new().fg(cx.token(Token::Accent)).bg(cx.token(Token::Surface0));
        let block = Block::bordered().title(" hello ").style(text)
            .border_style(accent).title_style(accent);
        block.render(area, &mut cx);
        Paragraph::new("Hello. Press q to quit.").style(text).render(block.inner(area), &mut cx);
    }
}

fn main() -> std::io::Result<()> {
    run(Hello, RunOptions::new(48, 10))
}
```

`cargo run` opens the screen. Press `q` to leave it. In this repository the same screen is `cargo run --example template_minimal` (that copy calls `lunatui_runtime::run`, which is what `lunatui::run` forwards to). Add `-- --snapshot` to print one frame as text.

## Where to go next

Copy-paste templates live in [`examples/templates`](../examples/templates) and are indexed in [the examples list](../examples/README.md):

| Template | Command | What it shows |
|---|---|---|
| Minimal | `cargo run --example template_minimal` | A titled box. Press `q` to quit. |
| List | `cargo run --example template_list` | A list. Up and down move the selection. |
| Form | `cargo run --example template_form` | Two fields. Tab moves focus. |
| Dashboard | `cargo run --example template_dashboard` | A sparkline, a gauge, and a table that tick. |
| Popover | `cargo run --example template_popover` | A panel beside an anchor. Enter toggles it. |
| Layout | `cargo run --example template_layout` | Header, sidebar, body, and a status bar. |

Each template takes `--snapshot` to print one frame. Those frames are on the site quickstart page, `site/quickstart.html`.

- [Architecture overview](architecture.md)
- [Widget gallery](../README.md#widget-gallery)
- [Examples](../examples/README.md)
- [Porting notes](porting-from-ratatui.md)
