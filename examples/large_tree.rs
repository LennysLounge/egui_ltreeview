use egui::ThemePreference;
use egui_ltreeview::{NodeBuilder, TreeView, TreeViewBuilder, TreeViewState};
use performance_measure::performance_measure::Measurer;
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
    measurer: Measurer,
}
impl MyApp {
    fn new() -> Self {
        MyApp {
            tree: build_tree(100_000, 11),
            state: TreeViewState::default(),
            measurer: Measurer::new(None),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                self.measurer.start_measure();
                TreeView::new(ui.make_persistent_id("Names tree view")).show_state(
                    ui,
                    &mut self.state,
                    |builder| {
                        build_node_once(&self.tree, builder);
                    },
                );
                //build_node_label(&self.tree, ui);

                self.measurer.stop_measure();
                // println!(
                //     "avg: {:?}\tlow: {:?}\thigh: {:?}",
                //     self.measurer.get_average(),
                //     self.measurer.get_min(),
                //     self.measurer.get_max()
                // );
            })
        });
        egui::TopBottomPanel::bottom("bottom panel").show(ctx, |ui| {
            let dt = ui.input(|i| i.stable_dt);
            ui.label(format!(
                "last frame: {:.0}ms, {}fps, tree view builder avgerage: {:?}ms, min: {:?}ms, max: {:?}ms",
                dt * 1000.0,
                (1.0 / dt).floor() as i32,
                self.measurer.get_average().as_millis(),
                self.measurer.get_min().as_millis(),
                self.measurer.get_max().as_millis()
            ));
        });
        if ctx.input(|i| i.viewport().close_requested()) {
            println!(
                "avg: {:?}\tlow: {:?}\thigh: {:?}",
                self.measurer.get_average(),
                self.measurer.get_min(),
                self.measurer.get_max()
            );
        }
    }
}

fn build_node_once(node: &Node, builder: &mut TreeViewBuilder<Uuid>) {
    enum Stack<'a> {
        Node(&'a Node),
        CloseDir,
    }
    let mut stack = vec![Stack::Node(node)];
    while !stack.is_empty() {
        let elem = stack.pop().unwrap();
        match elem {
            Stack::Node(node) => match node {
                Node::Directory { id, children, name } => {
                    build_dir(id, name, builder);
                    stack.push(Stack::CloseDir);
                    for child in children {
                        stack.push(Stack::Node(child))
                    }
                }
                Node::Leaf { id, name } => {
                    builder.leaf(*id, name);
                }
            },
            Stack::CloseDir => builder.close_dir(),
        }
    }
}

fn build_dir(id: &Uuid, name: &str, builder: &mut TreeViewBuilder<Uuid>) {
    builder.node(NodeBuilder::dir(*id).label(name).default_open(true));
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
