#[path = "data.rs"]
mod data;
use data::*;
use egui::{vec2, DragValue, Id, Ui};
use egui_ltreeview::TreeViewBuilder;
use uuid::Uuid;

fn main() -> Result<(), eframe::Error> {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        //viewport: egui::ViewportBuilder::default().with_inner_size([300.0, 500.0]),
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
    settings_id: Uuid,
    selected_node: Option<Uuid>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            tree: make_tree(),
            settings_id: Uuid::new_v4(),
            selected_node: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left(Id::new("tree view"))
            .resizable(true)
            .show(ctx, |ui| {
                ui.allocate_space(vec2(ui.available_width(), 0.0));
                let response = TreeViewBuilder::new(
                    ui,
                    ui.make_persistent_id("Names tree view"),
                    |mut builder| {
                        builder.leaf(&self.settings_id, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(ui.spacing().indent);
                                ui.label("Settings");
                            });
                        });
                        show_node(&mut builder, &self.tree);
                    },
                );
                self.selected_node = response.selected_node;
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected_node == Some(self.settings_id) {
                show_settings(ui);
            } else {
                ui.label("Center");
            }
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

fn show_settings(ui: &mut Ui) {
    egui::Grid::new("settings grid").show(ui, |ui| {
        ui.strong("Egui:");
        ui.end_row();

        ui.label("Indent:");
        let mut indent = ui.ctx().style().spacing.indent;
        ui.add(DragValue::new(&mut indent).clamp_range(0.0..=f32::INFINITY));
        ui.ctx().style_mut(|style| {
            style.spacing.indent = indent;
        });
        ui.end_row();

        ui.label("Item spacing:");
        let mut spacing = ui.ctx().style().spacing.item_spacing;
        ui.add(DragValue::new(&mut spacing.x));
        ui.add(DragValue::new(&mut spacing.y));
        ui.ctx().style_mut(|style| {
            style.spacing.item_spacing = spacing;
        });
        ui.end_row();
    });
}
