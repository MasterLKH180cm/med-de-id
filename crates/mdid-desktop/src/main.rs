use mdid_desktop::{
    DesktopFileImportPayload, DesktopFileImportTarget, DesktopPortableMode,
    DesktopPortableRequestState, DesktopRuntimeSettings, DesktopRuntimeSubmissionMode,
    DesktopRuntimeSubmissionSnapshot, DesktopRuntimeSubmitError, DesktopVaultMode,
    DesktopVaultRequestState, DesktopVaultResponseState, DesktopWorkflowMode,
    DesktopWorkflowRequestState, DesktopWorkflowResponseState,
};
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

type RuntimeSubmissionResult = Result<serde_json::Value, DesktopRuntimeSubmitError>;

const DROPPED_FILE_READ_ERROR: &str = "file import failed: unable to read dropped file";

fn read_dropped_file_path_bounded(path: &Path) -> Result<Vec<u8>, &'static str> {
    let metadata = std::fs::metadata(path).map_err(|_| DROPPED_FILE_READ_ERROR)?;
    if metadata.len() > mdid_desktop::DESKTOP_FILE_IMPORT_MAX_BYTES as u64 {
        return Err("file import failed: FileTooLarge");
    }

    std::fs::read(path).map_err(|_| DROPPED_FILE_READ_ERROR)
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "med-de-id desktop workstation",
        options,
        Box::new(|_cc| Box::<DesktopApp>::default()),
    )
}

#[derive(Default)]
struct DesktopApp {
    request_state: DesktopWorkflowRequestState,
    vault_request_state: DesktopVaultRequestState,
    portable_request_state: DesktopPortableRequestState,
    runtime_settings: DesktopRuntimeSettings,
    response_state: DesktopWorkflowResponseState,
    vault_response_state: DesktopVaultResponseState,
    runtime_submission_receiver: Option<Receiver<RuntimeSubmissionResult>>,
    runtime_submission_mode: Option<DesktopRuntimeSubmissionMode>,
}

impl DesktopApp {
    fn runtime_submission_snapshot(&self) -> DesktopRuntimeSubmissionSnapshot {
        match self.runtime_submission_mode {
            Some(mode) if self.runtime_submission_receiver.is_some() => {
                DesktopRuntimeSubmissionSnapshot::started(mode)
            }
            _ => DesktopRuntimeSubmissionSnapshot::idle(),
        }
    }

    fn poll_runtime_submission(&mut self) {
        let Some(receiver) = self.runtime_submission_receiver.take() else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(envelope)) => {
                let mode = self.runtime_submission_mode.take().unwrap_or(
                    DesktopRuntimeSubmissionMode::Workflow(self.request_state.mode),
                );
                if let Some(vault_mode) = mode.vault_response_mode() {
                    self.vault_response_state
                        .apply_success(vault_mode, &envelope);
                } else if let DesktopRuntimeSubmissionMode::Workflow(workflow_mode) = mode {
                    self.response_state
                        .apply_success_json(workflow_mode, envelope);
                }
            }
            Ok(Err(error)) => {
                let mode = self.runtime_submission_mode.take();
                if let Some(vault_mode) =
                    mode.and_then(DesktopRuntimeSubmissionMode::vault_response_mode)
                {
                    self.vault_response_state
                        .apply_error(vault_mode, format!("{error:?}"));
                } else {
                    self.response_state.apply_error(format!("{error:?}"));
                }
            }
            Err(TryRecvError::Empty) => {
                self.runtime_submission_receiver = Some(receiver);
            }
            Err(TryRecvError::Disconnected) => {
                let mode = self.runtime_submission_mode.take();
                if let Some(vault_mode) =
                    mode.and_then(DesktopRuntimeSubmissionMode::vault_response_mode)
                {
                    self.vault_response_state
                        .apply_error(vault_mode, "runtime submission worker disconnected");
                } else {
                    self.response_state
                        .apply_error("runtime submission worker disconnected".to_string());
                }
            }
        }
    }
    fn apply_file_import_target(&mut self, imported: DesktopFileImportTarget) {
        match imported {
            DesktopFileImportTarget::Workflow(payload) => {
                self.request_state.apply_imported_file(payload);
            }
            DesktopFileImportTarget::PortableArtifactInspect(payload) => {
                self.portable_request_state.mode = payload.mode;
                self.portable_request_state.artifact_json = payload.artifact_json;
            }
        }
    }

    fn import_dropped_files(&mut self, ctx: &egui::Context) {
        let files = ctx.input(|input| input.raw.dropped_files.clone());
        for file in files {
            let source_name = file
                .path
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
                .unwrap_or(file.name);
            let bytes = if let Some(bytes) = file.bytes {
                bytes.to_vec()
            } else if let Some(path) = file.path {
                match read_dropped_file_path_bounded(&path) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        self.response_state.apply_error(error.to_string());
                        continue;
                    }
                }
            } else {
                self.response_state
                    .apply_error("file import failed: no file bytes available".to_string());
                continue;
            };

            match DesktopFileImportPayload::from_bytes_target(source_name, &bytes) {
                Ok(imported) => self.apply_file_import_target(imported),
                Err(error) => self
                    .response_state
                    .apply_error(format!("file import failed: {error:?}")),
            }
        }
    }
    fn start_runtime_submission(
        &mut self,
        mode: DesktopRuntimeSubmissionMode,
        request: mdid_desktop::DesktopWorkflowRequest,
    ) {
        match self.runtime_settings.client() {
            Ok(client) => {
                let route = request.route;
                let (sender, receiver) = std::sync::mpsc::channel();
                self.runtime_submission_receiver = Some(receiver);
                self.runtime_submission_mode = Some(mode);
                if mode.vault_response_mode().is_some() {
                    self.vault_response_state.banner =
                        format!("Submitting {route} to local runtime...");
                    self.vault_response_state.error = None;
                } else {
                    self.response_state.banner = format!("Submitting {route} to local runtime...");
                    self.response_state.error = None;
                }
                std::thread::spawn(move || {
                    let _ = sender.send(client.submit(&request));
                });
            }
            Err(error) => {
                if let Some(vault_mode) = mode.vault_response_mode() {
                    self.vault_response_state
                        .apply_error(vault_mode, format!("{error:?}"));
                } else {
                    self.response_state.apply_error(format!("{error:?}"));
                }
            }
        }
    }
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_runtime_submission();
        self.import_dropped_files(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("med-de-id desktop workstation");
            ui.label("Bounded sensitive-workstation request preparation for local runtime routes.");

            egui::ComboBox::from_label("mode")
                .selected_text(self.request_state.mode.label())
                .show_ui(ui, |ui| {
                    for mode in DesktopWorkflowMode::ALL {
                        ui.selectable_value(&mut self.request_state.mode, mode, mode.label());
                    }
                });

            ui.separator();
            ui.label(self.request_state.mode.disclosure());
            ui.label(format!(
                "Route preview: {}",
                self.request_state.mode.endpoint()
            ));

            ui.label("Payload");
            ui.label(self.request_state.mode.payload_hint());
            ui.add(
                egui::TextEdit::multiline(&mut self.request_state.payload)
                    .desired_rows(10)
                    .hint_text(self.request_state.mode.payload_hint()),
            );

            match self.request_state.mode {
                DesktopWorkflowMode::CsvText | DesktopWorkflowMode::XlsxBase64 => {
                    ui.label("Field policy JSON");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.request_state.field_policy_json)
                            .desired_rows(6),
                    );
                }
                DesktopWorkflowMode::PdfBase64Review
                | DesktopWorkflowMode::DicomBase64
                | DesktopWorkflowMode::MediaMetadataJson => {
                    ui.label("Source name");
                    ui.text_edit_singleline(&mut self.request_state.source_name);
                }
            }

            ui.separator();
            ui.label(self.request_state.status_message());
            ui.horizontal(|ui| {
                ui.label("Runtime host");
                ui.text_edit_singleline(&mut self.runtime_settings.host);
                ui.label("port");
                ui.text_edit_singleline(&mut self.runtime_settings.port_text);
            });
            let submission = self.runtime_submission_snapshot();
            if let Some(progress) = submission.progress_banner() {
                ui.label(progress);
            }
            let submit_clicked = ui
                .add_enabled(
                    !submission.submit_button_disabled(),
                    egui::Button::new(submission.submit_button_label()),
                )
                .clicked();
            if submit_clicked {
                match self.request_state.try_build_request() {
                    Ok(request) => self.start_runtime_submission(
                        DesktopRuntimeSubmissionMode::Workflow(self.request_state.mode),
                        request,
                    ),
                    Err(error) => self.response_state.apply_error(format!("{error}")),
                }
            }

            ui.separator();
            ui.heading("Vault decode/audit workbench");
            ui.label(mdid_desktop::DESKTOP_VAULT_WORKBENCH_COPY);
            egui::ComboBox::from_label("vault mode")
                .selected_text(match self.vault_request_state.mode {
                    DesktopVaultMode::Decode => "Vault decode",
                    DesktopVaultMode::AuditEvents => "Vault audit events",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.vault_request_state.mode,
                        DesktopVaultMode::Decode,
                        "Vault decode",
                    );
                    ui.selectable_value(
                        &mut self.vault_request_state.mode,
                        DesktopVaultMode::AuditEvents,
                        "Vault audit events",
                    );
                });
            ui.label(format!("Route preview: {}", self.vault_request_state.mode.route()));
            ui.label("Vault path");
            ui.text_edit_singleline(&mut self.vault_request_state.vault_path);
            ui.label("Vault passphrase");
            ui.add(egui::TextEdit::singleline(&mut self.vault_request_state.vault_passphrase).password(true));
            match self.vault_request_state.mode {
                DesktopVaultMode::Decode => {
                    ui.label("Record IDs JSON");
                    ui.add(egui::TextEdit::multiline(&mut self.vault_request_state.record_ids_json).desired_rows(3));
                    ui.label("Output target");
                    ui.text_edit_singleline(&mut self.vault_request_state.output_target);
                    ui.label("Justification");
                    ui.text_edit_singleline(&mut self.vault_request_state.justification);
                    ui.label("Requested by");
                    ui.text_edit_singleline(&mut self.vault_request_state.requested_by);
                }
                DesktopVaultMode::AuditEvents => {
                    let kind = self.vault_request_state.audit_kind.get_or_insert_with(String::new);
                    ui.label("Audit kind filter");
                    ui.text_edit_singleline(kind);
                    let actor = self.vault_request_state.audit_actor.get_or_insert_with(String::new);
                    ui.label("Audit actor filter");
                    ui.text_edit_singleline(actor);
                }
            }
            if ui
                .add_enabled(
                    !submission.submit_button_disabled(),
                    egui::Button::new("Submit vault request to local runtime"),
                )
                .clicked()
            {
                match self.vault_request_state.try_build_request() {
                    Ok(request) => self.start_runtime_submission(
                        DesktopRuntimeSubmissionMode::Vault(self.vault_request_state.mode),
                        request,
                    ),
                    Err(error) => self.vault_response_state.apply_error(
                        match self.vault_request_state.mode {
                            DesktopVaultMode::Decode => mdid_desktop::DesktopVaultResponseMode::VaultDecode,
                            DesktopVaultMode::AuditEvents => mdid_desktop::DesktopVaultResponseMode::VaultAudit,
                        },
                        format!("{error:?}"),
                    ),
                }
            }

            ui.separator();
            ui.heading("Portable artifact workbench");
            egui::ComboBox::from_label("portable mode")
                .selected_text(match self.portable_request_state.mode {
                    DesktopPortableMode::VaultExport => "Vault export",
                    DesktopPortableMode::InspectArtifact => "Inspect artifact",
                    DesktopPortableMode::ImportArtifact => "Import artifact",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.portable_request_state.mode, DesktopPortableMode::VaultExport, "Vault export");
                    ui.selectable_value(&mut self.portable_request_state.mode, DesktopPortableMode::InspectArtifact, "Inspect artifact");
                    ui.selectable_value(&mut self.portable_request_state.mode, DesktopPortableMode::ImportArtifact, "Import artifact");
                });
            ui.label(format!("Route preview: {}", self.portable_request_state.mode.route()));
            match self.portable_request_state.mode {
                DesktopPortableMode::VaultExport => {
                    ui.label("Vault path");
                    ui.text_edit_singleline(&mut self.portable_request_state.vault_path);
                    ui.label("Vault passphrase");
                    ui.add(egui::TextEdit::singleline(&mut self.portable_request_state.vault_passphrase).password(true));
                    ui.label("Record IDs JSON");
                    ui.add(egui::TextEdit::multiline(&mut self.portable_request_state.record_ids_json).desired_rows(3));
                    ui.label("Export passphrase");
                    ui.add(egui::TextEdit::singleline(&mut self.portable_request_state.export_passphrase).password(true));
                    ui.label("Export context");
                    ui.text_edit_singleline(&mut self.portable_request_state.export_context);
                }
                DesktopPortableMode::InspectArtifact => {
                    ui.label("Artifact JSON (not rendered after submission)");
                    ui.add(egui::TextEdit::multiline(&mut self.portable_request_state.artifact_json).desired_rows(5));
                    ui.label("Portable passphrase");
                    ui.add(egui::TextEdit::singleline(&mut self.portable_request_state.portable_passphrase).password(true));
                }
                DesktopPortableMode::ImportArtifact => {
                    ui.label("Destination vault path");
                    ui.text_edit_singleline(&mut self.portable_request_state.destination_vault_path);
                    ui.label("Destination vault passphrase");
                    ui.add(egui::TextEdit::singleline(&mut self.portable_request_state.destination_vault_passphrase).password(true));
                    ui.label("Artifact JSON (not rendered after submission)");
                    ui.add(egui::TextEdit::multiline(&mut self.portable_request_state.artifact_json).desired_rows(5));
                    ui.label("Portable passphrase");
                    ui.add(egui::TextEdit::singleline(&mut self.portable_request_state.portable_passphrase).password(true));
                    ui.label("Import context");
                    ui.text_edit_singleline(&mut self.portable_request_state.import_context);
                }
            }
            ui.label("Requested by");
            ui.text_edit_singleline(&mut self.portable_request_state.requested_by);
            if ui
                .add_enabled(
                    !submission.submit_button_disabled(),
                    egui::Button::new("Submit portable request to local runtime"),
                )
                .clicked()
            {
                match self.portable_request_state.try_build_request() {
                    Ok(request) => self.start_runtime_submission(
                        DesktopRuntimeSubmissionMode::Portable(self.portable_request_state.mode),
                        request,
                    ),
                    Err(error) => {
                        let mode = DesktopRuntimeSubmissionMode::Portable(self.portable_request_state.mode)
                            .vault_response_mode()
                            .expect("portable response mode");
                        self.vault_response_state.apply_error(mode, format!("{error:?}"));
                    }
                }
            }

            ui.separator();
            ui.heading("Vault/portable response workbench");
            ui.label(&self.vault_response_state.banner);
            if let Some(error) = &self.vault_response_state.error {
                ui.colored_label(egui::Color32::RED, error);
            }
            ui.label("Safe summary");
            let mut vault_summary = self.vault_response_state.summary.clone();
            ui.add(egui::TextEdit::multiline(&mut vault_summary).desired_rows(3).interactive(false));
            if !self.vault_response_state.artifact_notice.is_empty() {
                ui.label(&self.vault_response_state.artifact_notice);
            }
            ui.separator();
            ui.heading("Runtime-shaped response workbench");
            ui.label(&self.response_state.banner);

            if let Some(error) = &self.response_state.error {
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.label("Summary");
            let mut summary = self.response_state.summary.clone();
            ui.add(
                egui::TextEdit::multiline(&mut summary)
                    .desired_rows(6)
                    .interactive(false),
            );

            ui.label("Review queue");
            let mut review_queue = self.response_state.review_queue.clone();
            ui.add(
                egui::TextEdit::multiline(&mut review_queue)
                    .desired_rows(6)
                    .interactive(false),
            );

            ui.label("Rewritten output / review notice");
            let mut output = self.response_state.output.clone();
            ui.add(
                egui::TextEdit::multiline(&mut output)
                    .desired_rows(8)
                    .interactive(false),
            );

            ui.label(
                "Not implemented in this desktop slice: file picker upload/download UX beyond bounded helper import/export, vault browsing, full decode workflow execution UX, audit investigation, OCR, visual redaction, PDF rewrite/export, and full review workflows.",
            );
        });
        self.poll_runtime_submission();
        if self.runtime_submission_receiver.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_disconnected_submission(mode: DesktopRuntimeSubmissionMode) -> DesktopApp {
        let (_sender, receiver) = std::sync::mpsc::channel();
        drop(_sender);
        DesktopApp {
            runtime_submission_receiver: Some(receiver),
            runtime_submission_mode: Some(mode),
            ..DesktopApp::default()
        }
    }

    #[test]
    fn path_backed_file_import_rejects_large_file_before_read() {
        let path = std::env::temp_dir().join(format!(
            "mdid-large-dropped-file-{}.csv",
            uuid::Uuid::new_v4()
        ));
        let file = std::fs::File::create(&path).expect("create temp dropped file");
        file.set_len((mdid_desktop::DESKTOP_FILE_IMPORT_MAX_BYTES + 1) as u64)
            .expect("size temp dropped file");

        let error = read_dropped_file_path_bounded(&path).expect_err("large file rejected");

        assert_eq!(error, "file import failed: FileTooLarge");
        std::fs::remove_file(path).expect("remove temp dropped file");
    }

    #[test]
    fn path_backed_file_import_read_error_is_phi_safe() {
        let path = std::env::temp_dir().join("patient-jane-doe-mrn-12345-missing-dropped-file.csv");

        let error = read_dropped_file_path_bounded(&path).expect_err("missing file rejected");

        assert_eq!(error, "file import failed: unable to read dropped file");
        assert!(!error.contains(path.to_string_lossy().as_ref()));
        assert!(!error.contains("jane-doe"));
        assert!(!error.contains("12345"));
    }

    #[test]
    fn app_file_import_target_populates_portable_artifact_inspect_state() {
        let artifact_json = r#"{"artifact":{"ciphertext":"secret"}}"#;
        let imported = mdid_desktop::DesktopFileImportPayload::from_bytes_target(
            "mdid-browser-portable-artifact.json",
            artifact_json.as_bytes(),
        )
        .expect("portable artifact import target");
        let mut app = DesktopApp::default();

        app.apply_file_import_target(imported);

        assert_eq!(
            app.portable_request_state.mode,
            DesktopPortableMode::InspectArtifact
        );
        assert_eq!(app.portable_request_state.artifact_json, artifact_json);
        assert_eq!(app.request_state.mode, DesktopWorkflowMode::CsvText);
        assert!(app.request_state.payload.is_empty());
    }

    #[test]
    fn disconnected_vault_submission_error_is_routed_to_vault_response_state() {
        let mut app = app_with_disconnected_submission(DesktopRuntimeSubmissionMode::Vault(
            DesktopVaultMode::Decode,
        ));

        app.poll_runtime_submission();

        assert_eq!(app.runtime_submission_mode, None);
        assert_eq!(app.response_state.error, None);
        assert_eq!(
            app.vault_response_state.banner,
            "bounded vault decode response rendered locally"
        );
        assert_eq!(
            app.vault_response_state.error.as_deref(),
            Some("runtime failed; details redacted")
        );
    }

    #[test]
    fn disconnected_portable_submission_error_is_routed_to_vault_response_state() {
        let mut app = app_with_disconnected_submission(DesktopRuntimeSubmissionMode::Portable(
            DesktopPortableMode::InspectArtifact,
        ));

        app.poll_runtime_submission();

        assert_eq!(app.runtime_submission_mode, None);
        assert_eq!(app.response_state.error, None);
        assert_eq!(
            app.vault_response_state.banner,
            "bounded portable artifact response rendered locally"
        );
        assert_eq!(
            app.vault_response_state.error.as_deref(),
            Some("runtime failed; details redacted")
        );
    }

    #[test]
    fn disconnected_workflow_submission_error_is_routed_to_workflow_response_state() {
        let mut app = app_with_disconnected_submission(DesktopRuntimeSubmissionMode::Workflow(
            DesktopWorkflowMode::CsvText,
        ));

        app.poll_runtime_submission();

        assert_eq!(app.runtime_submission_mode, None);
        assert_eq!(
            app.response_state.error.as_deref(),
            Some("runtime submission worker disconnected")
        );
        assert_eq!(app.vault_response_state.error, None);
    }
}
