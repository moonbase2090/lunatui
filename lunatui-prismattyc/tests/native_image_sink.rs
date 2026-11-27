// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(unix)]

use lunatui_core::Rect;
use lunatui_image::{Fit, ImageId, ImageRequest, ImageSink, ImageSource};
use lunatui_prismattyc::native_image::ImagePresenter;

fn response(opcode: u16) -> [u8; 48] {
    let mut bytes = [0; 48];
    bytes[..4].copy_from_slice(&48_u32.to_le_bytes());
    bytes[4..6].copy_from_slice(&opcode.to_le_bytes());
    bytes
}

#[test]
fn image_becomes_visible_only_after_ready_and_retires_after_omission() -> Result<(), String> {
    let source = ImageSource::rgba8(
        2,
        1,
        vec![255, 0, 0, 255, 0, 0, 255, 255].into_boxed_slice(),
    )
    .map_err(|error| error.to_string())?;
    let id = ImageId::new(17).ok_or("nonzero id")?;
    let key = (id.get(), 4);
    let mut images = ImagePresenter::new([8, 16]);
    images.begin_frame();
    let request = ImageRequest::new(id, 4, &source, Rect::new(1, 2, 3, 2), Fit::Contain, 5, 255);
    assert!(!images.submit(request));
    assert_eq!(images.take_request().map(|(key, _)| key), Some(key));
    assert!(
        !images.submit(request),
        "preparing sources are not queued twice"
    );
    assert!(images.take_request().is_none());

    let handle = 91_u64;
    let mut admitted = response(11);
    admitted[32..40].copy_from_slice(&handle.to_le_bytes());
    admitted[40..44].copy_from_slice(&0_u32.to_le_bytes());
    images
        .admission(key, &admitted)
        .map_err(|error| error.to_string())?;
    assert!(!images.submit(request), "pending sources are not visible");

    let mut ready = response(15);
    ready[32..40].copy_from_slice(&handle.to_le_bytes());
    ready[40..44].copy_from_slice(&2_u32.to_le_bytes());
    assert!(images.pong(&ready).map_err(|error| error.to_string())?);
    images.begin_frame();
    assert!(images.submit(request));
    assert_eq!(images.placements().len(), 1);

    images.begin_frame();
    assert_eq!(
        images.retirement().map(|(_, current)| current),
        Some(handle)
    );
    let mut retired = response(13);
    retired[32..40].copy_from_slice(&handle.to_le_bytes());
    retired[40..44].copy_from_slice(&0_u32.to_le_bytes());
    assert!(
        images
            .retired(key, &retired)
            .map_err(|error| error.to_string())?
    );
    Ok(())
}
