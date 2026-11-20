// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use fontdue::{Font, FontSettings};
use tiny_skia::{
    Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform,
};

const SCALE: f32 = 2.0;
const PANEL_WIDTH: f32 = 560.0;
const PANEL_HEIGHT: f32 = 1210.0;
const GAP: f32 = 24.0;
const PAD: f32 = 24.0;

type Rgb = (u8, u8, u8);

const BACKGROUND: Rgb = (0x14, 0x16, 0x1a);
const SURFACE: Rgb = (0x1b, 0x1e, 0x24);
const MENU_SURFACE: Rgb = (0x20, 0x24, 0x2b);
const HAIRLINE: Rgb = (0x2c, 0x31, 0x3b);
const TEXT: Rgb = (0xe6, 0xe9, 0xee);
const STRONG: Rgb = (0xff, 0xff, 0xff);
const MUTED: Rgb = (0xa3, 0xab, 0xb8);
const SELECT: Rgb = (0x25, 0x2a, 0x33);
const FOCUS: Rgb = (0x5a, 0xa2, 0xff);
const ON_FOCUS: Rgb = (0x0b, 0x12, 0x20);
const GREEN: Rgb = (0x4c, 0xc9, 0x8a);
const AMBER: Rgb = (0xf2, 0xb8, 0x4b);
const RED: Rgb = (0xff, 0x7a, 0x6b);

type Failure = Box<dyn Error>;

struct Family {
    title: &'static str,
    slug: &'static str,
    regular: Font,
    bold: Option<Font>,
}

struct Chain<'a> {
    primary: &'a Font,
    rest: Vec<&'a Font>,
}

impl<'a> Chain<'a> {
    fn new(primary: &'a Font, rest: &[&'a Font]) -> Self {
        Self {
            primary,
            rest: rest.to_vec(),
        }
    }

    fn pick(&self, character: char) -> &'a Font {
        std::iter::once(self.primary)
            .chain(self.rest.iter().copied())
            .find(|font| font.lookup_glyph_index(character) != 0)
            .unwrap_or(self.primary)
    }

    fn measure(&self, size: f32, text: &str) -> f32 {
        text.chars()
            .map(|character| {
                self.pick(character)
                    .metrics(character, size * SCALE)
                    .advance_width
                    / SCALE
            })
            .sum()
    }
}

struct Chains<'a> {
    regular: Chain<'a>,
    bold: Chain<'a>,
    alone: Chain<'a>,
}

#[derive(Clone, Copy)]
struct Shape {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    radius: f32,
}

impl Shape {
    const fn new(left: f32, top: f32, width: f32, height: f32, radius: f32) -> Self {
        Self {
            left,
            top,
            width,
            height,
            radius,
        }
    }

    fn path(self) -> Option<tiny_skia::Path> {
        let (x0, y0) = (self.left * SCALE, self.top * SCALE);
        let (width, height) = (self.width * SCALE, self.height * SCALE);
        let corner = (self.radius * SCALE).min(width / 2.0).min(height / 2.0);
        let mut path = PathBuilder::new();
        path.move_to(x0 + corner, y0);
        path.line_to(x0 + width - corner, y0);
        path.quad_to(x0 + width, y0, x0 + width, y0 + corner);
        path.line_to(x0 + width, y0 + height - corner);
        path.quad_to(x0 + width, y0 + height, x0 + width - corner, y0 + height);
        path.line_to(x0 + corner, y0 + height);
        path.quad_to(x0, y0 + height, x0, y0 + height - corner);
        path.line_to(x0, y0 + corner);
        path.quad_to(x0, y0, x0 + corner, y0);
        path.close();
        path.finish()
    }
}

struct Glyph {
    left: i64,
    top: i64,
    width: usize,
    height: usize,
}

struct Canvas {
    pix: Pixmap,
}

impl Canvas {
    fn fill(&mut self, shape: Shape, color: Rgb) {
        if let Some(path) = shape.path() {
            self.pix.fill_path(
                &path,
                &paint(color),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    fn outline(&mut self, shape: Shape, color: Rgb) {
        if let Some(path) = shape.path() {
            let stroke = Stroke {
                width: SCALE,
                ..Stroke::default()
            };
            self.pix
                .stroke_path(&path, &paint(color), &stroke, Transform::identity(), None);
        }
    }

    fn rule(&mut self, top: f32, width: f32) {
        if let Some(rect) = Rect::from_xywh(0.0, top * SCALE, width * SCALE, SCALE) {
            self.pix
                .fill_rect(rect, &paint(HAIRLINE), Transform::identity(), None);
        }
    }

    fn dot(&mut self, center: (f32, f32), radius: f32, color: Rgb) {
        let mut path = PathBuilder::new();
        path.push_circle(center.0 * SCALE, center.1 * SCALE, radius * SCALE);
        if let Some(path) = path.finish() {
            self.pix.fill_path(
                &path,
                &paint(color),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    fn text(&mut self, chain: &Chain<'_>, origin: (f32, f32), size: f32, text: &str, color: Rgb) {
        let physical = size * SCALE;
        let mut pen = origin.0 * SCALE;
        let mut previous: Option<(&Font, char)> = None;
        for character in text.chars() {
            let font = chain.pick(character);
            if let Some((_, last)) =
                previous.filter(|(last_font, _)| std::ptr::eq(*last_font, font))
            {
                pen += font
                    .horizontal_kern(last, character, physical)
                    .unwrap_or(0.0);
            }
            let (metrics, bitmap) = font.rasterize(character, physical);
            let glyph = Glyph {
                left: round_i64(pen) + i64::from(metrics.xmin),
                top: round_i64(origin.1 * SCALE)
                    - i64::try_from(metrics.height).unwrap_or(0)
                    - i64::from(metrics.ymin),
                width: metrics.width,
                height: metrics.height,
            };
            self.blit(&glyph, &bitmap, color);
            pen += metrics.advance_width;
            previous = Some((font, character));
        }
    }

    fn blit(&mut self, glyph: &Glyph, bitmap: &[u8], color: Rgb) {
        let width = i64::from(self.pix.width());
        let height = i64::from(self.pix.height());
        let data = self.pix.data_mut();
        for row in 0..glyph.height {
            let y = glyph.top + i64::try_from(row).unwrap_or(0);
            if y < 0 || y >= height {
                continue;
            }
            for column in 0..glyph.width {
                let x = glyph.left + i64::try_from(column).unwrap_or(0);
                if x < 0 || x >= width {
                    continue;
                }
                let coverage = u32::from(bitmap[row * glyph.width + column]);
                if coverage == 0 {
                    continue;
                }
                let index = usize::try_from((y * width + x) * 4).unwrap_or(0);
                let inverse = 255 - coverage;
                for (offset, channel) in [color.0, color.1, color.2].into_iter().enumerate() {
                    let old = u32::from(data[index + offset]);
                    data[index + offset] =
                        u8::try_from((u32::from(channel) * coverage + old * inverse) / 255)
                            .unwrap_or(255);
                }
                data[index + 3] = 255;
            }
        }
    }
}

fn round_i64(value: f32) -> i64 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "pixel coordinates are far inside the i64 range"
    )]
    let rounded = value.round() as i64;
    rounded
}

fn scaled_px(value: f32) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "layout sizes are small positive numbers"
    )]
    let pixels = (value * SCALE).round() as u32;
    pixels
}

fn paint(color: Rgb) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, 255);
    paint.anti_alias = true;
    paint
}

fn background() -> Color {
    Color::from_rgba8(BACKGROUND.0, BACKGROUND.1, BACKGROUND.2, 255)
}

fn main() -> Result<(), Failure> {
    let usage = "usage: graphite_fonts FONT_DIR OUTPUT_DIR";
    let mut args = env::args().skip(1);
    let fonts = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().ok_or(usage)?);
    fs::create_dir_all(&out)?;
    let families = [
        load(
            &fonts,
            ("Inter", "inter"),
            "Inter-Regular.ttf",
            Some("Inter-Bold.ttf"),
        )?,
        load(
            &fonts,
            ("IBM Plex Sans", "ibm-plex-sans"),
            "IBMPlexSans-Regular.ttf",
            Some("IBMPlexSans-Bold.ttf"),
        )?,
        load(
            &fonts,
            ("Noto Sans", "noto-sans"),
            "NotoSans-Regular.ttf",
            Some("NotoSans-Bold.ttf"),
        )?,
        load(
            &fonts,
            ("JetBrains Mono", "jetbrains-mono"),
            "JetBrainsMono-Regular.ttf",
            Some("JetBrainsMono-Bold.ttf"),
        )?,
        load(
            &fonts,
            ("Noto Emoji", "noto-emoji"),
            "NotoEmoji-Variable.ttf",
            None,
        )?,
    ];
    let mut panels = Vec::new();
    for family in &families {
        let panel = draw_panel(family, &families[2].regular, &families[4].regular)?;
        panel.save_png(out.join(format!("font-{}.png", family.slug)))?;
        panels.push(panel);
    }
    side_by_side(&panels)?.save_png(out.join("side-by-side.png"))?;
    Ok(())
}

fn load(
    dir: &Path,
    names: (&'static str, &'static str),
    regular: &str,
    bold: Option<&str>,
) -> Result<Family, Failure> {
    let read = |name: &str| -> Result<Font, Failure> {
        let bytes = fs::read(dir.join(name))?;
        Font::from_bytes(bytes, FontSettings::default()).map_err(Into::into)
    };
    Ok(Family {
        title: names.0,
        slug: names.1,
        regular: read(regular)?,
        bold: bold.map(read).transpose()?,
    })
}

fn side_by_side(panels: &[Pixmap]) -> Result<Pixmap, Failure> {
    let gap = scaled_px(GAP);
    let width = panels.iter().map(Pixmap::width).sum::<u32>()
        + gap * u32::try_from(panels.len().saturating_sub(1))?;
    let height = panels.iter().map(Pixmap::height).max().unwrap_or(1);
    let mut sheet = Pixmap::new(width, height).ok_or("empty sheet")?;
    sheet.fill(background());
    let mut x = 0i32;
    for panel in panels {
        sheet.draw_pixmap(
            x,
            0,
            panel.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        x += i32::try_from(panel.width() + gap)?;
    }
    Ok(sheet)
}

fn draw_panel(family: &Family, sans: &Font, emoji: &Font) -> Result<Pixmap, Failure> {
    let mut canvas = Canvas {
        pix: Pixmap::new(scaled_px(PANEL_WIDTH), scaled_px(PANEL_HEIGHT)).ok_or("empty panel")?,
    };
    canvas.pix.fill(background());
    canvas.fill(
        Shape::new(0.0, 0.0, PANEL_WIDTH, PANEL_HEIGHT, 0.0),
        SURFACE,
    );
    let bold_font = family.bold.as_ref().unwrap_or(&family.regular);
    let emoji_face = family.bold.is_none();
    let (lead, lead_bold, rest): (&Font, &Font, Vec<&Font>) = if emoji_face {
        (sans, sans, vec![&family.regular])
    } else {
        (&family.regular, bold_font, vec![sans, emoji])
    };
    let chains = Chains {
        regular: Chain::new(lead, &rest),
        bold: Chain::new(lead_bold, &rest),
        alone: Chain::new(&family.regular, &[]),
    };
    let mut y = header(&mut canvas, &chains, family);
    y = labels(&mut canvas, &chains, y);
    y = menu(&mut canvas, &chains, y);
    y = list(&mut canvas, &chains, y);
    y = paragraph(&mut canvas, &chains, y);
    y = sizes(&mut canvas, &chains, y);
    coverage(&mut canvas, &chains, y);
    Ok(canvas.pix)
}

fn section(canvas: &mut Canvas, chains: &Chains<'_>, top: f32, title: &str) {
    canvas.text(
        &chains.regular,
        (PAD, top),
        11.0,
        &title.to_uppercase(),
        MUTED,
    );
}

fn header(canvas: &mut Canvas, chains: &Chains<'_>, family: &Family) -> f32 {
    canvas.text(&chains.bold, (PAD, 44.0), 24.0, family.title, STRONG);
    let note = if family.bold.is_some() {
        "regular and bold"
    } else {
        "one weight; text uses Noto Sans, this face adds emoji"
    };
    canvas.text(
        &chains.regular,
        (PAD, 66.0),
        11.0,
        &format!("real raster, 2x, tiny-skia and fontdue  \u{b7}  {note}"),
        MUTED,
    );
    canvas.rule(80.0, PANEL_WIDTH);
    110.0
}

fn labels(canvas: &mut Canvas, chains: &Chains<'_>, top: f32) -> f32 {
    section(canvas, chains, top, "UI labels");
    let row = top + 16.0;
    let mut x = PAD;
    for (index, label) in ["Save", "Cancel", "Settings\u{2026}", "Search"]
        .into_iter()
        .enumerate()
    {
        let width = chains.regular.measure(16.0, label) + 28.0;
        let pill = Shape::new(x, row, width, 34.0, 17.0);
        if index == 0 {
            canvas.fill(pill, FOCUS);
            canvas.text(&chains.bold, (x + 14.0, row + 23.0), 16.0, label, ON_FOCUS);
        } else {
            canvas.outline(pill, HAIRLINE);
            canvas.text(&chains.regular, (x + 14.0, row + 23.0), 16.0, label, TEXT);
        }
        x += width + 10.0;
    }
    top + 78.0
}

fn menu(canvas: &mut Canvas, chains: &Chains<'_>, top: f32) -> f32 {
    section(canvas, chains, top, "Menu");
    let bar = top + 18.0;
    let mut x = PAD;
    for item in ["File", "Edit", "View", "Go", "Window", "Help"] {
        canvas.text(&chains.regular, (x, bar), 13.0, item, TEXT);
        x += chains.regular.measure(13.0, item) + 18.0;
    }
    let panel = Shape::new(PAD, bar + 10.0, 300.0, 132.0, 10.0);
    canvas.fill(panel, MENU_SURFACE);
    canvas.outline(panel, HAIRLINE);
    let rows = [
        ("New window", "\u{2318}N"),
        ("Open\u{2026}", "\u{2318}O"),
        ("Open recent \u{25b8}", ""),
        ("Close", "\u{2318}W"),
    ];
    let mut row = panel.top + 8.0;
    for (index, (label, shortcut)) in rows.into_iter().enumerate() {
        if index == 1 {
            canvas.fill(Shape::new(PAD + 6.0, row - 2.0, 288.0, 26.0, 6.0), SELECT);
            canvas.fill(Shape::new(PAD + 6.0, row + 4.0, 3.0, 14.0, 1.5), FOCUS);
        }
        let color = if index == 1 { STRONG } else { TEXT };
        canvas.text(
            &chains.regular,
            (PAD + 18.0, row + 16.0),
            13.0,
            label,
            color,
        );
        let width = chains.regular.measure(13.0, shortcut);
        canvas.text(
            &chains.regular,
            (PAD + 286.0 - width, row + 16.0),
            13.0,
            shortcut,
            MUTED,
        );
        row += 30.0;
    }
    panel.top + 132.0 + 34.0
}

fn list(canvas: &mut Canvas, chains: &Chains<'_>, top: f32) -> f32 {
    section(canvas, chains, top, "List");
    let rows = [
        ("builder", "test_failed", "54s", RED),
        ("reviewer", "waiting", "53s", AMBER),
        ("worker", "command_started", "52s", GREEN),
        ("scribe", "edit_finished", "2m", GREEN),
    ];
    let mut y = top + 10.0;
    for (index, (name, state, age, color)) in rows.into_iter().enumerate() {
        if index == 0 {
            let width = PANEL_WIDTH - 2.0 * (PAD - 8.0);
            canvas.fill(Shape::new(PAD - 8.0, y, width, 30.0, 8.0), SELECT);
            canvas.fill(Shape::new(PAD - 8.0, y + 6.0, 3.0, 18.0, 1.5), FOCUS);
        }
        canvas.dot((PAD + 8.0, y + 15.0), 4.5, color);
        canvas.text(&chains.bold, (PAD + 24.0, y + 20.0), 13.0, name, STRONG);
        canvas.text(&chains.regular, (PAD + 130.0, y + 20.0), 13.0, state, MUTED);
        let width = chains.regular.measure(13.0, age);
        canvas.text(
            &chains.regular,
            (PANEL_WIDTH - PAD - width, y + 20.0),
            13.0,
            age,
            TEXT,
        );
        y += 32.0;
    }
    y + 22.0
}

fn paragraph(canvas: &mut Canvas, chains: &Chains<'_>, top: f32) -> f32 {
    section(canvas, chains, top, "Paragraph, 16 px");
    let text = "The quick brown fox jumps over the lazy dog, and then packs five dozen liquor jugs into the box before sunrise. A change of typeface should change how this feels, not whether it fits.";
    let mut y = top + 22.0;
    for line in wrap(&chains.regular, 16.0, text, PANEL_WIDTH - 2.0 * PAD) {
        canvas.text(&chains.regular, (PAD, y), 16.0, &line, TEXT);
        y += 23.0;
    }
    y + 16.0
}

fn wrap(chain: &Chain<'_>, size: f32, text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && chain.measure(size, &candidate) > width {
            lines.push(std::mem::take(&mut line));
            word.clone_into(&mut line);
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn sizes(canvas: &mut Canvas, chains: &Chains<'_>, top: f32) -> f32 {
    section(
        canvas,
        chains,
        top,
        "Numbers, punctuation: 11 / 13 / 16 / 24 px",
    );
    let sample = "0O1lI  il1|  rn m  -_=+  ~^`'\"  {}[]()<>  1,234.56";
    let mut y = top + 6.0;
    for size in [11.0, 13.0, 16.0, 24.0] {
        let line = if size > 20.0 {
            "0O1lI il1| rn m -_=+ 1,234"
        } else {
            sample
        };
        y += size + 6.0;
        canvas.text(&chains.regular, (PAD, y), size, line, TEXT);
        y += size + 4.0;
        canvas.text(&chains.bold, (PAD, y), size, line, STRONG);
    }
    y + 26.0
}

fn coverage(canvas: &mut Canvas, chains: &Chains<'_>, top: f32) {
    section(
        canvas,
        chains,
        top,
        "Coverage: alone (boxes = missing), then with fallback",
    );
    let samples = [
        "Caf\u{e9} na\u{ef}ve r\u{e9}sum\u{e9} \u{c5}ngstr\u{f6}m",
        "\u{391}\u{3b8}\u{3ae}\u{3bd}\u{3b1} \u{3b5}\u{3bb}\u{3bb}\u{3b7}\u{3bd}\u{3b9}\u{3ba}\u{3ac}",
        "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}, \u{43c}\u{438}\u{440} \u{41a}\u{438}\u{440}\u{438}\u{43b}\u{43b}\u{438}\u{446}\u{430}",
        "\u{2713} \u{2192} \u{2605} \u{2665} \u{a9} \u{2318} \u{25b8}",
        "\u{1f600} \u{1f680} \u{2764} \u{1f389} \u{1f4a1}",
    ];
    let mut y = top + 6.0;
    for sample in samples {
        y += 19.0;
        canvas.text(&chains.alone, (PAD, y), 15.0, sample, TEXT);
        y += 19.0;
        canvas.text(&chains.bold, (PAD, y), 15.0, sample, MUTED);
    }
}
