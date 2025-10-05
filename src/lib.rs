#![warn(missing_docs)]
//! # `egui_ltreeview` is a tree view widget for [egui](https://github.com/emilk/egui)
//!
//! This tree view widget implements all the common features of a tree view to get you
//! up and running as fast as possible.
//!
//! # Features:
//! * Directory and leaf nodes
//! * Node selection
//! * Select multiple nodes
//! * Keyboard navigation using arrow keys
//! * Frontend for Drag and Drop support
//! * Agnostic to the implementation of your data.
//! * Performant (100k nodes in ~3 ms)
//!
//! # Crate feature flags
//! * `persistence` Adds serde to [`NodeId`] and enabled the `persistence` feature of egui.
//! * `doc` Adds additional documentation.
//!
//! # Quick start
//! ```
//! # use egui_ltreeview::*;
//! # use egui::*;
//! # fn ui(ui: &mut egui::Ui){
//! TreeView::new(Id::new("tree view")).show(ui, |builder| {
//!     builder.dir(0, "Root");
//!     builder.leaf(1, "Ava");
//!     builder.leaf(2, "Benjamin");
//!     builder.leaf(3, "Charlotte");
//!     builder.close_dir();
//! });
//! # }
//! ```
//! Create a new [`TreeView`] with its unique id and show it for the current ui.
//! Use the [`builder`](TreeViewBuilder) in the callback to add directories and leaves
//! to the tree. The nodes of the tree must have a unique id which implements the [`NodeId`] trait.
//!
//! # Further information
#![cfg_attr(feature = "doc",
    doc = "Visit the [`doc`] module documentation for further information about these topics:",
    doc = "",
    doc = make_table_of_contents::make_table_of_contents!("src/doc/doc.md", "doc/index.html")
)]
#![cfg_attr(
    not(feature = "doc"),
    doc = "Enable the `doc` feature for further information"
)]

#[cfg(feature = "doc")]
pub mod doc;

mod builder;
mod node;
mod state;

use egui::{
    self, emath, layers::ShapeIdx, vec2, EventFilter, Id, Key, LayerId, Layout, Modifiers, NumExt,
    Order, PointerButton, Pos2, Rangef, Rect, Response, Sense, Shape, Ui, UiBuilder, Vec2,
};
use std::{collections::HashSet, hash::Hash};

pub use builder::*;
pub use node::*;
pub use state::*;

/// Identifies a node in the tree.
///
/// This is just a trait alias for the collection of necessary traits that a node id
/// must implement.
#[cfg(not(feature = "persistence"))]
pub trait NodeId: Clone + PartialEq + Eq + Hash {}
#[cfg(not(feature = "persistence"))]
impl<T> NodeId for T where T: Clone + PartialEq + Eq + Hash {}

#[cfg(feature = "persistence")]
/// A node in the tree is identified by an id that must implement this trait.
///
/// This is just a trait alias for the collection of necessary traits that a node id
/// must implement.
pub trait NodeId:
    Clone + PartialEq + Eq + Hash + serde::de::DeserializeOwned + serde::Serialize
{
}
#[cfg(feature = "persistence")]
impl<T> NodeId for T where
    T: Clone + PartialEq + Eq + Hash + serde::de::DeserializeOwned + serde::Serialize
{
}

/// A tree view widget.
pub struct TreeView<'context_menu, NodeIdType> {
    id: Id,
    settings: TreeViewSettings,
    #[allow(clippy::type_complexity)]
    fallback_context_menu: Option<Box<dyn FnOnce(&mut Ui, &Vec<NodeIdType>) + 'context_menu>>,
}

impl<'context_menu, NodeIdType: NodeId> TreeView<'context_menu, NodeIdType> {
    /// Create a tree view from an unique id.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            settings: TreeViewSettings::default(),
            fallback_context_menu: None,
        }
    }

    /// Construct the tree view using the [`TreeViewBuilder`] by adding [nodes](`NodeBuilder`) to the tree.
    ///
    /// [`NodeId`] has to be thread safe for this to load the [`TreeViewState`] from egui data.
    /// If your [`NodeId`] is not threadsafe consider creating a [`TreeViewState`] directly and displaying
    /// the the tree with [`TreeView::show_state`]
    ///
    pub fn show(
        self,
        ui: &mut Ui,
        build_tree_view: impl FnOnce(&mut TreeViewBuilder<'_, NodeIdType>),
    ) -> (Response, Vec<Action<NodeIdType>>)
    where
        NodeIdType: NodeId + Send + Sync + 'static,
    {
        let id = self.id;
        let mut state = TreeViewState::load(ui, id).unwrap_or_default();
        let res = self.show_state(ui, &mut state, build_tree_view);
        state.store(ui, id);
        res
    }

    /// Start displaying the tree view with a [`TreeViewState`].
    ///
    /// Construct the tree view using the [`TreeViewBuilder`] by adding
    /// directories or leaves to the tree.
    pub fn show_state(
        self,
        ui: &mut Ui,
        state: &mut TreeViewState<NodeIdType>,
        build_tree_view: impl FnOnce(&mut TreeViewBuilder<'_, NodeIdType>),
    ) -> (Response, Vec<Action<NodeIdType>>)
    where
        NodeIdType: NodeId,
    {
        let TreeView {
            id,
            settings,
            mut fallback_context_menu,
        } = self;

        // Set the focus filter to get correct keyboard navigation while focused.
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                EventFilter {
                    tab: false,
                    escape: false,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                },
            )
        });

        let (ui_data, tree_view_rect) = draw_foreground(
            ui,
            id,
            &settings,
            state,
            build_tree_view,
            &mut fallback_context_menu,
        );

        if !settings.allow_multi_select {
            state.prune_selection_to_single_id();
        }
        // Remember the size of the tree for next frame.
        //state.size = response.rect.size();

        draw_background(ui, &ui_data);

        if ui.memory(|m| m.has_focus(id)) {
            // If the widget is focused but no node is selected we want to select any node
            // to allow navigating throught the tree.
            // In case we gain focus from a drag action we select the dragged node directly.
            if state.selected().is_empty() {
                let fallback_selection = state.get_dragged().and_then(|v| v.first());
                if let Some(fallback_selection) = fallback_selection {
                    state.set_one_selected(fallback_selection.clone());
                }
            }
        }
        if ui_data.interaction.clicked() || ui_data.interaction.drag_started() {
            ui.memory_mut(|m| m.request_focus(id));
        }

        let mut actions = Vec::new();
        // Create a drag or move action.
        if ui_data.interaction.dragged() {
            if let Some((drop_id, position)) = &ui_data.drop_target {
                actions.push(Action::Drag(DragAndDrop {
                    source: state.get_simplified_dragged().cloned().unwrap_or_default(),
                    target: drop_id.clone(),
                    position: position.clone(),
                    drop_marker_idx: ui_data.drop_marker_idx,
                }))
            } else if !ui_data.drop_on_self {
                if let Some(position) = ui.ctx().pointer_latest_pos() {
                    actions.push(Action::DragExternal(DragAndDropExternal {
                        position,
                        source: state.get_simplified_dragged().cloned().unwrap_or_default(),
                    }));
                }
            }
        }
        if ui_data.interaction.drag_stopped() {
            if let Some((drop_id, position)) = ui_data.drop_target {
                actions.push(Action::Move(DragAndDrop {
                    source: state.get_simplified_dragged().cloned().unwrap_or_default(),
                    target: drop_id,
                    position,
                    drop_marker_idx: ui_data.drop_marker_idx,
                }))
            } else if !ui_data.drop_on_self {
                if let Some(position) = ui.ctx().pointer_latest_pos() {
                    actions.push(Action::MoveExternal(DragAndDropExternal {
                        position,
                        source: state.get_simplified_dragged().cloned().unwrap_or_default(),
                    }));
                }
            }
        }

        if ui_data.selected {
            actions.push(Action::SetSelected(state.selected().clone()));
        }

        if let Some(nodes_to_activate) = ui_data.activate {
            actions.push(Action::Activate(Activate {
                selected: nodes_to_activate.clone(),
                modifiers: ui.ctx().input(|i| i.modifiers),
            }));
        }

        if ui_data.interaction.drag_stopped() {
            state.reset_dragged();
        }

        (ui_data.interaction.with_new_rect(tree_view_rect), actions)
    }
}
///
/// # Customizing the look and feel of the tree view.
///
/// To change the basic settings of the tree view you can use the [`TreeViewSettings`] to customize the tree view
/// or use the convenience methods on [`TreeView`] directly.
/// Check out [`TreeViewSettings`] for all settings possible on the tree view.
/// ```
/// # use egui_ltreeview::*;
/// # fn ui(ui: &mut egui::Ui, id: egui::Id){
/// TreeView::new(id)
///     .with_settings(TreeViewSettings{
///         override_indent: Some(15.0),
///         ..Default::default()
///     })
///     .min_height(200.0)
///     .show(ui, |builder| {
///         # builder.leaf(0, "");
///         // build your tree here
/// });
/// # }
/// ```
///
impl<'context_menu, NodeIdType: NodeId> TreeView<'context_menu, NodeIdType> {
    /// Set the settings for this tree view with the [`TreeViewSettings`] struct.
    ///
    /// This is maybe more convienient to you than setting each setting individually.
    pub fn with_settings(mut self, settings: TreeViewSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Override the indent value for this tree view.
    ///
    /// By default, this value is 'None' which means that the indent value from the
    /// current ui is used. If this value is set, this value will be used as the indent
    /// value without affecting the ui's indent value.
    pub fn override_indent(mut self, indent: Option<f32>) -> Self {
        self.settings.override_indent = indent;
        self
    }

    /// Override whether or not the background of the nodes should striped.
    ///
    /// By default, this value is 'None' which means that the striped setting from the
    /// current UI style is used. If this value is set, it will be used without
    /// affecting the ui's value.
    pub fn override_striped(mut self, striped: Option<bool>) -> Self {
        self.settings.override_striped = striped;
        self
    }

    /// Set the style of the indent hint to show the indentation level.
    pub fn indent_hint_style(mut self, style: IndentHintStyle) -> Self {
        self.settings.indent_hint_style = style;
        self
    }

    /// Set the row layout for this tree.
    pub fn row_layout(mut self, layout: RowLayout) -> Self {
        self.settings.row_layout = layout;
        self
    }

    /// Set if the tree view is allowed to select multiple nodes at once.
    pub fn allow_multi_selection(mut self, allow_multi_select: bool) -> Self {
        self.settings.allow_multi_select = allow_multi_select;
        self
    }

    /// Set if nodes in the tree are allowed to be dragged and dropped.
    pub fn allow_drag_and_drop(mut self, allow_drag_and_drop: bool) -> Self {
        self.settings.allow_drag_and_drop = allow_drag_and_drop;
        self
    }

    /// Set the default node height for this tree.
    pub fn default_node_height(mut self, default_node_height: Option<f32>) -> Self {
        self.settings.default_node_height = default_node_height;
        self
    }

    /// Add a fallback context menu to the tree.
    ///
    /// If the node did not configure a context menu, either through [`NodeBuilder`](`NodeBuilder::context_menu`) or [`NodeConfig`](`NodeConfig::has_context_menu`),
    /// or if multiple nodes were selected and right-clicked, then this fallback context menu will be opened.
    ///
    /// A context menu in egui gets its size the first time it becomes visible.
    /// Since all nodes in the tree view share the same context menu you must set
    /// the size of the context menu manually for each node if you want to have differently
    /// sized context menus.
    pub fn fallback_context_menu(
        mut self,
        context_menu: impl FnOnce(&mut Ui, &Vec<NodeIdType>) + 'context_menu,
    ) -> Self {
        self.fallback_context_menu = Some(Box::new(context_menu));
        self
    }

    /// Set the minimum width the tree can have.
    pub fn min_width(mut self, width: f32) -> Self {
        self.settings.min_width = width;
        self
    }

    /// Set the minimum height the tree can have.
    pub fn min_height(mut self, height: f32) -> Self {
        self.settings.min_height = height;
        self
    }
}

#[allow(clippy::type_complexity)]
fn draw_foreground<'context_menu, NodeIdType: NodeId>(
    ui: &mut Ui,
    id: Id,
    settings: &TreeViewSettings,
    state: &mut TreeViewState<NodeIdType>,
    build_tree_view: impl FnOnce(&mut TreeViewBuilder<'_, NodeIdType>),
    fall_back_context_menu: &mut Option<Box<dyn FnOnce(&mut Ui, &Vec<NodeIdType>) + 'context_menu>>,
) -> (UiData<NodeIdType>, Rect) {
    // Calculate the desired size of the tree view widget.
    let interaction_rect = Rect::from_min_size(
        ui.cursor().min,
        ui.available_size()
            .at_least(vec2(settings.min_width, settings.min_height))
            .at_least(vec2(state.min_width, state.last_height)),
    );

    let interaction = interact_no_expansion(ui, interaction_rect, id, Sense::click_and_drag());
    let mut output = Output::None;
    let mut input = get_input::<NodeIdType>(ui, &interaction, id, settings);
    let mut ui_data = UiData {
        interaction,
        context_menu_was_open: false,
        drag_layer: LayerId::new(Order::Tooltip, ui.make_persistent_id("ltreeviw drag layer")),
        has_focus: ui.memory(|m| m.has_focus(id)) || state.context_menu_was_open,
        drop_marker_idx: ui.painter().add(Shape::Noop),
        drop_target: None,
        drop_on_self: false,
        activate: None,
        selected: false,
        space_used: Rect::from_min_size(ui.cursor().min, Vec2::ZERO),
    };
    // Run the build tree view closure

    let mut builder_ui = ui.new_child(
        UiBuilder::new()
            .layout(Layout::top_down(egui::Align::Min))
            .max_rect(interaction_rect),
    );
    let mut tree_builder = TreeViewBuilder::new(
        &mut builder_ui,
        state,
        settings,
        &mut ui_data,
        &mut input,
        &mut output,
    );
    build_tree_view(&mut tree_builder);

    let tree_view_rect = ui_data.space_used.union(interaction_rect);
    ui.allocate_rect(tree_view_rect, Sense::hover());

    // Remember width of the tree view for next frame
    state.min_width = state.min_width.at_least(ui_data.space_used.width());
    state.last_height = ui_data.space_used.height();

    // Do context menu
    if !ui_data.context_menu_was_open {
        if let Some(fallback_context_menu) = fall_back_context_menu.take() {
            ui_data.interaction.context_menu(|ui| {
                fallback_context_menu(ui, state.selected());
            });
        }
    }
    // Read out results from inputs
    match input {
        Input::DragStarted {
            selected_node_dragged,
            simplified_dragged,
            ..
        } if selected_node_dragged => {
            state.set_dragged(DragState {
                dragged: state.selected().clone(),
                simplified: simplified_dragged,
            });
        }
        _ => (),
    };
    match output {
        Output::SetDragged(dragged) => {
            state.set_dragged(dragged);
        }
        Output::SetSecondaryClicked(id) => {
            state.secondary_selection = Some(id);
        }
        Output::ActivateSelection(selection) => {
            ui_data.activate = Some(selection);
        }
        Output::ActivateThis(id) => {
            ui_data.activate = Some(vec![id]);
        }
        Output::SelectOneNode(id, scroll_to_rect) => {
            ui_data.selected = true;
            state.set_one_selected(id.clone());
            state.set_cursor(None);
            if let Some(scroll_to_rect) = scroll_to_rect {
                ui.scroll_to_rect(scroll_to_rect, None);
            }
        }
        Output::ToggleSelection(id, scroll_to_rect) => {
            ui_data.selected = true;
            state.toggle_selected(&id);
            state.set_pivot(Some(id));
            if let Some(scroll_to_rect) = scroll_to_rect {
                ui.scroll_to_rect(scroll_to_rect, None);
            }
        }
        Output::ShiftSelect(ids) => {
            ui_data.selected = true;
            state.set_selected_dont_change_pivot(ids);
        }
        Output::Select {
            selection,
            pivot,
            cursor,
            scroll_to_rect,
        } => {
            ui_data.selected = true;
            state.set_selected(selection);
            state.set_pivot(Some(pivot));
            state.set_cursor(Some(cursor));
            ui.scroll_to_rect(scroll_to_rect, None);
        }
        Output::SetCursor(id, scroll_to_rect) => {
            state.set_cursor(Some(id));
            ui.scroll_to_rect(scroll_to_rect, None);
        }
        Output::None => (),
    }

    state.context_menu_was_open = ui_data.interaction.context_menu_opened();

    (ui_data, tree_view_rect)
}

fn draw_background<NodeIdType: NodeId>(ui: &mut Ui, ui_data: &UiData<NodeIdType>) {
    if ui_data.interaction.dragged() {
        let (start, current) = ui.input(|i| (i.pointer.press_origin(), i.pointer.hover_pos()));
        if let (Some(start), Some(current)) = (start, current) {
            let delta = current.to_vec2() - start.to_vec2();
            let transform = emath::TSTransform::from_translation(delta);
            ui.ctx()
                .transform_layer_shapes(ui_data.drag_layer, transform);
        }
    }
}

/// A position inside a directory node.
///
/// When a source node is dragged this enum describes the position
/// where the node should be dropped inside a directory node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirPosition<NodeIdType> {
    /// The source node should be inserted in the first position of the directory
    First,
    /// The source node should be inserted in the last position of the directory.
    Last,
    /// The source node should be inserted after the node with this node id.
    After(NodeIdType),
    /// The source node should be inserted before the node with this node id.
    Before(NodeIdType),
}

/// The global settings the tree view will use.
#[derive(Clone, Debug)]
pub struct TreeViewSettings {
    /// Override the indent value for the tree view.
    ///
    /// By default, this value is 'None' which means that the indent value from the
    /// current UI is used. If this value is set, this value will be used as the indent
    /// value without affecting the ui's indent value.
    pub override_indent: Option<f32>,
    /// Override whether or not the background of the nodes should striped.
    ///
    /// By default, this value is 'None' which means that the striped setting from the
    /// current UI style is used. If this value is set, it will be used without
    /// affecting the ui's value.
    pub override_striped: Option<bool>,
    /// The style of the indent hint to show the indentation level.
    pub indent_hint_style: IndentHintStyle,
    /// The row layout for this tree.
    pub row_layout: RowLayout,
    /// The minimum width the tree can have.
    pub min_width: f32,
    /// The minimum height the tree can have.
    pub min_height: f32,
    /// If the tree view is allowed to select multiple nodes at once.
    /// Default is true.
    pub allow_multi_select: bool,
    /// If the nodes in the tree view are allowed to be dragged and dropped.
    /// Default is true.
    pub allow_drag_and_drop: bool,
    /// The default height of a node.
    /// If none is set the default height will be `interact_size.y` from `egui::style::Spacing`.
    pub default_node_height: Option<f32>,
}

impl Default for TreeViewSettings {
    fn default() -> Self {
        Self {
            override_indent: None,
            override_striped: None,
            indent_hint_style: Default::default(),
            row_layout: Default::default(),
            min_width: 0.0,
            min_height: 0.0,
            allow_multi_select: true,
            allow_drag_and_drop: true,
            default_node_height: None,
        }
    }
}

/// Style of the vertical line to show the indentation level.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndentHintStyle {
    /// No indent hint is shown.
    None,
    /// A single vertical line is show for the full height of the directory.
    /// ```text
    /// v Foo
    /// │  Alice
    /// │  v Bar
    /// │  │ Bob
    /// │  │  v Baz
    /// │  │  │ Clair
    /// │  │  │ Denis
    /// │  Emil
    /// ```
    Line,
    /// A vertical line is show with horizontal hooks to the child nodes of the directory.
    /// ```text
    /// v Foo
    /// ├─ Alice
    /// ├─ v Bar
    /// │  ├─ Bob
    /// │  └─ v Baz
    /// │     ├─ Clair
    /// │     └─ Denis
    /// └─ Emil
    /// ```
    #[default]
    Hook,
}

/// How rows in the tree are laid out.
///
/// Each row in the tree is made up of three elements. A closer,
/// an icon and a label. The layout of these elements is controlled
/// by this value.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RowLayout {
    /// No icons are displayed.
    /// Directories only show the closer and the label.
    /// Leaves only show the label and allocate no additional space for a closer.
    /// Labels between leaves and directories do not align.
    Compact,
    /// The labels of leaves and directories are aligned.
    /// Icons are displayed for leaves only.
    CompactAlignedLabels,
    /// The icons of leaves and directories are aligned.
    /// If a leaf or directory does not show an icon, the label will fill the
    /// space. Labels between leaves and directories can be misaligned.
    #[default]
    AlignedIcons,
    /// The labels of leaves and directories are aligned.
    /// If a leaf or directory does not show an icon, the label will not fill
    /// the space.
    AlignedIconsAndLabels,
}

/// An action the tree view would like to take as a result
/// of some user input like drag and drop.
#[derive(Clone, Debug)]
pub enum Action<NodeIdType> {
    /// Set the selected node to be this.
    SetSelected(Vec<NodeIdType>),
    /// Move set of nodes from one place to another.
    Move(DragAndDrop<NodeIdType>),
    /// An in-process drag and drop action where the node
    /// is currently dragged but not yet dropped.
    Drag(DragAndDrop<NodeIdType>),
    /// Activate a set of nodes.
    ///
    /// When pressing enter or double clicking on a selection, the tree
    /// view will create this action.
    /// Can be used to open a file for example.
    Activate(Activate<NodeIdType>),
    /// Indicates that nodes are being dragged outside the TreeView
    /// (but not yet dropped).
    DragExternal(DragAndDropExternal<NodeIdType>),
    /// Triggered when dragged nodes are released outside the TreeView.
    /// Indicates that the nodes should be moved to an
    /// external target (e.g., another panel).
    MoveExternal(DragAndDropExternal<NodeIdType>),
}

/// Represents a drag-and-drop interaction where nodes are dragged outside the TreeView.
/// Used to handle external drops (e.g., onto another UI component or the workspace).
#[derive(Clone, Debug)]
pub struct DragAndDropExternal<NodeIdType> {
    /// The nodes that are being dragged
    pub source: Vec<NodeIdType>,
    /// The position where the dragged nodes are dropped outside of the TreeView.
    pub position: egui::Pos2,
}

/// Information about drag and drop action that is currently
/// happening on the tree.
#[derive(Clone, Debug)]
pub struct DragAndDrop<NodeIdType> {
    /// The nodes that are being dragged
    pub source: Vec<NodeIdType>,
    /// The node where the dragged nodes are dropped.
    pub target: NodeIdType,
    /// The position where the dragged nodes are dropped inside the target node.
    pub position: DirPosition<NodeIdType>,
    /// The shape index of the drop marker.
    drop_marker_idx: ShapeIdx,
}
impl<NodeIdType> DragAndDrop<NodeIdType> {
    /// Remove the drop marker from the tree view.
    ///
    /// Use this to remove the drop marker if a proposed drag and drop action
    /// is disallowed.
    pub fn remove_drop_marker(&self, ui: &mut Ui) {
        ui.painter().set(self.drop_marker_idx, Shape::Noop);
    }
}

/// Information about the `Activate` action in the tree.
#[derive(Clone, Debug)]
pub struct Activate<NodeIdType> {
    /// The nodes that are being activated.
    pub selected: Vec<NodeIdType>,
    /// The modifiers that were active when this action was generated.
    pub modifiers: Modifiers,
}

/// Interact with the ui without egui adding any extra space.
fn interact_no_expansion(ui: &mut Ui, rect: Rect, id: Id, sense: Sense) -> Response {
    let spacing_before = ui.spacing().clone();
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let res = ui.interact(rect, id, sense);
    *ui.spacing_mut() = spacing_before;
    res
}

enum DropQuarter {
    Top,
    MiddleTop,
    MiddleBottom,
    Bottom,
}

impl DropQuarter {
    fn new(range: Rangef, cursor_pos: f32) -> Option<DropQuarter> {
        pub const DROP_LINE_HOVER_HEIGHT: f32 = 5.0;

        let h0 = range.min;
        let h1 = range.min + DROP_LINE_HOVER_HEIGHT;
        let h2 = range.center();
        let h3 = range.max - DROP_LINE_HOVER_HEIGHT;
        let h4 = range.max;

        match cursor_pos {
            y if y >= h0 && y < h1 => Some(Self::Top),
            y if y >= h1 && y < h2 => Some(Self::MiddleTop),
            y if y >= h2 && y < h3 => Some(Self::MiddleBottom),
            y if y >= h3 && y < h4 => Some(Self::Bottom),
            _ => None,
        }
    }
}

struct UiData<NodeIdType> {
    context_menu_was_open: bool,
    interaction: Response,
    drag_layer: LayerId,
    has_focus: bool,
    drop_marker_idx: ShapeIdx,
    drop_target: Option<(NodeIdType, DirPosition<NodeIdType>)>,
    drop_on_self: bool,
    activate: Option<Vec<NodeIdType>>,
    selected: bool,
    space_used: Rect,
}

/// When you ast a rectangle if it contains a point it does so inclusive the upper bound.
/// like this: min <= p <= max
/// Visually the rectangle is displayed exclusive the upper bound.
/// like this: min <= p < max
///
/// This means that two rectangles can not overlap visually but overlap when aksing if a point
/// is contained in them.
/// This check if a point is contained exclusive the upper bound.
fn rect_contains_visually(rect: &Rect, pos: &Pos2) -> bool {
    rect.min.x <= pos.x && pos.x < rect.max.x && rect.min.y <= pos.y && pos.y < rect.max.y
}

enum Input<NodeIdType> {
    DragStarted {
        pos: Pos2,
        selected_node_dragged: bool,
        visited_selected_nodes: HashSet<NodeIdType>,
        simplified_dragged: Vec<NodeIdType>,
    },
    Dragged(Pos2),
    SecondaryClick(Pos2),
    Click {
        pos: Pos2,
        double: bool,
        modifiers: Modifiers,
        activatable_nodes: Vec<NodeIdType>,
        shift_click_nodes: Option<Vec<NodeIdType>>,
    },
    KeyLeft,
    KeyRight {
        select_next: bool,
    },
    KeyUp {
        previous_node: Option<(NodeIdType, Rect)>,
    },
    KeyUpAndCommand {
        previous_node: Option<(NodeIdType, Rect)>,
    },
    KeyUpAndShift {
        previous_node: Option<(NodeIdType, Rect)>,
        nodes_to_select: Option<Vec<NodeIdType>>,
        next_cursor: Option<(NodeIdType, Rect)>,
    },
    KeyDown(bool),
    KeyDownAndCommand {
        is_next: bool,
    },
    KeyDownAndShift {
        nodes_to_select: Option<Vec<NodeIdType>>,
        next_cursor: Option<(NodeIdType, Rect)>,
        is_next: bool,
    },
    KeySpace,
    KeyEnter {
        activatable_nodes: Vec<NodeIdType>,
    },
    None,
}
enum Output<NodeIdType> {
    SetDragged(DragState<NodeIdType>),
    SetSecondaryClicked(NodeIdType),
    ActivateSelection(Vec<NodeIdType>),
    ActivateThis(NodeIdType),
    SelectOneNode(NodeIdType, Option<Rect>),
    ShiftSelect(Vec<NodeIdType>),
    ToggleSelection(NodeIdType, Option<Rect>),
    Select {
        selection: Vec<NodeIdType>,
        pivot: NodeIdType,
        cursor: NodeIdType,
        scroll_to_rect: Rect,
    },
    SetCursor(NodeIdType, Rect),
    None,
}

fn get_input<NodeIdType>(
    ui: &Ui,
    interaction: &Response,
    id: Id,
    settings: &TreeViewSettings,
) -> Input<NodeIdType> {
    let press_origin = ui.input(|i| i.pointer.press_origin());
    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
    let modifiers = ui.input(|i| i.modifiers);

    if interaction.context_menu_opened() {
        if interaction.secondary_clicked() {
            return Input::SecondaryClick(
                pointer_pos.expect("If the tree view was clicked it must have a pointer position"),
            );
        }
        return Input::None;
    }

    if interaction.drag_started_by(PointerButton::Primary) && settings.allow_drag_and_drop {
        return Input::DragStarted {
            pos: press_origin
                .expect("If a drag has started it must have a position where the press started"),
            selected_node_dragged: false,
            visited_selected_nodes: HashSet::new(),
            simplified_dragged: Vec::new(),
        };
    }
    if (interaction.dragged_by(PointerButton::Primary)
        || interaction.drag_stopped_by(PointerButton::Primary))
        && settings.allow_drag_and_drop
    {
        return Input::Dragged(
            pointer_pos.expect("If the tree view is dragged it must have a pointer position"),
        );
    }
    if interaction.secondary_clicked() {
        return Input::SecondaryClick(
            pointer_pos.expect("If the tree view was clicked it must have a pointer position"),
        );
    }
    if interaction.clicked_by(PointerButton::Primary)
        || interaction.drag_started_by(PointerButton::Primary) && !settings.allow_drag_and_drop
    {
        return Input::Click {
            pos: pointer_pos.expect("If the tree view was clicked it must have a pointer position"),
            double: interaction.double_clicked(),
            modifiers,
            activatable_nodes: Vec::new(),
            shift_click_nodes: None,
        };
    }
    if !ui.memory(|m| m.has_focus(id)) {
        return Input::None;
    }
    if ui.input(|i| i.key_pressed(Key::ArrowLeft)) {
        return Input::KeyLeft;
    }
    if ui.input(|i| i.key_pressed(Key::ArrowRight)) {
        return Input::KeyRight { select_next: false };
    }
    if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
        if modifiers.shift_only() {
            return Input::KeyUpAndShift {
                previous_node: None,
                nodes_to_select: None,
                next_cursor: None,
            };
        }
        if modifiers.command_only() {
            return Input::KeyUpAndCommand {
                previous_node: None,
            };
        }
        return Input::KeyUp {
            previous_node: None,
        };
    }
    if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
        if modifiers.shift_only() {
            return Input::KeyDownAndShift {
                nodes_to_select: None,
                next_cursor: None,
                is_next: false,
            };
        }
        if modifiers.command_only() {
            return Input::KeyDownAndCommand { is_next: false };
        }
        return Input::KeyDown(false);
    }
    if ui.input(|i| i.key_pressed(Key::Space)) {
        return Input::KeySpace;
    }
    if ui.input(|i| i.key_pressed(Key::Enter)) {
        return Input::KeyEnter {
            activatable_nodes: Vec::new(),
        };
    }
    Input::None
}
