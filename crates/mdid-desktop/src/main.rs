fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "med-de-id desktop",
        options,
        Box::new(|_cc| Box::<DesktopApp>::default()),
    )
}

#[derive(Default)]
struct DesktopApp;

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("med-de-id desktop");
            ui.label("Sensitive workstation surface skeleton");
        });
    }
}
