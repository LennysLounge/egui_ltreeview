use std::{
    time::{Duration, Instant},
    u64,
};

use egui::{Label, NumExt, ThemePreference};
use egui_ltreeview::{NodeConfig, TreeView, TreeViewBuilder, TreeViewState};
use uuid::Uuid;

fn main() -> Result<(), eframe::Error> {
    //tracing::subscriber::set_global_default(FmtSubscriber::new());

    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Egui_ltreeview simple example",
        options,
        Box::new(|cc| {
            cc.egui_ctx
                .options_mut(|options| options.theme_preference = ThemePreference::Dark);
            Ok(Box::<MyApp>::new(MyApp::new()))
        }),
    )
}

struct MyApp {
    tree: Node,
    state: TreeViewState<Uuid>,
    min: Duration,
    max: Duration,
    avg: Duration,
    times: Vec<Duration>,
    index: usize,
}
impl MyApp {
    fn new() -> Self {
        let tree = build_tree(100_000, 1, 2);
        let mut state = TreeViewState::default();
        init_state(&mut state, &tree);
        MyApp {
            tree,
            state,
            min: Duration::from_secs(u64::MAX),
            max: Duration::ZERO,
            avg: Duration::ZERO,
            times: Vec::with_capacity(1000),
            index: 0,
        }
    }
}

fn init_state(state: &mut TreeViewState<Uuid>, node: &Node) {
    let Node::Directory { id, children, .. } = node else {
        return;
    };
    state.set_openness(*id, true);
    for child in children {
        init_state(state, child);
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("bottom panel").show(ctx, |ui| {
            let dt = ui.input(|i| i.stable_dt);
            ui.label(format!(
                "last frame: {:.0}ms, {}fps, tree view builder avgerage: {:.3}ms, min: {:.3}ms, max: {:.3}ms",
                dt * 1000.0,
                (1.0 / dt).floor() as i32,
                self.avg.as_secs_f64() * 1000.0,
                self.min.as_secs_f64() * 1000.0,
                self.max.as_secs_f64() * 1000.0
            ));
        });
        if ctx.input(|i| i.viewport().close_requested()) {
            println!(
                "avg: {:.3}ms\tlow: {:.3}ms\thigh: {:.3}ms",
                self.avg.as_secs_f64() * 1000.0,
                self.min.as_secs_f64() * 1000.0,
                self.max.as_secs_f64() * 1000.0
            );
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                let start = Instant::now();
                TreeView::new(ui.make_persistent_id("Names tree view")).show_state(
                    ui,
                    &mut self.state,
                    |builder| {
                        build_node_once(&self.tree, builder);
                    },
                );
                let duration = start.elapsed();
                if self.times.len() < self.times.capacity() {
                    self.times.push(duration);
                } else {
                    self.times.insert(self.index, duration);
                    self.index = (self.index + 1) % self.times.len();
                }
                if duration > self.max {
                    self.max = duration
                }
                if duration < self.min {
                    self.min = duration;
                }
                self.avg = self.times.iter().sum::<Duration>() / self.times.len() as u32;
            })
        });
    }
}

fn build_node_once(node: &Node, builder: &mut TreeViewBuilder<Uuid>) {
    match node {
        Node::Directory { id, children, name } => {
            builder.node(DefaultNode {
                id,
                name,
                is_dir: true,
            });
            builder.close_dir_in(children.len());
            for child in children {
                build_node_once(child, builder);
            }
        }
        Node::Leaf { id, name } => {
            builder.node(DefaultNode {
                id,
                name,
                is_dir: false,
            });
        }
    }
}

#[derive(Debug)]
enum Node {
    Directory {
        id: Uuid,
        children: Vec<Node>,
        name: String,
    },
    Leaf {
        id: Uuid,
        name: String,
    },
}

fn build_tree(max_node_count: u32, files_per_dir: u32, dirs_per_dir: u32) -> Node {
    let mut root_nodes = Vec::new();
    let mut node_count = 0;
    add_dir(
        &mut root_nodes,
        max_node_count - node_count,
        &mut node_count,
        files_per_dir,
        dirs_per_dir,
        0,
    );
    let node = Node::Directory {
        id: Uuid::new_v4(),
        name: "Root".to_string(),
        children: root_nodes,
    };
    let counts = count_nodes(&node);
    println!("{} total nodes produced", counts.0 + counts.1);
    println!(
        "dirs: {}, leafs: {}, max depth: {}",
        counts.0, counts.1, counts.2
    );
    node
}
fn add_dir(
    parent: &mut Vec<Node>,
    max_node_count: u32,
    node_count: &mut u32,
    files_per_dir: u32,
    dirs_per_dir: u32,
    depth: i32,
) {
    //println!("depth: {depth} start, node_count: {}", *node_count);
    for _ in 0..files_per_dir {
        if *node_count >= max_node_count {
            return;
        }
        parent.push(Node::Leaf {
            id: Uuid::new_v4(),
            name: format!("File {}", *node_count),
        });
        *node_count += 1;
    }

    for dirs_to_be_added in (1..=dirs_per_dir).rev() {
        if *node_count >= max_node_count {
            return;
        }
        //println!("depth: {depth} dirs to be added: {dirs_to_be_added}");
        let nodes_remaining = max_node_count - *node_count;
        //println!("depth: {depth} nodes_to_be_added: {nodes_remaining}");
        let nodes_per_dir =
            (nodes_remaining.at_least(dirs_to_be_added) - dirs_to_be_added) / dirs_to_be_added;
        //println!("depth: {depth} nodes per dir: {nodes_per_dir}");

        let mut children = Vec::new();
        let name = format!("Dir {} nodes inside: {}", *node_count, nodes_per_dir);
        *node_count += 1;

        add_dir(
            &mut children,
            *node_count + nodes_per_dir,
            node_count,
            files_per_dir,
            dirs_per_dir,
            depth + 1,
        );

        parent.push(Node::Directory {
            id: Uuid::new_v4(),
            children,
            name,
        });
    }
}

fn count_nodes(node: &Node) -> (i32, i32, i32) {
    match node {
        Node::Directory { children, .. } => {
            let mut dirs = 1;
            let mut leafs = 0;
            let mut max_depth = 0;
            for child in children {
                let counts = count_nodes(child);
                dirs += counts.0;
                leafs += counts.1;
                max_depth = max_depth.max(counts.2);
            }
            (dirs, leafs, max_depth + 1)
        }
        Node::Leaf { .. } => (0, 1, 0),
    }
}

struct DefaultNode<'a> {
    id: &'a Uuid,
    name: &'a str,
    is_dir: bool,
}
impl<'a> NodeConfig<Uuid> for DefaultNode<'a> {
    fn id(&self) -> &Uuid {
        self.id
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }

    fn default_open(&self) -> bool {
        true
    }

    fn label(&mut self, ui: &mut egui::Ui) {
        ui.add(Label::new(self.name).selectable(false));
    }
}
