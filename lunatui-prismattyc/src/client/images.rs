// SPDX-License-Identifier: MIT
use crate::client::frame::Placement;
use lunatui_core::Rect;
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Stable image identifier and source generation pair.
pub type Key = (u32, u64);

enum Resource {
    Pending(u64),
    Ready(u64),
    Declined,
    Retiring(u64),
}

/// Stateful native image admission and placement sink for an active session.
pub struct Images {
    cell: [u32; 2],
    resources: BTreeMap<Key, Resource>,
    requested: VecDeque<(Key, ImageSource)>,
    preparing: BTreeSet<Key>,
    placements: Vec<Placement>,
    placed_handles: Vec<u64>,
    placed_ids: Vec<u32>,
}

impl Images {
    /// Creates an image sink with measured physical cell dimensions.
    pub fn new(cell: [u32; 2]) -> Self {
        Self {
            cell,
            resources: BTreeMap::new(),
            requested: VecDeque::new(),
            preparing: BTreeSet::new(),
            placements: Vec::new(),
            placed_handles: Vec::new(),
            placed_ids: Vec::new(),
        }
    }
    /// Starts a new complete placement snapshot.
    pub fn begin_frame(&mut self) {
        self.placements.clear();
        self.placed_handles.clear();
        self.placed_ids.clear();
    }
    /// Host cell metrics used to size image targets in physical pixels.
    #[must_use]
    pub const fn cell_pixels(&self) -> [u32; 2] {
        self.cell
    }
    /// Placements accepted for the current frame.
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }
    /// Whether any admitted source is awaiting renderer readiness.
    pub fn pending(&self) -> bool {
        self.resources
            .values()
            .any(|resource| matches!(resource, Resource::Pending(_)))
    }

    /// Whether one source is currently being prepared off the render thread.
    #[must_use]
    pub fn preparing(&self, key: Key) -> bool {
        self.preparing.contains(&key)
    }
    /// Marks an image unavailable so the widget keeps its cell fallback.
    pub fn decline_preparation(&mut self, key: Key) {
        self.preparing.remove(&key);
        self.resources.insert(key, Resource::Declined);
    }
    /// Removes the next source that needs preparation and admission.
    pub fn take_request(&mut self) -> Option<(Key, ImageSource)> {
        while let Some((key, source)) = self.requested.pop_front() {
            if !self.resources.contains_key(&key) {
                self.preparing.insert(key);
                return Some((key, source));
            }
        }
        None
    }

    /// Applies the host's admission response to the source state.
    pub fn admission(&mut self, key: Key, bytes: &[u8]) -> std::io::Result<()> {
        self.preparing.remove(&key);
        if bytes.len() != 48 || crate::u16_at(bytes, 4) != 11 || crate::u32_at(bytes, 44) != 0 {
            return Err(crate::invalid("image result"));
        }
        let handle = crate::u64_at(bytes, 32);
        let status = crate::u32_at(bytes, 40);
        let resource = match (status, handle) {
            (0, handle) if handle != 0 => Resource::Pending(handle),
            (1, handle) if handle != 0 => Resource::Ready(handle),
            (2..=5, 0) => Resource::Declined,
            _ => return Err(crate::invalid("image admission status")),
        };
        self.resources.insert(key, resource);
        Ok(())
    }

    /// A READY response is the only path from PENDING to visible placement.
    /// Applies a heartbeat response and promotes resources reported as READY.
    pub fn pong(&mut self, bytes: &[u8]) -> std::io::Result<bool> {
        if bytes.len() != 48 || crate::u16_at(bytes, 4) != 15 {
            return Err(crate::invalid("image heartbeat"));
        }
        let handle = crate::u64_at(bytes, 32);
        let state = crate::u32_at(bytes, 40);
        let error = crate::u32_at(bytes, 44);
        if !matches!(
            (state, handle, error),
            (0, 0, 0) | (1 | 2, 1.., 0) | (3, 1.., 1..=3)
        ) {
            return Err(crate::invalid("image heartbeat state"));
        }
        let mut changed = false;
        for resource in self.resources.values_mut() {
            if matches!(resource, Resource::Pending(value) if *value == handle) {
                match state {
                    2 => {
                        *resource = Resource::Ready(handle);
                        changed = true;
                    }
                    3 => {
                        *resource = Resource::Declined;
                        changed = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(changed)
    }

    /// Returns the next resource that can be retired after omission.
    pub fn retirement(&mut self) -> Option<(Key, u64)> {
        self.resources
            .iter_mut()
            .find_map(|(key, resource)| match *resource {
                Resource::Ready(handle) if !self.placed_handles.contains(&handle) => {
                    *resource = Resource::Retiring(handle);
                    Some((*key, handle))
                }
                Resource::Retiring(handle) => Some((*key, handle)),
                _ => None,
            })
    }

    /// Applies a host response to a resource retirement request.
    pub fn retired(&mut self, key: Key, bytes: &[u8]) -> std::io::Result<bool> {
        if bytes.len() != 48 || crate::u16_at(bytes, 4) != 13 || crate::u32_at(bytes, 44) != 0 {
            return Err(crate::invalid("image retirement reply"));
        }
        let Some(Resource::Retiring(handle)) = self.resources.get(&key) else {
            return Err(crate::invalid("retirement identity"));
        };
        if *handle != crate::u64_at(bytes, 32) {
            return Err(crate::invalid("retirement handle"));
        }
        match crate::u32_at(bytes, 40) {
            0 => {
                self.resources.remove(&key);
                Ok(true)
            }
            1 => Ok(false),
            _ => Err(crate::invalid("retirement status")),
        }
    }
}

impl ImageSink for Images {
    fn supports_pixels(&self) -> bool {
        true
    }

    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        if self.placements.len() >= 256 {
            return None;
        }
        let width = u32::from(area.width).checked_mul(self.cell[0])?;
        let height = u32::from(area.height).checked_mul(self.cell[1])?;
        if width > 4096 || height > 4096 || width.checked_mul(height)? > 1_048_576 {
            return None;
        }
        Some(PixelTarget::new(
            PixelSize::new(width, height)?,
            true,
            Some([0, 0, 0]),
        ))
    }

    fn submit(&mut self, request: ImageRequest<'_>) -> bool {
        let key = (request.id().get(), request.generation());
        if self.preparing.contains(&key) {
            return false;
        }
        let handle = match self.resources.get(&key) {
            Some(Resource::Ready(handle)) => *handle,
            Some(_) => return false,
            None => {
                // One coalesced candidate. A renderer repeats declined/pending images
                // on the next scheduled render and keeps its actual fallback cells.
                if self.requested.len() < 256
                    && !self.requested.iter().any(|(queued, _)| *queued == key)
                {
                    self.requested.push_back((key, request.source().clone()));
                }
                return false;
            }
        };
        if self.placements.len() >= 256 || self.placed_ids.contains(&key.0) {
            return false;
        }
        let Ok(placement) =
            Placement::from_request(request, handle, self.cell, self.placements.len() as u32)
        else {
            return false;
        };
        self.placements.push(placement);
        self.placed_handles.push(handle);
        self.placed_ids.push(key.0);
        true
    }
}
