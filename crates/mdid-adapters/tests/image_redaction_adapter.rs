use mdid_adapters::{
    redact_ppm_p6_bytes, redact_ppm_p6_bytes_with_verification, redact_rgb_regions,
    ImageRedactionError,
};
use mdid_domain::ImageRedactionRegion;

#[test]
fn ppm_visual_verification_counts_bounded_approved_pixels_without_phi_artifacts() {
    let input = [
        b"P6\n3 2\n255\n".as_slice(),
        &[
            1, 1, 1, // (0,0)
            2, 2, 2, // (1,0) redacted
            3, 3, 3, // (2,0) redacted
            4, 4, 4, // (0,1)
            5, 5, 5, // (1,1)
            6, 6, 6, // (2,1)
        ],
    ]
    .concat();
    let region = ImageRedactionRegion::new(1, 0, 2, 1).expect("valid region");

    let (output, verification) =
        redact_ppm_p6_bytes_with_verification(&input, &[region], [0, 0, 0])
            .expect("ppm redaction verification succeeds");

    assert_eq!(verification.format, "ppm_p6");
    assert_eq!(verification.width, 3);
    assert_eq!(verification.height, 2);
    assert_eq!(verification.redacted_region_count, 1);
    assert_eq!(verification.redacted_pixel_count, 2);
    assert_eq!(verification.unchanged_pixel_count, 4);
    assert_eq!(verification.output_byte_count, output.len() as u64);
    assert!(verification.verified_changed_pixels_within_regions);
}

#[test]
fn ppm_visual_verification_out_of_bounds_fails_without_artifact() {
    let input = [
        b"P6\n2 2\n255\n".as_slice(),
        &[b'J', b'a', b'n', b'e', 1, 2, 3, 4, 5, 6, 7, 8],
    ]
    .concat();
    let region = ImageRedactionRegion::new(1, 1, 2, 1).expect("valid region shape");

    let err = redact_ppm_p6_bytes_with_verification(&input, &[region], [0, 0, 0])
        .expect_err("oob fails before returning verification");

    assert_eq!(err, ImageRedactionError::RegionOutOfBounds);
}

#[test]
fn ppm_p6_redacts_approved_bbox_to_black_and_preserves_other_bytes() {
    let input = [
        b"P6\n2 2\n255\n".as_slice(),
        &[
            10, 11, 12, // (0,0)
            20, 21, 22, // (1,0) redacted
            30, 31, 32, // (0,1)
            40, 41, 42, // (1,1) redacted
        ],
    ]
    .concat();
    let region = ImageRedactionRegion::new(1, 0, 1, 2).expect("valid region");

    let output = redact_ppm_p6_bytes(&input, &[region]).expect("ppm redaction succeeds");

    assert_eq!(
        output,
        [
            b"P6\n2 2\n255\n".as_slice(),
            &[10, 11, 12, 0, 0, 0, 30, 31, 32, 0, 0, 0,],
        ]
        .concat()
    );
}

#[test]
fn ppm_p6_out_of_bounds_region_fails_without_debugging_raw_source_names() {
    let input = [
        b"P6\n2 2\n255\n".as_slice(),
        &[b'J', b'a', b'n', b'e', 1, 2, 3, 4, 5, 6, 7, 8],
    ]
    .concat();
    let region = ImageRedactionRegion::new(1, 1, 2, 1).expect("valid region shape");

    let err = redact_ppm_p6_bytes(&input, &[region]).expect_err("oob fails");
    let debug = format!("{err:?}");

    assert_eq!(err, ImageRedactionError::RegionOutOfBounds);
    assert!(!debug.contains("Jane"));
    assert!(!debug.contains("patient.ppm"));
}

#[test]
fn ppm_p6_zero_width_is_malformed() {
    let input = b"P6\n0 1\n255\n";

    let err = redact_ppm_p6_bytes(input, &[]).expect_err("zero-width PPM fails closed");

    assert_eq!(err, ImageRedactionError::MalformedPpmP6);
}

#[test]
fn ppm_p6_zero_height_is_malformed() {
    let input = b"P6\n1 0\n255\n";

    let err = redact_ppm_p6_bytes(input, &[]).expect_err("zero-height PPM fails closed");

    assert_eq!(err, ImageRedactionError::MalformedPpmP6);
}

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
