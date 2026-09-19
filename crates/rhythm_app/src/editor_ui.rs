pub fn draw_editor_shell(context: &egui::Context) {
    egui::TopBottomPanel::top("transport_rhythm")
        .exact_height(54.0)
        .show(context, |ui| {
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

    egui::TopBottomPanel::bottom("timeline")
        .resizable(true)
        .default_height(300.0)
        .show(context, |ui| {
            ui.heading("Timeline");
            ui.separator();
            ui.label("Musical timeline placeholder");
        });

    egui::SidePanel::left("objects")
        .resizable(true)
        .default_width(240.0)
        .show(context, |ui| {
            ui.heading("Objects");
            ui.separator();
            ui.label("Composition objects will appear here.");
        });

    egui::SidePanel::right("inspector")
        .resizable(true)
        .default_width(320.0)
        .show(context, |ui| {
            ui.heading("Inspector");
            ui.separator();
            ui.label("Selected object properties will appear here.");
        });

    egui::CentralPanel::default().show(context, |ui| {
        ui.heading("Viewport");
        ui.separator();
        ui.centered_and_justified(|ui| {
            ui.label("1920 × 1080 composition");
        });
    });
}
