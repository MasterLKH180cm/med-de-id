use mdid_desktop::{
    DesktopRuntimeSettings, DesktopRuntimeSubmissionSnapshot, DesktopRuntimeSubmitError,
    DesktopWorkflowMode, DesktopWorkflowRequestState, DesktopWorkflowResponseState,
};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

type RuntimeSubmissionResult = Result<serde_json::Value, DesktopRuntimeSubmitError>;

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
    runtime_settings: DesktopRuntimeSettings,
    response_state: DesktopWorkflowResponseState,
    runtime_submission_receiver: Option<Receiver<RuntimeSubmissionResult>>,
    runtime_submission_mode: Option<DesktopWorkflowMode>,
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
                let mode = self
                    .runtime_submission_mode
                    .take()
                    .unwrap_or(self.request_state.mode);
                self.response_state.apply_success_json(mode, envelope);
            }
            Ok(Err(error)) => {
                self.runtime_submission_mode = None;
                self.response_state.apply_error(format!("{error:?}"));
            }
            Err(TryRecvError::Empty) => {
                self.runtime_submission_receiver = Some(receiver);
            }
            Err(TryRecvError::Disconnected) => {
                self.runtime_submission_mode = None;
                self.response_state
                    .apply_error("runtime submission worker disconnected".to_string());
            }
        }
    }
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_runtime_submission();
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
                DesktopWorkflowMode::PdfBase64Review => {
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
                    Ok(request) => match self.runtime_settings.client() {
                        Ok(client) => {
                            let mode = self.request_state.mode;
                            let route = request.route;
                            let (sender, receiver) = std::sync::mpsc::channel();
                            self.runtime_submission_receiver = Some(receiver);
                            self.runtime_submission_mode = Some(mode);
                            self.response_state.banner =
                                format!("Submitting {route} to local runtime...");
                            self.response_state.error = None;
                            std::thread::spawn(move || {
                                let _ = sender.send(client.submit(&request));
                            });
                        }
                        Err(error) => self.response_state.apply_error(format!("{error:?}")),
                    },
                    Err(error) => self.response_state.apply_error(format!("{error:?}")),
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

            ui.label(
                "Not implemented in this desktop slice: file picker upload/download UX, vault browsing, decode, audit investigation, OCR, visual redaction, PDF rewrite/export, and full review workflows.",
            );
        });
        self.poll_runtime_submission();
        if self.runtime_submission_receiver.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}
