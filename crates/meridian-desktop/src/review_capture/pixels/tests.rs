use super::*;
#[test]
fn copy_admission_needs_directory_actual_usage_and_supported_format() {
    let supported_usage =
        Some(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
    assert!(copy_eligible(
        true,
        supported_usage,
        wgpu::TextureFormat::Bgra8UnormSrgb
    ));
    assert!(!copy_eligible(
        false,
        supported_usage,
        wgpu::TextureFormat::Bgra8UnormSrgb
    ));
    assert!(!copy_eligible(
        true,
        None,
        wgpu::TextureFormat::Bgra8UnormSrgb
    ));
    assert!(!copy_eligible(
        true,
        Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
        wgpu::TextureFormat::Bgra8UnormSrgb
    ));
    assert!(!copy_eligible(
        true,
        supported_usage,
        wgpu::TextureFormat::Rgba16Float
    ));
}
#[test]
fn padding_limits_and_unpack_keep_rows_channels_and_exclude_only_copy_padding() {
    let layout = Layout::new(3, 2).unwrap_or_else(|| panic!("valid small target"));
    assert_eq!(layout.row, 12);
    assert_eq!(layout.padded, 256);
    assert_eq!(layout.bytes, 512);
    let mut mapped = vec![0xee; 512];
    mapped[..12].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    mapped[256..268].copy_from_slice(&[21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    assert_eq!(
        layout.unpack(&mapped),
        Some(vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
        ])
    );
    assert!(layout.unpack(&mapped[..511]).is_none());
    assert!(Layout::new(0, 1).is_none());
    assert!(Layout::new(u32::MAX, 2).is_none());
    assert!(Layout::new(4096, 4096).is_none());
    assert_eq!(Layout::new(2560, 1440).map(|p| p.bytes), Some(14_745_600));
}
#[test]
fn artifact_uses_request_and_frame_identity_and_never_overwrites_previous_bytes() {
    let directory = crate::profile::tests::TestDirectory::new();
    let tag = Tag {
        request: 2,
        frame: 42,
        review: super::super::ReviewId {
            token: 3,
            item: 17,
            operation: "Fit",
            facing: Some(meridian_core::LaserFacing::Front),
        },
    };
    let path = save_pixels(&directory.0, tag, &[1, 2, 3, 4]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        path.file_name().and_then(|s| s.to_str()),
        Some("review-request-2-frame-42.pixels")
    );
    assert!(save_pixels(&directory.0, tag, &[8, 8, 8, 8]).is_err());
    assert_eq!(
        std::fs::read(path).unwrap_or_else(|e| panic!("{e}")),
        [1, 2, 3, 4]
    );
}

#[test]
fn callback_channel_cannot_promote_missing_or_failed_completion_to_readable_pixels() {
    // Channel policy only, not a fabricated GPU callback/completion.
    let (send, receive) = mpsc::channel();
    assert_eq!(map_status(&receive), None);
    send.send(Err("mapping refused".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(map_status(&receive), Some(Err("mapping refused".into())));
    assert_eq!(map_status(&receive), None);
    drop(send);
    assert!(matches!(map_status(&receive),Some(Err(reason)) if reason.contains("disconnected")));
}
