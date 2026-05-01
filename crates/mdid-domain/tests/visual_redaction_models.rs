use mdid_domain::{ImageRedactionRegion, ImageRedactionRegionError};

#[test]
fn image_redaction_region_accepts_positive_bounded_bbox() {
    let region = ImageRedactionRegion::new(1, 2, 3, 4).expect("valid bbox");

    assert_eq!(region.x(), 1);
    assert_eq!(region.y(), 2);
    assert_eq!(region.width(), 3);
    assert_eq!(region.height(), 4);
}

#[test]
fn image_redaction_region_rejects_empty_bbox_dimensions() {
    assert_eq!(
        ImageRedactionRegion::new(0, 0, 0, 1),
        Err(ImageRedactionRegionError::EmptyRegion)
    );
    assert_eq!(
        ImageRedactionRegion::new(0, 0, 1, 0),
        Err(ImageRedactionRegionError::EmptyRegion)
    );
}

#[test]
fn image_redaction_region_deserialize_rejects_empty_bbox_dimensions() {
    let zero_width =
        serde_json::from_str::<ImageRedactionRegion>(r#"{"x":0,"y":0,"width":0,"height":1}"#);
    let zero_height =
        serde_json::from_str::<ImageRedactionRegion>(r#"{"x":0,"y":0,"width":1,"height":0}"#);

    assert!(zero_width.is_err());
    assert!(zero_height.is_err());
}

#[test]
fn image_redaction_region_debug_is_coordinate_only() {
    let region = ImageRedactionRegion::new(4, 5, 6, 7).expect("valid bbox");
    let debug = format!("{region:?}");

    assert!(debug.contains("ImageRedactionRegion"));
    assert!(debug.contains("x"));
    assert!(!debug.contains("patient.png"));
    assert!(!debug.contains("[255, 0, 0]"));
}
