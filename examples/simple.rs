#[path = "data.rs"]
mod data;
use data::*;

use egui_ltreeview::TreeViewBuilder;
use uuid::Uuid;

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

struct MyApp {
    tree: Node,
}

impl Default for MyApp {
    fn default() -> Self {
        Self { tree: make_tree() }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            TreeViewBuilder::new(
                ui,
                ui.make_persistent_id("Names tree view"),
                |mut builder| {
                    show_node(&mut builder, &self.tree);
                },
            );
        });
    }
}

fn show_node(builder: &mut TreeViewBuilder<Uuid>, node: &Node) {
    match node {
        Node::Directory(dir) => show_dir(builder, dir),
        Node::File(file) => show_file(builder, file),
    }
}
fn show_dir(builder: &mut TreeViewBuilder<Uuid>, dir: &Directory) {
    builder.dir(&dir.id, |ui| {
        ui.label(&dir.name);
    });

    for node in dir.children.iter() {
        show_node(builder, node);
    }

    builder.close_dir();
}
fn show_file(builder: &mut TreeViewBuilder<Uuid>, file: &File) {
    builder.leaf(&file.id, |ui| {
        ui.label(&file.name);
    });
}
