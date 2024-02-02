#[path = "data.rs"]
mod data;

use egui_ltreeview::{builder::NodeBuilder, TreeView};

fn main() -> Result<(), eframe::Error> {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([300.0, 500.0]),
        default_theme: eframe::Theme::Dark,
        follow_system_theme: false,
        ..Default::default()
    };
    eframe::run_native(
        "Egui_ltreeview example",
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
                builder.dir(0, |ui| _ = ui.label("root"));

                // Sometimes you want to a section of the tree to behave like a dir
                // without incrasing the depth of the tree. In that case you can flatten
                // the dir. This will not render the dir but still register it in the tree.
                // instead of this: ```builder.dir(1, |ui| _ = ui.label("first"));```
                // do this:
                //builder.flat_dir(1);
                builder.node(NodeBuilder::dir(1).flatten(true), |ui| {
                    _ = ui.label("not visible")
                });
                builder.leaf(2, |ui| _ = ui.label("A"));
                builder.leaf(3, |ui| _ = ui.label("B"));
                builder.leaf(4, |ui| _ = ui.label("C"));
                builder.close_dir();

                builder.dir(5, |ui| _ = ui.label("second"));
                builder.leaf(6, |ui| _ = ui.label("D"));
                builder.leaf(7, |ui| _ = ui.label("E"));
                builder.leaf(8, |ui| _ = ui.label("F"));
                builder.close_dir();

                builder.close_dir();
            });
        });
    }
}
