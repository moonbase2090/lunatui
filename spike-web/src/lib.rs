//! Spike: drive a Lunatui `App` without `run()` and emit the ANSI bytes xterm.js would show.
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{App, ControlFlow};
use lunatui_term::{ColorMode, Encoder, Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget};
use wasm_bindgen::prelude::wasm_bindgen;

struct Demo {
    presses: u32,
}

impl App for Demo {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Char(' ') {
                self.presses += 1;
            }
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let block = Block::bordered().title(" spike ").style(style);
        block.render(area, &mut cx);
        Paragraph::new(format!("space pressed {} times", self.presses))
            .style(style)
            .render(block.inner(area), &mut cx);
    }
}

/// One browser-side session: app, previous frame, encoder.
#[wasm_bindgen]
pub struct Session {
    app: Demo,
    prev: Grid,
    encoder: Encoder,
    area: Rect,
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            app: Demo { presses: 0 },
            prev: Grid::new(width, height),
            encoder: Encoder::new(),
            area: Rect::new(0, 0, width, height),
        }
    }

    /// Feeds one printable key and returns the ANSI bytes for the next frame.
    pub fn key(&mut self, ch: char) -> Vec<u8> {
        self.app.update(&Event::Key(lunatui_term::KeyEvent::press(KeyCode::Char(ch))));
        self.frame()
    }

    /// Renders and diffs against the last frame.
    pub fn frame(&mut self) -> Vec<u8> {
        let mut next = Grid::new(self.area.width, self.area.height);
        let mut clusters = ClusterTable::new();
        self.app.render(&mut next, self.area, &mut clusters);
        let bytes = self
            .encoder
            .encode_diff(&self.prev, &next, &clusters, ColorMode::TrueColor)
            .to_vec();
        self.prev = next;
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::Session;

    #[test]
    fn first_frame_has_text_then_diff_is_small() {
        let mut s = Session::new(30, 5);
        let first = String::from_utf8_lossy(&s.frame()).into_owned();
        assert!(first.contains("space pressed 0 times"));
        let second = s.key(' ');
        let text = String::from_utf8_lossy(&second).into_owned();
        assert!(text.contains('1'));
        assert!(second.len() < 60, "diff was {} bytes", second.len());
    }
}
