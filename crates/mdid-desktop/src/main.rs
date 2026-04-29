use mdid_desktop::{
    write_portable_artifact_json, write_safe_vault_response_json, DesktopFileImportPayload,
    DesktopFileImportTarget, DesktopPortableFileImportPayload, DesktopPortableMode,
    DesktopPortableRequestState, DesktopRuntimeSettings, DesktopRuntimeSubmissionMode,
    DesktopRuntimeSubmissionSnapshot, DesktopRuntimeSubmitError, DesktopVaultMode,
    DesktopVaultRequestState, DesktopVaultResponseMode, DesktopVaultResponseState,
    DesktopWorkflowMode, DesktopWorkflowRequestState, DesktopWorkflowResponseState,
};
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

type RuntimeSubmissionResult = Result<serde_json::Value, DesktopRuntimeSubmitError>;

const DROPPED_FILE_READ_ERROR: &str = "file import failed: unable to read dropped file";
const DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH: &str = "desktop-deidentified-output.bin";
const DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH: &str = "desktop-vault-response-report.json";

fn is_replaceable_workflow_output_save_path(path: &str, generated_path: Option<&str>) -> bool {
    let path = path.trim();
    path.is_empty()
        || path == DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH
        || generated_path.is_some_and(|generated| path == generated.trim())
}

fn is_replaceable_vault_response_report_save_path(
    path: &str,
    generated_path: Option<&str>,
) -> bool {
    let path = path.trim();
    path.is_empty()
        || path == DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH
        || generated_path.is_some_and(|generated| path == generated.trim())
}

fn next_vault_response_report_save_path(
    current_path: &str,
    generated_path: Option<&str>,
    portable_source_name: Option<&str>,
    state: &DesktopVaultResponseState,
) -> (String, Option<String>) {
    if !is_replaceable_vault_response_report_save_path(current_path, generated_path) {
        return (current_path.to_string(), None);
    }

    let next_path = state
        .safe_response_report_download_for_source(portable_source_name)
        .map(|download| download.file_name)
        .unwrap_or_else(|_| DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH.to_string());
    let next_generated_path =
        (next_path != DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH).then(|| next_path.clone());
    (next_path, next_generated_path)
}

fn read_dropped_file_path_bounded(path: &Path) -> Result<Vec<u8>, &'static str> {
    let metadata = std::fs::metadata(path).map_err(|_| DROPPED_FILE_READ_ERROR)?;
    if metadata.len() > mdid_desktop::DESKTOP_FILE_IMPORT_MAX_BYTES as u64 {
        return Err("file import failed: FileTooLarge");
    }

    std::fs::read(path).map_err(|_| DROPPED_FILE_READ_ERROR)
}

fn is_portable_artifact_drop_source_name(source_name: &str) -> bool {
    let filename = source_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(source_name)
        .to_ascii_lowercase();

    filename == "mdid-browser-portable-artifact.json"
        || filename.ends_with(".mdid-portable.json")
        || filename.ends_with("-mdid-portable.json")
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "med-de-id desktop workstation",
        options,
        Box::new(|_cc| Box::<DesktopApp>::default()),
    )
}

struct DesktopApp {
    request_state: DesktopWorkflowRequestState,
    vault_request_state: DesktopVaultRequestState,
    portable_request_state: DesktopPortableRequestState,
    runtime_settings: DesktopRuntimeSettings,
    response_state: DesktopWorkflowResponseState,
    vault_response_state: DesktopVaultResponseState,
    runtime_submission_receiver: Option<Receiver<RuntimeSubmissionResult>>,
    runtime_submission_mode: Option<DesktopRuntimeSubmissionMode>,
    workflow_output_save_path: String,
    generated_workflow_output_save_path: Option<String>,
    workflow_output_save_status: String,
    portable_artifact_save_path: String,
    portable_artifact_save_status: String,
    vault_response_report_save_path: String,
    generated_vault_response_report_save_path: Option<String>,
    vault_response_report_save_status: String,
    portable_response_report_source_name: Option<String>,
}

impl Default for DesktopApp {
    fn default() -> Self {
        Self {
            request_state: DesktopWorkflowRequestState::default(),
            vault_request_state: DesktopVaultRequestState::default(),
            portable_request_state: DesktopPortableRequestState::default(),
            runtime_settings: DesktopRuntimeSettings::default(),
            response_state: DesktopWorkflowResponseState::default(),
            vault_response_state: DesktopVaultResponseState::default(),
            runtime_submission_receiver: None,
            runtime_submission_mode: None,
            workflow_output_save_path: DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH.to_string(),
            generated_workflow_output_save_path: None,
            workflow_output_save_status: String::new(),
            portable_artifact_save_path: "desktop-portable-artifact.mdid-portable.json".to_string(),
            portable_artifact_save_status: String::new(),
            vault_response_report_save_path: DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH.to_string(),
            generated_vault_response_report_save_path: None,
            vault_response_report_save_status: String::new(),
            portable_response_report_source_name: None,
        }
    }
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
                    let (next_path, generated_path) = next_vault_response_report_save_path(
                        &self.vault_response_report_save_path,
                        self.generated_vault_response_report_save_path.as_deref(),
                        self.portable_response_report_source_name.as_deref(),
                        &self.vault_response_state,
                    );
                    self.vault_response_report_save_path = next_path;
                    self.generated_vault_response_report_save_path = generated_path;
                } else if let DesktopRuntimeSubmissionMode::Workflow(workflow_mode) = mode {
                    self.response_state
                        .apply_success_json(workflow_mode, envelope);
                    self.refresh_workflow_output_save_path(workflow_mode);
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
                self.portable_response_report_source_name = None;
                self.request_state.apply_imported_file(payload);
            }
            DesktopFileImportTarget::PortableArtifactInspect(payload) => {
                self.portable_response_report_source_name = Some(payload.source_name.clone());
                self.portable_request_state.mode = payload.mode;
                self.portable_request_state.artifact_json = payload.artifact_json;
            }
        }
    }

    fn import_file_bytes_for_current_state(&mut self, source_name: String, bytes: &[u8]) {
        let imported = if is_portable_artifact_drop_source_name(&source_name)
            && matches!(
                self.portable_request_state.mode,
                DesktopPortableMode::InspectArtifact | DesktopPortableMode::ImportArtifact
            ) {
            DesktopPortableFileImportPayload::from_bytes_for_mode(
                self.portable_request_state.mode,
                source_name,
                bytes,
            )
            .map(DesktopFileImportTarget::PortableArtifactInspect)
        } else {
            DesktopFileImportPayload::from_bytes_target(source_name, bytes)
        };

        match imported {
            Ok(imported) => self.apply_file_import_target(imported),
            Err(error) => self
                .response_state
                .apply_error(format!("file import failed: {error:?}")),
        }
    }

    fn save_portable_artifact_response(&mut self) {
        match write_portable_artifact_json(
            &self.vault_response_state,
            self.portable_artifact_save_path.trim(),
        ) {
            Ok(_) => {
                self.portable_artifact_save_status =
                    "Portable artifact JSON saved; encrypted contents only.".to_string();
            }
            Err(error) => {
                self.portable_artifact_save_status = error.to_string();
            }
        }
    }

    fn refresh_workflow_output_save_path(&mut self, mode: DesktopWorkflowMode) {
        if !is_replaceable_workflow_output_save_path(
            &self.workflow_output_save_path,
            self.generated_workflow_output_save_path.as_deref(),
        ) {
            self.generated_workflow_output_save_path = None;
            return;
        }

        let next_path = self
            .response_state
            .workflow_output_download(mode)
            .map(|download| download.file_name.to_string())
            .unwrap_or_else(|| DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH.to_string());
        let next_generated_path =
            (next_path != DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH).then(|| next_path.clone());
        self.workflow_output_save_path = next_path;
        self.generated_workflow_output_save_path = next_generated_path;
    }

    fn save_vault_response_report(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let mode = self
            .vault_response_state
            .safe_response_report_mode()
            .unwrap_or(DesktopVaultResponseMode::VaultAudit);

        write_safe_vault_response_json(&self.vault_response_state, mode, path)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn save_vault_response_report_response(&mut self) {
        match self.save_vault_response_report(self.vault_response_report_save_path.trim()) {
            Ok(()) => {
                self.vault_response_report_save_status =
                    "Safe vault/portable response report saved.".to_string();
            }
            Err(error) => {
                self.vault_response_report_save_status = error;
            }
        }
    }

    fn save_workflow_output(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let download = self
            .response_state
            .workflow_output_download(self.request_state.mode)
            .ok_or_else(|| {
                "workflow output save failed: no rewritten output is available".to_string()
            })?;
        mdid_desktop::write_workflow_output_file(path, &download)
    }

    fn save_workflow_output_response(&mut self) {
        match self.save_workflow_output(self.workflow_output_save_path.trim()) {
            Ok(()) => {
                self.workflow_output_save_status = "Rewritten workflow output saved.".to_string();
            }
            Err(error) => {
                self.workflow_output_save_status = error;
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

            self.import_file_bytes_for_current_state(source_name, &bytes);
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
                    let limit = self.vault_request_state.audit_limit.get_or_insert_with(String::new);
                    ui.label("Audit limit (optional)");
                    ui.text_edit_singleline(limit);
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
            if self
                .vault_response_state
                .portable_artifact_download_json(DesktopVaultResponseMode::VaultExport)
                .is_ok()
            {
                ui.label("Save portable artifact JSON");
                ui.text_edit_singleline(&mut self.portable_artifact_save_path);
                if ui.button("Save portable artifact JSON").clicked() {
                    self.save_portable_artifact_response();
                }
                if !self.portable_artifact_save_status.is_empty() {
                    ui.label(&self.portable_artifact_save_status);
                }
            }
            if self.vault_response_state.safe_response_report_json().is_ok() {
                ui.label("Save safe vault/portable response report JSON");
                ui.text_edit_singleline(&mut self.vault_response_report_save_path);
                if ui
                    .button("Save safe vault/portable response report JSON")
                    .clicked()
                {
                    self.save_vault_response_report_response();
                }
                if !self.vault_response_report_save_status.is_empty() {
                    ui.label(&self.vault_response_report_save_status);
                }
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

            if self
                .response_state
                .workflow_output_download(self.request_state.mode)
                .is_some()
            {
                ui.label("Save rewritten workflow output");
                ui.text_edit_singleline(&mut self.workflow_output_save_path);
                if ui.button("Save rewritten workflow output").clicked() {
                    self.save_workflow_output_response();
                }
                if !self.workflow_output_save_status.is_empty() {
                    ui.label(&self.workflow_output_save_status);
                }
            }

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
    use base64::Engine as _;

    fn app_with_disconnected_submission(mode: DesktopRuntimeSubmissionMode) -> DesktopApp {
        let (_sender, receiver) = std::sync::mpsc::channel();
        drop(_sender);
        DesktopApp {
            runtime_submission_receiver: Some(receiver),
            runtime_submission_mode: Some(mode),
            ..DesktopApp::default()
        }
    }

    fn portable_inspect_report_state() -> DesktopVaultResponseState {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::InspectArtifact,
            &serde_json::json!({
                "record_count": 2,
                "preview": [
                    {"record_id": "record-1"},
                    {"record_id": "record-2"}
                ],
                "artifact_path": "C:\\vaults\\Clinic Batch.mdid-portable.json"
            }),
        );
        state
    }

    #[test]
    fn workflow_output_save_path_refreshes_default_after_csv_success() {
        let mut app = DesktopApp::default();
        app.request_state.mode = DesktopWorkflowMode::CsvText;
        app.response_state.apply_success_json(
            DesktopWorkflowMode::CsvText,
            serde_json::json!({"csv": "name\nTOKEN-1\n", "summary": {}}),
        );

        app.refresh_workflow_output_save_path(DesktopWorkflowMode::CsvText);

        assert_eq!(app.workflow_output_save_path, "desktop-deidentified.csv");
        assert_eq!(
            app.generated_workflow_output_save_path.as_deref(),
            Some("desktop-deidentified.csv")
        );
    }

    #[test]
    fn workflow_output_save_path_refreshes_with_received_mode_not_current_ui_mode() {
        let mut app = DesktopApp::default();
        app.request_state.mode = DesktopWorkflowMode::PdfBase64Review;
        app.response_state.apply_success_json(
            DesktopWorkflowMode::CsvText,
            serde_json::json!({"csv": "name\nTOKEN-1\n", "summary": {}}),
        );

        app.refresh_workflow_output_save_path(DesktopWorkflowMode::CsvText);

        assert_eq!(app.workflow_output_save_path, "desktop-deidentified.csv");
        assert_eq!(
            app.generated_workflow_output_save_path.as_deref(),
            Some("desktop-deidentified.csv")
        );
    }

    #[test]
    fn workflow_output_save_path_preserves_user_override_after_dicom_success() {
        let mut app = DesktopApp {
            workflow_output_save_path: "C:\\exports\\custom-output.dcm".to_string(),
            ..DesktopApp::default()
        };
        app.request_state.mode = DesktopWorkflowMode::DicomBase64;
        app.response_state.apply_success_json(
            DesktopWorkflowMode::DicomBase64,
            serde_json::json!({
                "rewritten_dicom_bytes_base64": base64::engine::general_purpose::STANDARD.encode(b"dicom"),
                "summary": {}
            }),
        );

        app.refresh_workflow_output_save_path(DesktopWorkflowMode::DicomBase64);

        assert_eq!(
            app.workflow_output_save_path,
            "C:\\exports\\custom-output.dcm"
        );
        assert_eq!(app.generated_workflow_output_save_path, None);
    }

    #[test]
    fn workflow_output_save_path_resets_generated_path_when_no_binary_output() {
        let mut app = DesktopApp {
            workflow_output_save_path: "desktop-deidentified.csv".to_string(),
            generated_workflow_output_save_path: Some("desktop-deidentified.csv".to_string()),
            ..DesktopApp::default()
        };
        app.request_state.mode = DesktopWorkflowMode::PdfBase64Review;
        app.response_state.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            serde_json::json!({"summary": {}, "page_statuses": [], "review_queue": []}),
        );

        app.refresh_workflow_output_save_path(DesktopWorkflowMode::PdfBase64Review);

        assert_eq!(
            app.workflow_output_save_path,
            DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH
        );
        assert_eq!(app.generated_workflow_output_save_path, None);
    }

    #[test]
    fn portable_response_report_path_uses_sanitized_imported_source_when_default() {
        assert_eq!(
            next_vault_response_report_save_path(
                DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH,
                None,
                Some("C:\\vaults\\Clinic Batch.mdid-portable.json"),
                &portable_inspect_report_state(),
            ),
            (
                "Clinic-Batch.mdid-portable-response-report.json".to_string(),
                Some("Clinic-Batch.mdid-portable-response-report.json".to_string())
            )
        );
    }

    #[test]
    fn portable_response_report_path_refreshes_previous_generated_portable_path() {
        assert_eq!(
            next_vault_response_report_save_path(
                "Clinic-Batch.mdid-portable-response-report.json",
                Some("Clinic-Batch.mdid-portable-response-report.json"),
                Some("C:\\\\vaults\\\\Partner Export.mdid-portable.json"),
                &portable_inspect_report_state(),
            ),
            (
                "Partner-Export.mdid-portable-response-report.json".to_string(),
                Some("Partner-Export.mdid-portable-response-report.json".to_string())
            )
        );
    }

    #[test]
    fn portable_response_report_path_preserves_generated_shaped_path_without_marker() {
        assert_eq!(
            next_vault_response_report_save_path(
                "Clinic-Batch.mdid-portable-response-report.json",
                None,
                Some("C:\\\\vaults\\\\Partner Export.mdid-portable.json"),
                &portable_inspect_report_state(),
            ),
            (
                "Clinic-Batch.mdid-portable-response-report.json".to_string(),
                None
            )
        );
    }

    #[test]
    fn portable_response_report_path_refreshes_previous_generated_path_for_non_portable_report_modes(
    ) {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultDecode,
            &serde_json::json!({
                "decoded_count": 1,
                "audit_event": {"event_id": "evt-1"},
                "decoded_values": {"record-1": {"name": "Jane Doe"}}
            }),
        );

        assert_eq!(
            next_vault_response_report_save_path(
                "Clinic-Batch.mdid-portable-response-report.json",
                Some("Clinic-Batch.mdid-portable-response-report.json"),
                Some("C:\\\\vaults\\\\Clinic Batch.mdid-portable.json"),
                &state,
            ),
            (
                "Clinic-Batch.mdid-portable-response-report.json".to_string(),
                Some("Clinic-Batch.mdid-portable-response-report.json".to_string())
            )
        );
    }

    #[test]
    fn portable_response_report_path_preserves_user_overridden_path() {
        assert_eq!(
            next_vault_response_report_save_path(
                "C:\\\\exports\\\\custom-report.json",
                None,
                Some("C:\\\\vaults\\\\Clinic Batch.mdid-portable.json"),
                &portable_inspect_report_state(),
            ),
            ("C:\\\\exports\\\\custom-report.json".to_string(), None)
        );
    }

    #[test]
    fn portable_response_report_path_preserves_explicit_relative_user_override() {
        assert_eq!(
            next_vault_response_report_save_path(
                "custom-portable-response-report.json",
                None,
                Some("C:\\\\vaults\\\\Clinic Batch.mdid-portable.json"),
                &portable_inspect_report_state(),
            ),
            ("custom-portable-response-report.json".to_string(), None)
        );
    }

    #[test]
    fn portable_response_report_path_uses_sanitized_source_for_non_portable_report_modes() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultDecode,
            &serde_json::json!({
                "decoded_count": 1,
                "audit_event": {"event_id": "evt-1"},
                "decoded_values": {"record-1": {"name": "Jane Doe"}}
            }),
        );

        assert_eq!(
            next_vault_response_report_save_path(
                DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH,
                None,
                Some("C:\\vaults\\Clinic Batch.mdid-portable.json"),
                &state,
            ),
            (
                "Clinic-Batch.mdid-portable-response-report.json".to_string(),
                Some("Clinic-Batch.mdid-portable-response-report.json".to_string())
            )
        );
    }

    #[test]
    fn app_save_workflow_output_writes_latest_csv_output() {
        let dir = std::env::temp_dir().join(format!(
            "mdid-desktop-workflow-output-save-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&dir).expect("tempdir");
        let path = dir.join("patient-jane-doe-mrn-12345-deidentified.csv");
        let mut app = DesktopApp::default();
        app.request_state.mode = DesktopWorkflowMode::CsvText;
        app.response_state.apply_success_json(
            DesktopWorkflowMode::CsvText,
            serde_json::json!({
                "csv": "patient_name\n<NAME-1>\n",
                "summary": {"encoded_fields": 1},
                "review_queue": []
            }),
        );

        app.save_workflow_output(&path)
            .expect("workflow output saved");

        assert_eq!(
            std::fs::read(&path).expect("saved output readable"),
            b"patient_name\n<NAME-1>\n"
        );
        std::fs::remove_dir_all(dir).expect("remove tempdir");
    }

    #[test]
    fn app_save_workflow_output_action_sets_phi_safe_success_status() {
        let dir = std::env::temp_dir().join(format!(
            "mdid-desktop-workflow-output-status-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&dir).expect("tempdir");
        let path = dir.join("patient-jane-doe-mrn-12345-deidentified.csv");
        let mut app = DesktopApp::default();
        app.request_state.mode = DesktopWorkflowMode::CsvText;
        app.response_state.apply_success_json(
            DesktopWorkflowMode::CsvText,
            serde_json::json!({
                "csv": "patient_name\n<NAME-1>\n",
                "summary": {"encoded_fields": 1},
                "review_queue": []
            }),
        );
        app.workflow_output_save_path = path.to_string_lossy().to_string();

        app.save_workflow_output_response();

        assert_eq!(
            app.workflow_output_save_status,
            "Rewritten workflow output saved."
        );
        assert!(!app
            .workflow_output_save_status
            .contains(path.to_string_lossy().as_ref()));
        assert!(!app.workflow_output_save_status.contains("jane-doe"));
        assert!(!app.workflow_output_save_status.contains("12345"));
        assert_eq!(
            std::fs::read(&path).expect("saved output readable"),
            b"patient_name\n<NAME-1>\n"
        );
        std::fs::remove_dir_all(dir).expect("remove tempdir");
    }

    #[test]
    fn app_save_workflow_output_action_sets_phi_safe_no_output_status() {
        let path = "/tmp/patient-jane-doe-mrn-12345-deidentified.csv";
        let mut app = DesktopApp {
            workflow_output_save_path: path.to_string(),
            ..DesktopApp::default()
        };

        app.save_workflow_output_response();

        assert_eq!(
            app.workflow_output_save_status,
            "workflow output save failed: no rewritten output is available"
        );
        assert!(!app.workflow_output_save_status.contains(path));
        assert!(!app.workflow_output_save_status.contains("jane-doe"));
        assert!(!app.workflow_output_save_status.contains("12345"));
    }

    #[test]
    fn app_save_portable_artifact_writes_artifact_json_without_sensitive_runtime_envelope() {
        let dir = std::env::temp_dir().join(format!(
            "mdid-desktop-portable-artifact-save-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&dir).expect("tempdir");
        let path = dir.join("desktop-export.mdid-portable.json");
        let mut app = DesktopApp::default();
        app.vault_response_state.apply_success(
            mdid_desktop::DesktopVaultResponseMode::VaultExport,
            &serde_json::json!({
                "artifact": {"version": 1, "ciphertext": "encrypted-payload"},
                "audit_event": {"detail": "exported Alice Example"},
                "vault_path": "/secret/Alice.vault"
            }),
        );
        app.portable_artifact_save_path = path.to_string_lossy().to_string();

        app.save_portable_artifact_response();

        let saved = std::fs::read_to_string(&path).expect("artifact saved");
        assert!(saved.contains("encrypted-payload"));
        assert!(!saved.contains("Alice Example"));
        assert!(!saved.contains("/secret"));
        assert_eq!(
            app.portable_artifact_save_status,
            "Portable artifact JSON saved; encrypted contents only."
        );
    }

    #[test]
    fn app_save_vault_response_report_writes_safe_audit_summary_only() {
        let dir = std::env::temp_dir().join(format!(
            "mdid-desktop-vault-response-report-ui-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&dir).expect("tempdir");
        let path = dir.join("patient-jane-doe-mrn-12345-vault-report.json");
        let mut app = DesktopApp::default();
        app.vault_response_state.apply_success(
            mdid_desktop::DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({
                "event_count": 1,
                "returned_event_count": 1,
                "events": [
                    {
                        "event_id": "evt-1",
                        "kind": "decode",
                        "actor": "clinician-a",
                        "record_id": "record-7",
                        "scope": ["patient_name"],
                        "occurred_at": "2026-04-30T01:00:00Z",
                        "detail": "decoded Alice Example with token <NAME-1>"
                    }
                ],
                "vault_path": "/secret/Alice.vault",
                "passphrase": "do-not-save"
            }),
        );
        app.vault_response_report_save_path = path.to_string_lossy().to_string();

        app.save_vault_response_report_response();

        let saved = std::fs::read_to_string(&path).expect("safe vault report saved");
        assert!(saved.contains("\"mode\": \"vault_audit\""));
        assert!(saved.contains("events returned: 1 / 1"));
        assert!(!saved.contains("\"events\""));
        assert!(!saved.contains("\"kind\""));
        assert!(!saved.contains("Alice Example"));
        assert!(!saved.contains("<NAME-1>"));
        assert!(!saved.contains("/secret"));
        assert!(!saved.contains("do-not-save"));
        assert_eq!(
            app.vault_response_report_save_status,
            "Safe vault/portable response report saved."
        );
        assert!(!app
            .vault_response_report_save_status
            .contains(path.to_string_lossy().as_ref()));
        assert!(!app.vault_response_report_save_status.contains("jane-doe"));
        assert!(!app.vault_response_report_save_status.contains("12345"));
        std::fs::remove_dir_all(dir).expect("remove tempdir");
    }

    #[test]
    fn app_save_vault_response_report_keeps_rendered_mode_when_request_controls_change() {
        let dir = std::env::temp_dir().join(format!(
            "mdid-desktop-vault-response-rendered-mode-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&dir).expect("tempdir");
        let path = dir.join("vault-response-report.json");
        let mut app = DesktopApp::default();
        app.vault_response_state.apply_success(
            DesktopVaultResponseMode::InspectArtifact,
            &serde_json::json!({
                "record_count": 2,
                "preview": [
                    {"record_id": "record-1"},
                    {"record_id": "record-2"}
                ]
            }),
        );
        app.portable_request_state.mode = DesktopPortableMode::ImportArtifact;
        app.vault_response_report_save_path = path.to_string_lossy().to_string();

        app.save_vault_response_report_response();

        let saved = std::fs::read_to_string(&path).expect("safe vault report saved");
        assert!(saved.contains("\"mode\": \"portable_artifact_inspect\""));
        assert!(!saved.contains("portable_artifact_import"));
        std::fs::remove_dir_all(dir).expect("remove tempdir");
    }

    #[test]
    fn app_save_vault_response_report_action_sets_phi_safe_no_response_status() {
        let path = "/tmp/patient-jane-doe-mrn-12345-vault-report.json";
        let mut app = DesktopApp {
            vault_response_report_save_path: path.to_string(),
            ..DesktopApp::default()
        };

        app.save_vault_response_report_response();

        assert_eq!(
            app.vault_response_report_save_status,
            "safe response report or portable artifact is unavailable"
        );
        assert!(!app.vault_response_report_save_status.contains(path));
        assert!(!app.vault_response_report_save_status.contains("jane-doe"));
        assert!(!app.vault_response_report_save_status.contains("12345"));
    }

    #[test]
    fn app_save_vault_response_report_action_sets_report_specific_phi_safe_write_error_status() {
        let path = std::env::temp_dir()
            .join(format!("mdid-missing-jane-doe-{}", uuid::Uuid::new_v4()))
            .join("patient-mrn-12345")
            .join("report.json");
        let mut app = DesktopApp::default();
        app.vault_response_state.apply_success(
            mdid_desktop::DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({
                "event_count": 1,
                "returned_event_count": 1,
                "events": [
                    {
                        "event_id": "evt-1",
                        "kind": "decode",
                        "actor": "clinician-a",
                        "record_id": "record-7",
                        "scope": ["patient_name"],
                        "occurred_at": "2026-04-30T01:00:00Z",
                        "detail": "decoded Jane Doe with MRN 12345"
                    }
                ]
            }),
        );
        app.vault_response_report_save_path = path.to_string_lossy().to_string();

        app.save_vault_response_report_response();

        assert_eq!(
            app.vault_response_report_save_status,
            "portable artifact JSON could not be written"
        );
        assert!(!app
            .vault_response_report_save_status
            .contains(path.to_string_lossy().as_ref()));
        assert!(!app.vault_response_report_save_status.contains("jane-doe"));
        assert!(!app.vault_response_report_save_status.contains("12345"));
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
    fn app_imports_dropped_portable_artifact_into_selected_import_mode() {
        let mut app = DesktopApp::default();
        app.portable_request_state.mode = DesktopPortableMode::ImportArtifact;

        app.import_file_bytes_for_current_state(
            "Clinic Bundle.mdid-portable.json".to_string(),
            br#"{\"records\":[{\"record_id\":\"patient-1\"}]}"#,
        );

        assert_eq!(
            app.portable_request_state.mode,
            DesktopPortableMode::ImportArtifact
        );
        assert_eq!(
            app.portable_request_state.artifact_json,
            r#"{\"records\":[{\"record_id\":\"patient-1\"}]}"#
        );
        assert_eq!(
            app.portable_response_report_source_name.as_deref(),
            Some("Clinic Bundle.mdid-portable.json")
        );
        assert!(app.response_state.error.is_none());
    }

    #[test]
    fn app_imports_dropped_portable_artifact_from_export_mode_as_inspect() {
        let artifact_json = r#"{\"artifact\":{\"ciphertext\":\"secret\"}}"#;
        let mut app = DesktopApp::default();
        app.portable_request_state.mode = DesktopPortableMode::VaultExport;

        app.import_file_bytes_for_current_state(
            "Clinic Bundle.mdid-portable.json".to_string(),
            artifact_json.as_bytes(),
        );

        assert_eq!(
            app.portable_request_state.mode,
            DesktopPortableMode::InspectArtifact
        );
        assert_eq!(app.portable_request_state.artifact_json, artifact_json);
        assert_eq!(
            app.portable_response_report_source_name.as_deref(),
            Some("Clinic Bundle.mdid-portable.json")
        );
        assert!(app.response_state.error.is_none());
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
