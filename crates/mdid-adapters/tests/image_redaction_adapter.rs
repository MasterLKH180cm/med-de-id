use mdid_adapters::{redact_rgb_regions, ImageRedactionError};
use mdid_domain::ImageRedactionRegion;

#[test]
fn redacts_approved_region_pixels_to_black_and_leaves_outside_unchanged() {
    // 4x3 RGB image: each pixel is [n, n, n] so unchanged pixels are easy to assert.
    let mut pixels: Vec<u8> = (0..12).flat_map(|n| [n, n, n]).collect();
    let original = pixels.clone();
    let region = ImageRedactionRegion::new(1, 1, 2, 1).expect("valid region");

    redact_rgb_regions(&mut pixels, 4, 3, &[region], [0, 0, 0]).expect("redaction succeeds");

    for pixel_index in 0..12 {
        let start = pixel_index * 3;
        let pixel = &pixels[start..start + 3];
        if pixel_index == 5 || pixel_index == 6 {
            assert_eq!(pixel, [0, 0, 0]);
        } else {
            assert_eq!(pixel, &original[start..start + 3]);
        }
    }
}

#[test]
fn redacts_with_configured_fill_color() {
    let mut pixels: Vec<u8> = (0..4).flat_map(|n| [n, n, n]).collect();
    let region = ImageRedactionRegion::new(0, 0, 1, 1).expect("valid region");

    redact_rgb_regions(&mut pixels, 2, 2, &[region], [9, 8, 7]).expect("redaction succeeds");

    assert_eq!(&pixels[0..3], [9, 8, 7]);
    assert_eq!(&pixels[3..], [1, 1, 1, 2, 2, 2, 3, 3, 3]);
}

#[test]
fn out_of_bounds_region_fails_closed_without_mutating_pixels() {
    let mut pixels: Vec<u8> = (0..4).flat_map(|n| [n, n, n]).collect();
    let original = pixels.clone();
    let region = ImageRedactionRegion::new(1, 1, 2, 1).expect("valid region shape");

    let err = redact_rgb_regions(&mut pixels, 2, 2, &[region], [0, 0, 0]).expect_err("oob fails");

    assert_eq!(err, ImageRedactionError::RegionOutOfBounds);
    assert_eq!(pixels, original);
}

#[test]
fn malformed_rgb_buffer_fails_closed_without_debugging_raw_bytes_or_source_names() {
    let mut pixels = vec![255, 0, 0, 42];
    let original = pixels.clone();
    let region = ImageRedactionRegion::new(0, 0, 1, 1).expect("valid region");

    let err =
        redact_rgb_regions(&mut pixels, 2, 1, &[region], [0, 0, 0]).expect_err("bad len fails");
    let debug = format!("{err:?}");

    assert_eq!(err, ImageRedactionError::MalformedRgbBuffer);
    assert_eq!(pixels, original);
    assert!(!debug.contains("patient.png"));
    assert!(!debug.contains("255"));
    assert!(!debug.contains("42"));
}
