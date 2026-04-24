use mdid_domain::{AuditEventKind, DecodeRequest, DecodeRequestError, MappingScope, SurfaceKind};
use serde_json::{from_str, to_string};
use uuid::Uuid;

#[test]
fn audit_event_kind_flags_decode_as_high_risk() {
    assert_eq!(AuditEventKind::Encode.as_str(), "encode");
    assert_eq!(AuditEventKind::Decode.as_str(), "decode");
    assert!(!AuditEventKind::Encode.is_high_risk());
    assert!(AuditEventKind::Decode.is_high_risk());
}

#[test]
fn mapping_scope_builds_a_stable_scope_key() {
    let scope = MappingScope::new(
        Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
        "patient.name".into(),
    );

    assert_eq!(
        scope.scope_key(),
        "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb/patient.name"
    );
}

#[test]
fn decode_request_requires_scope_and_justification() {
    let err = DecodeRequest::new(
        vec![],
        "stdout".into(),
        "incident triage".into(),
        SurfaceKind::Desktop,
    )
    .unwrap_err();
    assert!(matches!(err, DecodeRequestError::EmptyScope));

    let err = DecodeRequest::new(
        vec![Uuid::new_v4()],
        "stdout".into(),
        "   ".into(),
        SurfaceKind::Desktop,
    )
    .unwrap_err();
    assert!(matches!(err, DecodeRequestError::MissingJustification));
}

#[test]
fn decode_request_requires_an_explicit_output_target() {
    let err = DecodeRequest::new(
        vec![Uuid::new_v4()],
        "   ".into(),
        "incident triage".into(),
        SurfaceKind::Desktop,
    )
    .unwrap_err();

    assert!(matches!(err, DecodeRequestError::MissingOutputTarget));
}

#[test]
fn decode_request_exposes_validated_fields_via_accessors() {
    let record_id = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();

    let request = DecodeRequest::new(
        vec![record_id],
        "stdout".into(),
        "incident triage".into(),
        SurfaceKind::Desktop,
    )
    .unwrap();

    assert_eq!(request.record_ids(), &[record_id]);
    assert_eq!(request.output_target(), "stdout");
    assert_eq!(request.justification(), "incident triage");
    assert_eq!(request.requested_by(), SurfaceKind::Desktop);
}

#[test]
fn serde_uses_stable_lowercase_wire_values_for_surface_and_audit_kinds() {
    assert_eq!(to_string(&SurfaceKind::Desktop).unwrap(), "\"desktop\"");
    assert_eq!(to_string(&AuditEventKind::Decode).unwrap(), "\"decode\"");
    assert_eq!(
        from_str::<SurfaceKind>("\"browser\"").unwrap(),
        SurfaceKind::Browser
    );
    assert_eq!(
        from_str::<AuditEventKind>("\"encode\"").unwrap(),
        AuditEventKind::Encode
    );
}
