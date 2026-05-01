use mdid_application::{ApplicationError, VisualRedactionService};
use mdid_domain::ImageRedactionRegion;

fn sample_ppm() -> Vec<u8> {
    let mut ppm = b"P6\n3 2\n255\n".to_vec();
    ppm.extend_from_slice(&[
        10, 11, 12, 20, 21, 22, 30, 31, 32, 40, 41, 42, 50, 51, 52, 60, 61, 62,
    ]);
    ppm
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
