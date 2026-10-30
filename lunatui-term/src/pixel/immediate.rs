// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::{PixelImage, PixelLayer, PixelMode, Placed, push_base64, push_decimal, write_cup};
use lunatui_core::Rect;

const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;
const MAX_PIXELS: u32 = 1_048_576;

impl PixelLayer {
    pub(super) fn immediate_image(
        &mut self,
        id: u32,
        image: &PixelImage<'_>,
        at: Rect,
        z: i32,
    ) -> bool {
        let Some(cell) = self.cell_pixels else {
            return false;
        };
        let width = u32::from(at.width) * u32::from(cell.width());
        let height = u32::from(at.height) * u32::from(cell.height());
        if z < 0
            || width == 0
            || height == 0
            || width.checked_mul(height).is_none_or(|n| n > MAX_PIXELS)
            || self.frame.len() >= 256
        {
            return false;
        }
        let rgba = resize(
            image,
            width,
            height,
            self.background,
            self.mode == PixelMode::Sixel,
        );
        let Some(rgba) = rgba else {
            return false;
        };
        let mut encoded = Vec::new();
        encoded.extend_from_slice(b"\x1b7");
        write_cup(&mut encoded, at.x, at.y);
        let accepted = match self.mode {
            PixelMode::Iterm2 => inline_png(&mut encoded, width, height, &rgba, at),
            PixelMode::Sixel => sixel(&mut encoded, width, height, &rgba),
            _ => false,
        };
        encoded.extend_from_slice(b"\x1b8");
        if !accepted || self.out.len().saturating_add(encoded.len()) > MAX_FRAME_BYTES {
            return false;
        }
        self.out.extend_from_slice(&encoded);
        self.frame_pixels += u64::from(width) * u64::from(height);
        self.frame.push(Placed {
            id,
            wire_id: id,
            rect: at,
            z,
            fresh: true,
        });
        true
    }
}

fn resize(
    image: &PixelImage<'_>,
    width: u32,
    height: u32,
    background: Option<[u8; 3]>,
    opaque: bool,
) -> Option<Vec<u8>> {
    let mut rgba =
        Vec::with_capacity(usize::try_from(width.checked_mul(height)?.checked_mul(4)?).ok()?);
    for y in 0..height {
        for x in 0..width {
            let offset = usize::try_from(
                (y * image.height / height * image.width + x * image.width / width) * 4,
            )
            .ok()?;
            let source = image.rgba.get(offset..offset + 4)?;
            if opaque && source[3] < 255 {
                let bg = background?;
                for (value, base) in source[..3].iter().zip(bg) {
                    let value = (u32::from(*value) * u32::from(source[3])
                        + u32::from(base) * (255 - u32::from(source[3]))
                        + 127)
                        / 255;
                    rgba.push(u8::try_from(value).ok()?);
                }
                rgba.push(255);
            } else {
                rgba.extend_from_slice(source);
            }
        }
    }
    Some(rgba)
}

fn inline_png(out: &mut Vec<u8>, width: u32, height: u32, rgba: &[u8], at: Rect) -> bool {
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let Ok(mut writer) = encoder.write_header() else {
        return false;
    };
    if writer.write_image_data(rgba).is_err() || writer.finish().is_err() {
        return false;
    }
    out.extend_from_slice(b"\x1b]1337;File=inline=1;size=");
    push_decimal(out, u64::try_from(file.len()).unwrap_or(u64::MAX));
    out.extend_from_slice(b";width=");
    push_decimal(out, u64::from(at.width));
    out.extend_from_slice(b";height=");
    push_decimal(out, u64::from(at.height));
    out.extend_from_slice(b";preserveAspectRatio=0:");
    push_base64(out, &file);
    out.extend_from_slice(b"\x1b\\");
    true
}

#[derive(Clone, Copy, Default)]
struct Bin {
    count: u32,
    sums: [u64; 3],
}
impl Bin {
    fn rgb(self) -> [u8; 3] {
        self.sums
            .map(|sum| u8::try_from(sum / u64::from(self.count.max(1))).unwrap_or(255))
    }
}
fn bucket(rgb: &[u8]) -> usize {
    usize::from(rgb[0] >> 3) * 1024 + usize::from(rgb[1] >> 3) * 32 + usize::from(rgb[2] >> 3)
}

fn palette(rgba: &[u8]) -> (Vec<[u8; 3]>, Vec<u8>) {
    let mut histogram = vec![Bin::default(); 32768];
    for pixel in rgba.as_chunks::<4>().0 {
        let bin = &mut histogram[bucket(pixel)];
        bin.count += 1;
        for (sum, value) in bin.sums.iter_mut().zip(pixel) {
            *sum += u64::from(*value);
        }
    }
    let populated: Vec<Bin> = histogram
        .iter()
        .copied()
        .filter(|bin| bin.count > 0)
        .collect();
    let mut boxes = vec![populated];
    while boxes.len() < 256 {
        let Some((index, _)) = boxes
            .iter()
            .enumerate()
            .filter(|(_, bins)| bins.len() > 1)
            .max_by_key(|(_, bins)| bins.iter().map(|bin| u64::from(bin.count)).sum::<u64>())
        else {
            break;
        };
        let mut bins = boxes.swap_remove(index);
        let channel = (0..3)
            .max_by_key(|channel| {
                let lo = bins
                    .iter()
                    .map(|bin| bin.rgb()[*channel])
                    .min()
                    .unwrap_or(0);
                let hi = bins
                    .iter()
                    .map(|bin| bin.rgb()[*channel])
                    .max()
                    .unwrap_or(0);
                hi - lo
            })
            .unwrap_or(0);
        bins.sort_unstable_by_key(|bin| bin.rgb()[channel]);
        let half = bins.iter().map(|bin| u64::from(bin.count)).sum::<u64>() / 2;
        let mut count = 0;
        let split = bins
            .iter()
            .position(|bin| {
                count += u64::from(bin.count);
                count >= half
            })
            .unwrap_or(0)
            .saturating_add(1)
            .clamp(1, bins.len() - 1);
        let right = bins.split_off(split);
        boxes.push(bins);
        boxes.push(right);
    }
    let colors: Vec<[u8; 3]> = boxes
        .into_iter()
        .map(|bins| {
            let combined = bins.into_iter().fold(Bin::default(), |mut total, bin| {
                total.count += bin.count;
                for (sum, value) in total.sums.iter_mut().zip(bin.sums) {
                    *sum += value;
                }
                total
            });
            combined.rgb()
        })
        .collect();
    let lookup = histogram
        .into_iter()
        .map(|bin| {
            if bin.count == 0 {
                return 0;
            }
            let rgb = bin.rgb();
            colors
                .iter()
                .enumerate()
                .min_by_key(|(_, color)| {
                    color
                        .iter()
                        .zip(rgb)
                        .map(|(a, b)| i32::from(*a).abs_diff(i32::from(b)).pow(2))
                        .sum::<u32>()
                })
                .and_then(|(index, _)| u8::try_from(index).ok())
                .unwrap_or(0)
        })
        .collect();
    (colors, lookup)
}

fn sixel(out: &mut Vec<u8>, width: u32, height: u32, rgba: &[u8]) -> bool {
    let (colors, lookup) = palette(rgba);
    let indices: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| lookup[bucket(pixel)])
        .collect();
    out.extend_from_slice(b"\x1bP0;1;0q\"1;1;");
    push_decimal(out, u64::from(width));
    out.push(b';');
    push_decimal(out, u64::from(height));
    for (index, color) in colors.iter().enumerate() {
        out.push(b'#');
        push_decimal(out, u64::try_from(index).unwrap_or(0));
        out.extend_from_slice(b";2");
        for channel in color {
            out.push(b';');
            push_decimal(out, (u64::from(*channel) * 100 + 127) / 255);
        }
    }
    let width = usize::try_from(width).unwrap_or(0);
    let height = usize::try_from(height).unwrap_or(0);
    let mut row = vec![b'?'; width];
    for top in (0..height).step_by(6) {
        for color in 0..colors.len() {
            row.fill(b'?');
            for y in top..(top + 6).min(height) {
                for (x, value) in row.iter_mut().enumerate() {
                    if usize::from(indices[y * width + x]) == color {
                        *value += 1 << (y - top);
                    }
                }
            }
            let Some(last) = row.iter().rposition(|value| *value != b'?') else {
                continue;
            };
            out.push(b'#');
            push_decimal(out, u64::try_from(color).unwrap_or(0));
            let mut x = 0;
            while x <= last {
                let value = row[x];
                let run = row[x..=last]
                    .iter()
                    .take_while(|next| **next == value)
                    .count();
                if run > 3 {
                    out.push(b'!');
                    push_decimal(out, u64::try_from(run).unwrap_or(0));
                    out.push(value);
                } else {
                    out.extend(std::iter::repeat_n(value, run));
                }
                x += run;
            }
            out.push(b'$');
            if out.len() > MAX_FRAME_BYTES {
                return false;
            }
        }
        if top + 6 < height {
            out.push(b'-');
        }
    }
    out.extend_from_slice(b"\x1b\\");
    true
}
