use std::{
    time::{Duration, Instant},
    u64,
};

use egui::{Label, ThemePreference};
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
        let tree = build_tree(200_000, 11);
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
    let mut stack = vec![node];
    while let Some(elem) = stack.pop() {
        match elem {
            Node::Directory { id, children, name } => {
                //builder.node(NodeBuilder::dir(*id).label(name).default_open(true));
                builder.node(DefaultNode {
                    id,
                    name,
                    is_dir: true,
                });
                builder.close_dir_in(children.len());
                for child in children {
                    stack.push(child)
                }
                // let open = builder.node(NodeBuilder::dir(*id).label(name).default_open(false));
                // if open {
                //     builder.close_dir_in(children.len());
                //     for child in children {
                //         stack.push(child)
                //     }
                // } else {
                //     builder.close_dir();
                // }
            }
            Node::Leaf { id, name } => {
                //builder.node(NodeBuilder::leaf(*id).label(name));
                builder.node(DefaultNode {
                    id,
                    name,
                    is_dir: false,
                });
            }
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

fn build_tree(node_count: u32, max_depth: u32) -> Node {
    let (width, max_nodes) = get_tree_width(node_count, max_depth);
    println!(
        "max depth of {} and a width of {} gives {} total possible nodes",
        max_depth, width, max_nodes
    );
    let (node, _) = build_sub_tree(node_count, max_depth, width);
    let counts = count_nodes(&node);
    println!("{} total nodes produced", counts.0 + counts.1);
    println!("dirs: {}, leafs: {}", counts.0, counts.1);
    node
}
fn build_sub_tree(node_count: u32, max_depth: u32, max_width: u32) -> (Node, u32) {
    if max_depth == 0 {
        let id = Uuid::new_v4();
        return (
            Node::Leaf {
                id,
                name: format!("{node_count}"),
            },
            1,
        );
    }

    let mut child_nodes = Vec::new();
    let mut nodes_made = 1;
    for _ in 0..max_width {
        if node_count - nodes_made > 0 {
            let (node, new_nodes_made) =
                build_sub_tree(node_count - nodes_made, max_depth - 1, max_width);
            nodes_made += new_nodes_made;
            child_nodes.push(node);
        }
    }

    let id = Uuid::new_v4();
    (
        Node::Directory {
            id,
            children: child_nodes,
            name: format!("{node_count}"),
        },
        nodes_made,
    )
}

fn get_tree_width(node_count: u32, max_depth: u32) -> (u32, u32) {
    for width in 2..100 {
        let mut total_count = width;
        let mut prev_width = width;
        for d in 0..max_depth {
            prev_width = prev_width * width;
            total_count += prev_width;
            println!("total count {total_count} width: {width}, depth: {d}");
            if total_count > node_count {
                return (width, total_count);
            }
        }
    }
    panic!("dude what the hell")
}

fn count_nodes(node: &Node) -> (i32, i32) {
    match node {
        Node::Directory { children, .. } => {
            let mut dirs = 1;
            let mut leafs = 0;
            for child in children {
                let counts = count_nodes(child);
                dirs += counts.0;
                leafs += counts.1;
            }
            (dirs, leafs)
        }
        Node::Leaf { .. } => (0, 1),
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
