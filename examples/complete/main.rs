#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use data::{make_tree, TreeNode, Visitable};
use eframe::egui;
use egui::Ui;
use egui_ltreeview::TreeViewBuilder;
use visitor::{
    DropAllowedVisitor, InsertNodeVisitor, RemoveNodeVisitor, SearchVisitor, TreeViewVisitor,
};

mod data;
mod visitor;

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
    tree: TreeNode,
}

impl Default for MyApp {
    fn default() -> Self {
        Self { tree: make_tree() }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            show_tree(ui, &mut self.tree);
        });
    }
}

fn show_tree(ui: &mut Ui, tree: &mut TreeNode) {
    let tree_res = TreeViewBuilder::new(ui, ui.make_persistent_id("tree view"), |root| {
        tree.walk(&mut TreeViewVisitor { builder: root });
    });

    if let Some(selected_id) = tree_res.selected_node {
        SearchVisitor::new(selected_id, |selected| {
            ui.label(format!("selected: {}", selected.name()));
        })
        .search_in(tree);
    }

    if let Some(drop_action) = &tree_res.drag_drop_action {
        // Test if drop is valid
        let drop_allowed = {
            SearchVisitor::new(drop_action.drag_id, |dragged| {
                SearchVisitor::new(drop_action.drop_id, |dropped| {
                    DropAllowedVisitor::new(dragged.as_any()).test(dropped)
                })
                .search_in(tree)
            })
            .search_in(tree)
            .flatten()
            .unwrap_or(false)
        };

        if drop_allowed {
            if tree_res.dropped {
                // remove dragged node
                let removed_node = RemoveNodeVisitor::new(drop_action.drag_id).remove_from(tree);

                // insert node
                if let Some(dragged_node) = removed_node {
                    tree.walk_mut(&mut InsertNodeVisitor {
                        target_id: drop_action.drop_id,
                        position: drop_action.position,
                        node: Some(dragged_node),
                    });
                }
            }
        } else {
            // Render the dissallowed drop
            tree_res.remove_drop_marker(ui);
        }
    }
}
