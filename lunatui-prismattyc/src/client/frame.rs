// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use crate::{FrameEncoder, SLOT_BYTES};
use anyhow::{Result, ensure};
use lunatui_core::{ClusterTable, Grid};
use lunatui_image::{ImageGeometry, ImageRequest, PixelSize};

#[derive(Clone, Debug, PartialEq, Eq)]
/// One validated image placement in a native frame snapshot.
pub struct Placement {
    bytes: [u8; 64],
}

impl Placement {
    /// Builds a placement from a widget viewport and host cell metrics.
    pub fn from_request(
        request: ImageRequest<'_>,
        handle: u64,
        cell: [u32; 2],
        ordinal: u32,
    ) -> Result<Self> {
        ensure!(handle != 0 && ordinal < 256, "placement identity");
        let area = request.area();
        let width = u32::from(area.width).checked_mul(cell[0]);
        let height = u32::from(area.height).checked_mul(cell[1]);
        let viewport = width.zip(height).and_then(|(w, h)| PixelSize::new(w, h));
        let viewport = viewport.ok_or_else(|| crate::invalid("placement viewport"))?;
        let geometry = ImageGeometry::fit(request.source().size(), viewport, request.fit());
        let crop = geometry.source;
        let destination = geometry.destination;
        let x = u64::from(area.x) * u64::from(cell[0]) + u64::from(destination.origin.x);
        let y = u64::from(area.y) * u64::from(cell[1]) + u64::from(destination.origin.y);
        ensure!(
            x + u64::from(destination.size.width()) <= 1 << 30,
            "placement x"
        );
        ensure!(
            y + u64::from(destination.size.height()) <= 1 << 30,
            "placement y"
        );
        let mut bytes = [0; 64];
        put32(&mut bytes, 0, request.id().get());
        put32(
            &mut bytes,
            4,
            u32::try_from(request.depth()).map_err(|_| crate::invalid("negative image depth"))?,
        );
        bytes[8..16].copy_from_slice(&handle.to_le_bytes());
        for (offset, value) in [
            (16, crop.origin.x),
            (20, crop.origin.y),
            (24, crop.size.width()),
            (28, crop.size.height()),
            (
                32,
                u32::try_from(x).map_err(|_| crate::invalid("placement x"))?,
            ),
            (
                36,
                u32::try_from(y).map_err(|_| crate::invalid("placement y"))?,
            ),
            (40, destination.size.width()),
            (44, destination.size.height()),
            (52, ordinal),
        ] {
            put32(&mut bytes, offset, value);
        }
        bytes[48] = request.opacity();
        Ok(Self { bytes })
    }
}

/// Encodes a complete grid snapshot and its native image placements.
pub fn encode(
    encoder: &mut FrameEncoder,
    grid: &Grid,
    clusters: &ClusterTable,
    epoch: u64,
    metrics: u64,
    sequence: u64,
    placements: &[Placement],
) -> Result<Vec<u8>> {
    ensure!(metrics != 0 && sequence != 0, "frame identity");
    ensure!(placements.len() <= 256, "placement count");
    let mut bytes = encoder
        .encode(grid, clusters, epoch, sequence, None)?
        .to_vec();
    ensure!(
        crate::u32_at(&bytes, 76) == 0,
        "probe does not select alpha"
    );
    bytes[96..104].copy_from_slice(&metrics.to_le_bytes());
    if !placements.is_empty() {
        let offset = bytes.len();
        ensure!(
            offset % 8 == 0 && offset + placements.len() * 64 <= SLOT_BYTES,
            "frame size"
        );
        put32(
            &mut bytes,
            84,
            u32::try_from(placements.len()).map_err(|_| crate::invalid("placement count"))?,
        );
        put32(
            &mut bytes,
            88,
            u32::try_from(offset).map_err(|_| crate::invalid("frame offset"))?,
        );
        for placement in placements {
            bytes.extend_from_slice(&placement.bytes);
        }
        let length = u32::try_from(bytes.len()).map_err(|_| crate::invalid("frame length"))?;
        put32(&mut bytes, 0, length);
    }
    Ok(bytes)
}

fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
