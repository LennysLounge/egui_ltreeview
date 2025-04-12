//! Demonstrates how to 'activate' nodes by pressing enter on them
//! or double clicking them. By default leaf nodes are activatable and
//! directory nodes are not.
//! You can configure if a node should be activatable by using the
//! node builder.
//! Works with multiple-selection too.
//!
#[path = "data.rs"]
mod data;

use egui::{Id, ThemePreference};
use egui_ltreeview::{Action, Activate, TreeView, TreeViewState};

fn main() -> Result<(), eframe::Error> {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Egui_ltreeview 'activatable' example",
        options,
        Box::new(|cc| {
            cc.egui_ctx
                .options_mut(|options| options.theme_preference = ThemePreference::Dark);
            Ok(Box::<MyApp>::default())
        }),
    )
}

struct MyApp {
    tree: TreeViewState<i32>,
    activated_history: Vec<Vec<i32>>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            tree: TreeViewState::default(),
            activated_history: Default::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left(Id::new("left")).show(ctx, |ui| {
            let (_response, actions) = TreeView::new(ui.make_persistent_id("Names tree view"))
                .show_state(ui, &mut self.tree, |builder| {
                    builder.dir(0, "Root");
                    builder.dir(1, "Foo");
                    builder.leaf(2, "Ava");
                    builder.dir(3, "Bar");
                    builder.leaf(4, "Benjamin");
                    builder.leaf(5, "Charlotte");
                    builder.close_dir();
                    builder.close_dir();
                    builder.leaf(6, "Daniel");
                    builder.leaf(7, "Emma");
                    builder.dir(8, "Baz");
                    builder.leaf(9, "Finn");
                    builder.leaf(10, "Grayson");
                    builder.close_dir();
                    builder.close_dir();
                });

            for action in actions {
                match action {
                    Action::Activate(Activate {
                        selected,
                        modifiers: _,
                    }) => {
                        self.activated_history.push(selected);
                    }
                    _ => {}
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Activate a selections by pressing enter or double-clicking.");
            ui.separator();
            ui.label("History");

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_max_width(ui.available_width());
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(200.0);

                    if self.activated_history.is_empty() {
                        ui.label("Empty");
                    } else {
                        for selection in &self.activated_history {
                            ui.label(format!("selection: {:?}", selection));
                        }
                    }
                });
            });

            if ui.button("Clear history").clicked() {
                self.activated_history.clear();
            }
        });
    }
}
