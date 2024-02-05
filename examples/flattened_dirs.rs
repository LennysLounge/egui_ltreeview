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
            TreeView::new(ui.make_persistent_id("Names tree view"))
            .show(ui, |mut builder| {
                builder.dir(0, |ui| _ = ui.label("root"));

                // Sometimes you want to a section of the tree to behave like a dir
                // without incrasing the depth of the tree. In that case you can flatten
                // the dir. This will not render the dir but still register it in the tree.
                builder.node(NodeBuilder::dir(1).flatten(true), |ui| _ = ui.label("Foo"));
                builder.leaf(2, |ui| _ = ui.label("Ava"));
                builder.node(NodeBuilder::dir(3).flatten(true), |ui| _ = ui.label("Bar"));
                builder.leaf(4, |ui| _ = ui.label("Benjamin"));
                builder.leaf(5, |ui| _ = ui.label("Charlotte"));
                builder.close_dir();
                builder.close_dir();
                builder.leaf(6, |ui| _ = ui.label("Daniel"));
                builder.leaf(7, |ui| _ = ui.label("Emma"));
                builder.node(NodeBuilder::dir(8).flatten(true), |ui| _ = ui.label("Baz"));
                builder.leaf(9, |ui| _ = ui.label("Finn"));
                builder.leaf(10, |ui| _ = ui.label("Grayson"));
                builder.close_dir();

                builder.close_dir();
            });
            ui.label("hello");
        });
    }
}
