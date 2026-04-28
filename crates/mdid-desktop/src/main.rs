use mdid_desktop::{
    DesktopWorkflowMode, DesktopWorkflowRequestState, DesktopWorkflowResponseState,
};

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
    response_state: DesktopWorkflowResponseState,
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                "Not implemented in this desktop slice: runtime networking, file picker upload/download UX, vault browsing, decode, audit investigation, OCR, visual redaction, PDF rewrite/export, and controller workflows.",
            );
        });
    }
}
