use mdid_adapters::{ExtractedTabularData, FieldPolicy};
use mdid_application::TabularDeidentificationService;
use mdid_domain::{
    PhiCandidate, ReviewDecision, SurfaceKind, TabularCellRef, TabularColumn, TabularFormat,
};
use mdid_vault::LocalVaultStore;
use tempfile::tempdir;

#[test]
fn csv_deidentification_reuses_tokens_and_reports_review_items() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = TabularDeidentificationService;
    let policies = vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ];

    let output = service
        .deidentify_csv(
            "patient_id,patient_name\nMRN-001,Alice Smith\nMRN-001,Alice Smith\n",
            &policies,
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();

    let lines = output
        .csv
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect::<Vec<_>>();

    assert_eq!(lines[1], lines[2]);
    assert!(lines[1].starts_with("tok-"));
    assert!(lines[1].contains(",Alice Smith"));
    assert!(!lines[1].contains("MRN-001"));
    assert_eq!(output.summary.total_rows, 2);
    assert_eq!(output.summary.encoded_cells, 2);
    assert_eq!(output.summary.review_required_cells, 2);
    assert_eq!(output.summary.failed_rows, 0);
    assert_eq!(output.review_queue.len(), 2);
    assert!(output
        .review_queue
        .iter()
        .all(|candidate| candidate.decision == ReviewDecision::NeedsReview));
    assert_eq!(vault.audit_events().len(), 2);
}

#[test]
fn deidentification_reports_partial_failure_when_a_row_cannot_be_rewritten() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = TabularDeidentificationService;
    let column = TabularColumn::new(0, "patient_id".into(), "string".into());

    let output = service
        .deidentify_extracted(
            ExtractedTabularData {
                format: TabularFormat::Csv,
                columns: vec![column.clone()],
                rows: vec![vec!["MRN-001".into()]],
                candidates: vec![PhiCandidate {
                    format: TabularFormat::Csv,
                    column,
                    cell: TabularCellRef::new(0, 1, "patient_id".into()),
                    phi_type: "patient_id".into(),
                    value: "MRN-001".into(),
                    confidence: 100,
                    decision: ReviewDecision::Approved,
                }],
            },
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();

    let lines = output
        .csv
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect::<Vec<_>>();

    assert_eq!(lines, vec!["patient_id", "MRN-001"]);
    assert_eq!(output.summary.total_rows, 1);
    assert_eq!(output.summary.encoded_cells, 0);
    assert_eq!(output.summary.review_required_cells, 0);
    assert_eq!(output.summary.failed_rows, 1);
    assert!(output.summary.is_partial_failure());
    assert!(output.review_queue.is_empty());
    assert!(vault.audit_events().is_empty());
}

#[test]
fn deidentification_ignores_out_of_range_candidates_in_failure_summary() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = TabularDeidentificationService;
    let column = TabularColumn::new(0, "patient_id".into(), "string".into());

    let output = service
        .deidentify_extracted(
            ExtractedTabularData {
                format: TabularFormat::Csv,
                columns: vec![column.clone()],
                rows: vec![vec!["MRN-001".into()]],
                candidates: vec![
                    PhiCandidate {
                        format: TabularFormat::Csv,
                        column: column.clone(),
                        cell: TabularCellRef::new(2, 0, "patient_id".into()),
                        phi_type: "patient_id".into(),
                        value: "MRN-BOGUS-002".into(),
                        confidence: 100,
                        decision: ReviewDecision::Approved,
                    },
                    PhiCandidate {
                        format: TabularFormat::Csv,
                        column,
                        cell: TabularCellRef::new(4, 0, "patient_id".into()),
                        phi_type: "patient_id".into(),
                        value: "MRN-BOGUS-004".into(),
                        confidence: 100,
                        decision: ReviewDecision::Approved,
                    },
                ],
            },
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();

    assert_eq!(output.summary.total_rows, 1);
    assert_eq!(output.summary.failed_rows, 0);
    assert!(output.summary.failed_rows <= output.summary.total_rows);
    assert_eq!(output.csv, "patient_id\nMRN-001\n");
    assert!(vault.audit_events().is_empty());
}

#[test]
fn tabular_deidentification_output_debug_redacts_phi() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = TabularDeidentificationService;
    let policies = vec![FieldPolicy::review("patient_name", "patient_name")];

    let output = service
        .deidentify_csv(
            "patient_name\nAlice Smith\n",
            &policies,
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();

    let debug = format!("{output:?}");

    assert!(debug.contains("TabularDeidentificationOutput"));
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("Alice Smith"));
    assert!(!debug.contains("patient_name\\nAlice Smith"));
}
