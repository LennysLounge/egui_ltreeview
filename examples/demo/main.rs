use std::{env, path::Path};

use egui::{Id, Label, Modal, ScrollArea, ThemePreference};
use egui_ltreeview::{NodeConfig, RowLayout, TreeView, TreeViewBuilder};
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
            cc.egui_ctx.set_zoom_factor(2.0);
            cc.egui_ctx
                .options_mut(|options| options.theme_preference = ThemePreference::Dark);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            //catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::MOCHA);
            Ok(Box::new(MyApp {
                tree: build_tree(Path::new("../egui_ltreeview")),
                add_dir_at: None,
                dir_name: String::from("New Dir"),
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
impl Node {
    fn name(&self) -> &str {
        match self {
            Node::Folder { name, .. } => name,
            Node::File { name, .. } => name,
        }
    }
    fn id(&self) -> &Uuid {
        match self {
            Node::Folder { id, .. } => id,
            Node::File { id, .. } => id,
        }
    }
}

struct MyApp {
    tree: Node,
    add_dir_at: Option<Uuid>,
    dir_name: String,
}

struct TreeNode<'a> {
    node: &'a Node,
    default_open: bool,
    add_dir_at: &'a mut bool,
}
impl<'a> NodeConfig<Uuid> for TreeNode<'a> {
    fn id(&self) -> &Uuid {
        self.node.id()
    }

    fn is_dir(&self) -> bool {
        match self.node {
            Node::Folder { .. } => true,
            Node::File { .. } => false,
        }
    }

    fn label(&mut self, ui: &mut egui::Ui) {
        ui.add(Label::new(self.node.name()).selectable(false));
    }

    fn default_open(&self) -> bool {
        self.default_open
    }

    fn has_custom_icon(&self) -> bool {
        match self.node {
            Node::Folder { .. } => false,
            Node::File { .. } => true,
        }
    }

    fn icon(&mut self, ui: &mut egui::Ui) {
        let name = match self.node {
            Node::Folder { name, .. } => name,
            Node::File { name, .. } => name,
        };
        let image_source =
            if name.ends_with(".gif") || name.ends_with(".svg") || name.ends_with(".png") {
                egui::include_image!("./image.svg")
            } else if name.ends_with(".rs") {
                egui::include_image!("./rust.svg")
            } else if name.ends_with(".md") {
                egui::include_image!("./markdown.svg")
            } else if name.ends_with(".toml") {
                egui::include_image!("./config.svg")
            } else if name.ends_with(".gitignore") {
                egui::include_image!("./git_ignore.svg")
            } else if name.ends_with("LICENSE") {
                egui::include_image!("./license.svg")
            } else if name.ends_with(".json") {
                egui::include_image!("./json.svg")
            } else {
                egui::include_image!("./default.svg")
            };
        egui::Image::new(image_source).paint_at(ui, ui.available_rect_before_wrap().expand(3.0));
    }
    fn has_custom_closer(&self) -> bool {
        match self.node {
            Node::Folder { .. } => true,
            Node::File { .. } => false,
        }
    }
    fn closer(&mut self, ui: &mut egui::Ui, closer_state: egui_ltreeview::CloserState) {
        let color = if closer_state.is_hovered {
            ui.visuals().widgets.hovered.fg_stroke.color
        } else {
            ui.visuals().widgets.noninteractive.fg_stroke.color
        };
        let image_source = if closer_state.is_open {
            egui::include_image!("./folder_open.png")
        } else {
            egui::include_image!("./folder.png")
        };
        egui::Image::new(image_source)
            .tint(color)
            .paint_at(ui, ui.available_rect_before_wrap());
    }

    fn has_context_menu(&self) -> bool {
        true
    }
    fn context_menu(&mut self, ui: &mut egui::Ui) {
        _ = ui.button("New file");
        if ui.button("New folder").clicked() {
            *self.add_dir_at = true;
        };
        ui.separator();
        _ = ui.button("Reveal in File Explorer");
        _ = ui.button("Open in integrated Terminal");
        ui.separator();
        _ = ui.button("Cut");
        _ = ui.button("Copy");
        ui.separator();
        _ = ui.button("Copy path");
        _ = ui.button("Copy relative path");
        ui.separator();
        _ = ui.button("Rename");
        _ = ui.button("Delete");
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(target) = self.add_dir_at {
            Modal::new(Id::new("modal")).show(ctx, |ui| {
                ui.label("Folder name:");
                ui.text_edit_singleline(&mut self.dir_name);
                egui::Sides::new().show(
                    ui,
                    |_ui| {},
                    |ui| {
                        if ui.button("Create").clicked() {
                            let node = Node::Folder {
                                id: Uuid::new_v4(),
                                name: self.dir_name.clone(),
                                content: Vec::new(),
                            };
                            insert_nodes(&mut self.tree, vec![node], &target, true);
                            self.add_dir_at = None;
                        }
                    },
                );
            });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut delete_nodes = Vec::new();
            let output = ScrollArea::both().show(ui, |ui| {
                TreeView::new(ui.make_persistent_id("Names tree view"))
                    .row_layout(RowLayout::CompactAlignedLabels)
                    .fallback_context_menu(|ui: &mut egui::Ui, nodes: &Vec<Uuid>| {
                        _ = ui.button("Reveal in File Explorer");
                        _ = ui.button("Open in integrated Terminal");
                        ui.separator();
                        _ = ui.button("Cut");
                        _ = ui.button("Copy");
                        ui.separator();
                        if ui.button("Delete").clicked() {
                            delete_nodes = nodes.clone();
                        };
                    })
                    .show(ui, |builder| {
                        show_node(builder, &self.tree, true, &mut self.add_dir_at);
                    })
            });
            for node_to_delete in delete_nodes {
                remove_node(&mut self.tree, &node_to_delete);
            }
            let (_tree_response, actions) = output.inner;
            for action in actions {
                match action {
                    egui_ltreeview::Action::Move(drag_and_drop) => {
                        let mut nodes = Vec::new();
                        for to_remove in drag_and_drop.source {
                            let removed = remove_node(&mut self.tree, &to_remove);
                            if let Some(removed) = removed {
                                nodes.push(removed);
                            }
                        }
                        insert_nodes(&mut self.tree, nodes, &drag_and_drop.target, false);
                    }
                    _ => (),
                }
            }
        });
    }
}

fn show_node(
    builder: &mut TreeViewBuilder<Uuid>,
    node: &Node,
    first: bool,
    add_dir_at: &mut Option<Uuid>,
) {
    let mut add_dir = false;
    builder.node(TreeNode {
        node,
        default_open: first,
        add_dir_at: &mut add_dir,
    });
    if add_dir {
        *add_dir_at = builder.parent_id().cloned();
    }
    match node {
        Node::Folder { content, .. } => {
            builder.close_dir_in(content.len());
            content
                .iter()
                .for_each(|n| show_node(builder, n, false, add_dir_at));
        }
        Node::File { .. } => (),
    }
}

fn remove_node(tree: &mut Node, id: &Uuid) -> Option<Node> {
    if let Node::Folder { content, .. } = tree {
        let pos = content.iter().position(|n| n.id() == id);
        if let Some(pos) = pos {
            return Some(content.remove(pos));
        }
        for node in content {
            let n = remove_node(node, id);
            if n.is_some() {
                return n;
            }
        }
    }
    None
}
fn insert_nodes(
    tree: &mut Node,
    mut to_insert: Vec<Node>,
    target: &Uuid,
    first: bool,
) -> Vec<Node> {
    if tree.id() == target {
        match tree {
            Node::Folder { content, .. } => {
                if first {
                    for x in to_insert {
                        content.insert(0, x);
                    }
                } else {
                    content.extend(to_insert);
                }

                return Vec::new();
            }
            _ => (),
        }
    } else {
        match tree {
            Node::Folder { content, .. } => {
                for node in content {
                    to_insert = insert_nodes(node, to_insert, target, first);
                    if to_insert.is_empty() {
                        return to_insert;
                    }
                }
            }
            _ => (),
        }
    }
    return to_insert;
}
