use std::{env, path::Path};

use egui::{
    epaint::text::{FontInsert, InsertFontFamily},
    FontData, FontDefinitions, FontFamily, FontId, Label, ScrollArea, TextStyle, ThemePreference,
};
use egui_ltreeview::{NodeConfig, TreeView, TreeViewBuilder};
use uuid::Uuid;

fn main() -> Result<(), eframe::Error> {
    env::set_var("RUST_BACKTRACE", "1");
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Egui_ltreeview simple example",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_zoom_factor(1.0);
            cc.egui_ctx
                .options_mut(|options| options.theme_preference = ThemePreference::Dark);
            //catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::MOCHA);
            Ok(Box::new(MyApp {
                tree: build_tree(Path::new("../egui_ltreeview")),
            }))
        }),
    )
}

fn build_tree(path: &Path) -> Node {
    let name = path
        .file_name()
        .and_then(|file_name| file_name.to_os_string().into_string().ok())
        .unwrap_or_else(|| format!("{:?}", path));
    if path.is_dir() {
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for dir_entry in std::fs::read_dir(path).unwrap() {
            let dir_entry = dir_entry.unwrap();
            let node = build_tree(&dir_entry.path());
            match node {
                Node::Folder { .. } => dirs.push(node),
                Node::File { .. } => files.push(node),
            }
        }
        dirs.extend(files);
        Node::Folder {
            id: Uuid::new_v4(),
            name,
            content: dirs,
        }
    } else {
        Node::File {
            id: Uuid::new_v4(),
            name,
        }
    }
}

enum Node {
    Folder {
        id: Uuid,
        name: String,
        content: Vec<Node>,
    },
    File {
        id: Uuid,
        name: String,
    },
}

struct MyApp {
    tree: Node,
}

struct TreeNode<'a> {
    node: &'a Node,
    default_open: bool,
}
impl<'a> NodeConfig<Uuid> for TreeNode<'a> {
    fn id(&self) -> &Uuid {
        match self.node {
            Node::Folder { id, .. } => id,
            Node::File { id, .. } => id,
        }
    }

    fn is_dir(&self) -> bool {
        match self.node {
            Node::Folder { .. } => true,
            Node::File { .. } => false,
        }
    }

    fn label(&mut self, ui: &mut egui::Ui) {
        let name = match self.node {
            Node::Folder { name, .. } => name,
            Node::File { name, .. } => name,
        };
        ui.add(Label::new(name).selectable(false));
    }

    fn default_open(&self) -> bool {
        self.default_open
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both().show(ui, |ui| {
                TreeView::new(ui.make_persistent_id("Names tree view")).show(ui, |builder| {
                    add_node(builder, &self.tree, true);
                });
            });
        });
    }
}

fn add_node(builder: &mut TreeViewBuilder<Uuid>, node: &Node, first: bool) {
    builder.node(TreeNode {
        node,
        default_open: first,
    });
    match node {
        Node::Folder { content, .. } => {
            builder.close_dir_in(content.len());
            content.iter().for_each(|n| add_node(builder, n, false));
        }
        Node::File { .. } => (),
    }
}
