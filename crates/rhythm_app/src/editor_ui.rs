pub fn draw_editor_shell(ui: &mut egui::Ui) {
    egui::Panel::top("transport_rhythm")
        .exact_size(54.0)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("Rhythm Effects");
                ui.separator();
                ui.label("Transport");
                ui.separator();
                ui.label("BPM —");
                ui.label("4/4");
                ui.label("Grid 1/4");
            });
        });

    egui::Panel::bottom("timeline")
        .resizable(true)
        .default_size(300.0)
        .show(ui, |ui| {
            ui.heading("Timeline");
            ui.separator();
            ui.label("Musical timeline placeholder");
            ui.take_available_space();
        });

    egui::Panel::left("objects")
        .resizable(true)
        .default_size(240.0)
        .show(ui, |ui| {
            ui.heading("Objects");
            ui.separator();
            ui.label("Composition objects will appear here.");
            ui.take_available_space();
        });

    egui::Panel::right("inspector")
        .resizable(true)
        .default_size(320.0)
        .show(ui, |ui| {
            ui.heading("Inspector");
            ui.separator();
            ui.label("Selected object properties will appear here.");
            ui.take_available_space();
        });

    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("Viewport");
        ui.separator();
        ui.centered_and_justified(|ui| {
            ui.label("1920 × 1080 composition");
        });
    });
}
