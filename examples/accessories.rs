#[path = "data.rs"]
mod data;

use egui::ThemePreference;
use egui_ltreeview::{NodeBuilder, TreeView};

fn main() -> Result<(), eframe::Error> {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([300.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Egui_ltreeview simple example",
        options,
        Box::new(|cc| {
            cc.egui_ctx
                .options_mut(|options| options.theme_preference = ThemePreference::Dark);
            Ok(Box::<MyApp>::default())
        }),
    )
}

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                TreeView::new(ui.make_persistent_id("Names tree view"))
                    .max_width(ui.available_width())
                    .show(ui, |builder| {
                        builder.dir(0, "Root");
                        builder.node(
                            NodeBuilder::leaf(1)
                                .label("Im a label with an accessory")
                                .accessory(|ui| {
                                    _ = ui.button("click me");
                                }),
                        );
                        builder.close_dir();
                    });
            });
        });
    }
}
