use egui::{ScrollArea, ThemePreference};
use egui_ltreeview::TreeView;

fn main() -> Result<(), eframe::Error> {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Egui_ltreeview tall tree with varying width nodes",
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("tree panel").show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                TreeView::new(ui.make_persistent_id("Names tree view")).show(ui, |builder| {
                    for val in 1..100 {
                        let width = 1 + val / 5;
                        let name = width.to_string().repeat(width);
                        builder.leaf(val, name);
                    }
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |_ui| {});
    }
}
