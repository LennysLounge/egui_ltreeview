#[path = "data.rs"]
mod data;

use egui_ltreeview::TreeView;

fn main() -> Result<(), eframe::Error> {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([300.0, 500.0]),
        default_theme: eframe::Theme::Dark,
        follow_system_theme: false,
        ..Default::default()
    };
    eframe::run_native(
        "Egui_ltreeview simple example",
        options,
        Box::new(|_| Box::<MyApp>::default()),
    )
}

struct MyApp {}

impl Default for MyApp {
    fn default() -> Self {
        Self {}
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            TreeView::new(ui.make_persistent_id("Names tree view")).show(ui, |mut builder| {
                builder.dir(0, |ui| {
                    ui.add(egui::Label::new("Root").selectable(false));
                });
                builder.dir(1, |ui| {
                    ui.add(egui::Label::new("Foo").selectable(false));
                });
                builder.leaf(2, |ui| {
                    ui.add(egui::Label::new("Ava").selectable(false));
                });
                builder.dir(3, |ui| {
                    ui.add(egui::Label::new("Bar").selectable(false));
                });
                builder.leaf(4, |ui| {
                    ui.add(egui::Label::new("Benjamin").selectable(false));
                });
                builder.leaf(5, |ui| {
                    ui.add(egui::Label::new("Charlotte").selectable(false));
                });
                builder.close_dir();
                builder.close_dir();
                builder.leaf(6, |ui| {
                    ui.add(egui::Label::new("Daniel").selectable(false));
                });
                builder.leaf(7, |ui| {
                    ui.add(egui::Label::new("Emma").selectable(false));
                });
                builder.dir(8, |ui| {
                    ui.add(egui::Label::new("Baz").selectable(false));
                });
                builder.leaf(9, |ui| {
                    ui.add(egui::Label::new("Finn").selectable(false));
                });
                builder.leaf(10, |ui| {
                    ui.add(egui::Label::new("Grayson").selectable(false));
                });
                builder.close_dir();
                builder.close_dir();
            });
        });
    }
}
