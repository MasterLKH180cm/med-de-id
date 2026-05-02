use mdid_adapters::ImageRedactionError;
use mdid_application::{ApplicationError, VisualRedactionError, VisualRedactionService};
use mdid_domain::ImageRedactionRegion;

fn sample_ppm() -> Vec<u8> {
    let mut ppm = b"P6\n3 2\n255\n".to_vec();
    ppm.extend_from_slice(&[
        10, 11, 12, 20, 21, 22, 30, 31, 32, 40, 41, 42, 50, 51, 52, 60, 61, 62,
    ]);
    ppm
}

fn sample_png() -> Vec<u8> {
    let image = image::RgbaImage::from_raw(2, 1, vec![10, 11, 12, 255, 20, 21, 22, 255])
        .expect("valid fixture pixels");
    let mut output = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut output),
            image::ImageFormat::Png,
        )
        .expect("encode png fixture");
    output
}

#[test]
fn visual_redaction_service_redacts_ppm_and_returns_phi_safe_verification() {
    let service = VisualRedactionService;
    let region = ImageRedactionRegion::new(1, 0, 1, 2).unwrap();

    let output = service
        .redact_ppm_p6_bytes(&sample_ppm(), &[region])
        .unwrap();

    assert_eq!(&output.rewritten_ppm_bytes[..11], b"P6\n3 2\n255\n");
    assert_eq!(&output.rewritten_ppm_bytes[14..17], &[0, 0, 0]);
    assert_eq!(&output.rewritten_ppm_bytes[23..26], &[0, 0, 0]);
    assert_eq!(output.verification.format, "ppm_p6");
    assert_eq!(output.verification.width, 3);
    assert_eq!(output.verification.height, 2);
    assert_eq!(output.verification.redacted_region_count, 1);
    assert_eq!(output.verification.redacted_pixel_count, 2);
    assert!(output.verification.verified_changed_pixels_within_regions);

    let debug = format!("{output:?}");
    assert!(debug.contains("[REDACTED]"), "{debug}");
    assert!(!debug.contains("rewritten_ppm_bytes: ["), "{debug}");
    assert!(!debug.contains("patient-face.ppm"), "{debug}");
    assert!(!debug.contains("/tmp"), "{debug}");
}

#[test]
fn visual_redaction_service_maps_malformed_ppm_without_raw_source_names() {
    let service = VisualRedactionService;
    let region = ImageRedactionRegion::new(0, 0, 1, 1).unwrap();

    let err = service
        .redact_ppm_p6_bytes(b"patient-face.ppm is not a ppm", &[region])
        .unwrap_err();

    assert!(matches!(err, ApplicationError::VisualRedaction(_)));
    let rendered = err.to_string();
    assert!(rendered.contains("visual redaction failed"), "{rendered}");
    assert!(!rendered.contains("patient-face.ppm"), "{rendered}");
    assert!(!rendered.contains("not a ppm"), "{rendered}");
}

#[test]
fn visual_redaction_service_redacts_png_and_returns_bounded_verification() {
    let service = VisualRedactionService;
    let input = sample_png();
    let region = ImageRedactionRegion::new(0, 0, 1, 1).unwrap();

    let output = service.redact_png_bytes(&input, &[region]).unwrap();

    assert!(!output.rewritten_png_bytes.is_empty());
    assert_ne!(output.rewritten_png_bytes, input);
    assert_eq!(output.verification.format, "png");
    assert_eq!(output.verification.width, 2);
    assert_eq!(output.verification.height, 1);
    assert_eq!(output.verification.redacted_region_count, 1);
    assert_eq!(output.verification.redacted_pixel_count, 1);
    assert_eq!(output.verification.unchanged_pixel_count, 1);
    assert_eq!(
        output.verification.output_byte_count,
        output.rewritten_png_bytes.len() as u64
    );
    assert!(output.verification.verified_changed_pixels_within_regions);

    let decoded =
        image::load_from_memory_with_format(&output.rewritten_png_bytes, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
    assert_eq!(decoded.get_pixel(0, 0).0, [0, 0, 0, 255]);
    assert_eq!(decoded.get_pixel(1, 0).0, [20, 21, 22, 255]);

    let debug = format!("{output:?}");
    assert!(debug.contains("[REDACTED]"), "{debug}");
    assert!(!debug.contains("rewritten_png_bytes: ["), "{debug}");
}

#[test]
fn visual_redaction_service_maps_malformed_png_without_raw_source_names() {
    let service = VisualRedactionService;
    let region = ImageRedactionRegion::new(0, 0, 1, 1).unwrap();

    let err = service
        .redact_png_bytes(b"Patient Jane Example.png is not a png", &[region])
        .unwrap_err();

    assert!(matches!(err, ApplicationError::VisualRedaction(_)));
    let rendered = err.to_string();
    assert!(rendered.contains("visual redaction failed"), "{rendered}");
    assert!(!rendered.contains("Patient Jane Example.png"), "{rendered}");
    assert!(!rendered.contains("not a png"), "{rendered}");
}

#[test]
fn visual_redaction_error_mapping_distinguishes_malformed_png_from_ppm() {
    let mapped = VisualRedactionError::from(ImageRedactionError::MalformedPng);

    assert_eq!(mapped, VisualRedactionError::MalformedPng);
    assert_eq!(mapped.to_string(), "malformed or unsupported PNG image");
}

#[test]
fn visual_redaction_service_rejects_empty_regions_without_returning_unredacted_bytes() {
    let service = VisualRedactionService;

    let err = service.redact_ppm_p6_bytes(&sample_ppm(), &[]).unwrap_err();

    assert!(matches!(err, ApplicationError::VisualRedaction(_)));
    let rendered = err.to_string();
    assert!(rendered.contains("visual redaction failed"), "{rendered}");
    assert!(
        rendered.contains("at least one redaction region"),
        "{rendered}"
    );
    assert!(!rendered.contains("P6"), "{rendered}");
    assert!(!rendered.contains("patient-face.ppm"), "{rendered}");
}

#[test]
fn visual_redaction_service_maps_out_of_bounds_without_raw_source_names() {
    let service = VisualRedactionService;
    let region = ImageRedactionRegion::new(2, 1, 2, 1).unwrap();

    let err = service
        .redact_ppm_p6_bytes(&sample_ppm(), &[region])
        .unwrap_err();

    assert!(matches!(err, ApplicationError::VisualRedaction(_)));
    let rendered = err.to_string();
    assert!(rendered.contains("visual redaction failed"), "{rendered}");
    assert!(!rendered.contains("patient-face.ppm"), "{rendered}");
    assert!(!rendered.contains("/tmp"), "{rendered}");
}
