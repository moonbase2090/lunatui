// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Color, Rect, Size, Style};
use lunatui_image::{Fit, ImageGeometry, ImageId, ImageRequest, ImageSource, PixelSize};
use lunatui_theme::{Profile, Token};

use crate::{Block, BorderType, RenderCx, Widget, WidgetState};

/// Draws an immutable RGBA8 image using two vertical pixels in each terminal cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    source: ImageSource,
    fit: Fit,
    state: WidgetState,
    id: Option<ImageId>,
    generation: u64,
    depth: i32,
}

impl Image {
    /// Creates an image widget from a validated source.
    #[must_use]
    pub fn new(source: ImageSource) -> Self {
        Self {
            source,
            fit: Fit::Contain,
            state: WidgetState::Normal,
            id: None,
            generation: 0,
            depth: 0,
        }
    }

    /// Gives this image a stable pixel-protocol identity. Zero is reserved.
    #[must_use]
    pub const fn id(mut self, id: ImageId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the source generation. Increment it whenever pixels change.
    #[must_use]
    pub const fn generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }

    /// Sets relative depth for pixel protocols; smaller values appear behind text.
    #[must_use]
    pub const fn depth(mut self, depth: i32) -> Self {
        self.depth = depth;
        self
    }

    /// Sets how source pixels fit within the image area.
    #[must_use]
    pub const fn fit(mut self, fit: Fit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets the semantic appearance for normal, focused, or disabled use. A focused image
    /// is inset by a rounded focus ring drawn over the background already in its area.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// The decoded source held by this widget.
    #[must_use]
    pub const fn source(&self) -> &ImageSource {
        &self.source
    }
}

impl Widget for Image {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let image_area = if self.state == WidgetState::Focused {
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(cx.token(Token::FocusRing)));
            let inner = block.inner(area);
            cx.keep_backgrounds(area, |cx| block.render(area, cx));
            inner
        } else {
            area
        };
        if image_area.width == 0 || image_area.height == 0 {
            return;
        }
        if let Some(id) = self.id {
            let opacity = if self.state == WidgetState::Disabled {
                153
            } else {
                u8::MAX
            };
            if cx.submit_image(ImageRequest::new(
                id,
                self.generation,
                &self.source,
                image_area,
                self.fit,
                self.depth,
                opacity,
            )) {
                return;
            }
        }
        let Some(viewport_height) = u32::from(image_area.height).checked_mul(2) else {
            return;
        };
        let Some(viewport) = PixelSize::new(u32::from(image_area.width), viewport_height) else {
            return;
        };
        let geometry = ImageGeometry::fit(self.source.size(), viewport, self.fit);
        let backdrop = resolve_color(cx, cx.token(Token::Surface0));
        for row in 0..image_area.height {
            let top_y = u32::from(row) * 2;
            let bottom_y = top_y + 1;
            for column in 0..image_area.width {
                let x = u32::from(column);
                let foreground = color_at(
                    &self.source,
                    geometry,
                    x,
                    top_y,
                    backdrop,
                    cx.color_profile(),
                    self.state,
                );
                let background = color_at(
                    &self.source,
                    geometry,
                    x,
                    bottom_y,
                    backdrop,
                    cx.color_profile(),
                    self.state,
                );
                cx.put_char_patched(
                    image_area.x.saturating_add(column),
                    image_area.y.saturating_add(row),
                    '▀',
                    Style::new().fg(foreground).bg(background),
                );
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let minimum = if self.state == WidgetState::Focused {
            Size::new(3, 3)
        } else {
            Size::new(1, 1)
        };
        minimum.clamp(avail)
    }
}

fn color_at(
    source: &ImageSource,
    geometry: ImageGeometry,
    x: u32,
    y: u32,
    backdrop: Color,
    profile: Option<Profile>,
    state: WidgetState,
) -> Color {
    let destination = geometry.destination;
    let source_rect = geometry.source;
    if x < destination.origin.x
        || y < destination.origin.y
        || x >= destination.origin.x + destination.size.width()
        || y >= destination.origin.y + destination.size.height()
    {
        return downsample(backdrop, profile);
    }
    let relative_x = x - destination.origin.x;
    let relative_y = y - destination.origin.y;
    let source_x = source_rect.origin.x
        + u32::try_from(
            u64::from(relative_x) * u64::from(source_rect.size.width())
                / u64::from(destination.size.width()),
        )
        .unwrap_or(u32::MAX)
        .min(source_rect.size.width() - 1);
    let source_y = source_rect.origin.y
        + u32::try_from(
            u64::from(relative_y) * u64::from(source_rect.size.height())
                / u64::from(destination.size.height()),
        )
        .unwrap_or(u32::MAX)
        .min(source_rect.size.height() - 1);
    let offset = (u64::from(source_y) * u64::from(source.size().width()) + u64::from(source_x)) * 4;
    let Some(offset) = usize::try_from(offset).ok() else {
        return downsample(backdrop, profile);
    };
    let pixels = source.rgba8_pixels();
    let Some(pixel) = pixels.get(offset..offset.saturating_add(4)) else {
        return downsample(backdrop, profile);
    };
    let source_color = Color::Rgb(pixel[0], pixel[1], pixel[2]);
    let alpha = if state == WidgetState::Disabled {
        u8::try_from(u16::from(pixel[3]) * 3 / 5).unwrap_or(u8::MAX)
    } else {
        pixel[3]
    };
    let composited = composite(source_color, alpha, backdrop);
    downsample(composited, profile)
}

fn composite(source: Color, alpha: u8, backdrop: Color) -> Color {
    if alpha == 0 {
        return backdrop;
    }
    if alpha == u8::MAX {
        return source;
    }
    let Some((br, bg, bb)) = rgb(backdrop) else {
        return source;
    };
    let Color::Rgb(sr, sg, sb) = source else {
        return source;
    };
    let blend = |front: u8, back: u8| {
        let front = u32::from(front) * u32::from(alpha);
        let back = u32::from(back) * u32::from(u8::MAX - alpha);
        u8::try_from((front + back + 127) / 255).unwrap_or(u8::MAX)
    };
    Color::Rgb(blend(sr, br), blend(sg, bg), blend(sb, bb))
}

fn rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(red, green, blue) => Some((red, green, blue)),
        Color::Indexed(index) => Some(indexed_rgb(index)),
        Color::Default | Color::Token(_) => None,
    }
}

fn indexed_rgb(index: u8) -> (u8, u8, u8) {
    const ANSI: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    match index {
        0..=15 => ANSI[usize::from(index)],
        16..=231 => {
            const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];
            let value = index - 16;
            (
                CUBE[usize::from(value / 36)],
                CUBE[usize::from(value / 6 % 6)],
                CUBE[usize::from(value % 6)],
            )
        }
        _ => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

fn downsample(color: Color, profile: Option<Profile>) -> Color {
    profile.map_or(color, |profile| profile.downsample(color))
}

fn resolve_color(cx: &RenderCx<'_>, mut color: Color) -> Color {
    for _ in 0..16 {
        let Color::Token(id) = color else {
            return color;
        };
        color = cx.theme().resolve_id(id);
    }
    Color::Default
}
