pub mod builder;
mod row;

use std::collections::HashMap;

use egui::{
    self, layers::ShapeIdx, util::id_type_map::SerializableAny, Event, EventFilter, Id, Key,
    Layout, Pos2, Rect, Response, Sense, Shape, Ui, Vec2,
};

pub use builder::TreeViewBuilder;

pub struct TreeView {
    id: Id,
    settings: TreeViewSettings,
}
impl TreeView {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            settings: TreeViewSettings::default(),
        }
    }

    /// Override the indent value from the current ui style with this value.
    ///
    /// If `None`, the value of the current ui style is used.
    /// Defaults to `None`.
    pub fn override_indent(mut self, indent: Option<f32>) -> Self {
        self.settings.override_indent = indent;
        self
    }

    /// Set the style of the vline to show the indentation level.
    pub fn vline_style(mut self, style: VLineStyle) -> Self {
        self.settings.vline_style = style;
        self
    }

    /// Set the row layout for this tree.
    pub fn row_layout(mut self, layout: RowLayout) -> Self {
        self.settings.row_layout = layout;
        self
    }

    /// Start displaying the tree view.
    ///
    /// Construct the tree view using the [`TreeViewBuilder`] by addind
    /// directories or leaves to the tree.
    pub fn show<NodeIdType>(
        self,
        ui: &mut Ui,
        mut build_tree_view: impl FnMut(TreeViewBuilder<'_, NodeIdType>),
    ) -> TreeViewResponse<NodeIdType>
    where
        NodeIdType: Clone
            + Copy
            + Send
            + Sync
            + std::hash::Hash
            + PartialEq
            + Eq
            + std::fmt::Debug
            + 'static,
    {
        let mut state = TreeViewState::load(ui, self.id);

        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                self.id,
                EventFilter {
                    tab: false,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: false,
                },
            )
        });

        // ui.painter().rect_stroke(
        //     state.peristant.rect,
        //     egui::Rounding::ZERO,
        //     egui::Stroke::new(
        //         1.0,
        //         if state.has_focus {
        //             egui::Color32::WHITE
        //         } else {
        //             egui::Color32::BLACK
        //         },
        //     ),
        // );

        let res = ui.allocate_ui_with_layout(
            ui.available_size_before_wrap(),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.add_space(ui.spacing().item_spacing.y * 0.5);
                build_tree_view(TreeViewBuilder::new(ui, &mut state, &self.settings));
                // Add negative space because the place will add the item spacing on top of this.
                ui.add_space(-ui.spacing().item_spacing.y * 0.5);
            },
        );

        ui.label(format!("dragged: {:?}", state.peristant.dragged));
        ui.label(format!("drop: {:?}", state.drop));

        let tree_view_interact = state.interact(&res.response.rect);
        if tree_view_interact.clicked || tree_view_interact.drag_started {
            ui.memory_mut(|m| m.request_focus(self.id));
        }

        if ui.memory(|m| m.has_focus(self.id)) {
            if state.peristant.selected == None {
                state.peristant.selected = state.node_order.first().map(|n| n.node_id);
            }
            ui.input(|i| {
                for event in i.events.iter() {
                    match event {
                        Event::Key { key, pressed, .. } if *pressed == true => {
                            handle_input(&mut state, key)
                        }
                        _ => (),
                    }
                }
            });
        }

        let drag_drop_action =
            state
                .peristant
                .dragged
                .zip(state.drop)
                .map(|(drag_id, (drop_id, position))| DragDropAction {
                    drag_id,
                    drop_id,
                    position,
                });
        let dropped = ui.ctx().input(|i| i.pointer.any_released()) && drag_drop_action.is_some();

        state.peristant.rect = res.response.rect;
        if state.response.drag_released() {
            state.peristant.dragged = None;
        }
        let res = TreeViewResponse {
            response: state.response.clone(),
            dropped,
            drag_drop_action,
            drop_marker_idx: state.drop_marker_idx.clone(),
            selected_node: state.peristant.selected.clone(),
            context_menu_node: state.peristant.context_menu.clone(),
        };

        state.store(ui, self.id);
        res
    }
}

fn handle_input<NodeIdType>(state: &mut TreeViewState<NodeIdType>, key: &Key)
where
    NodeIdType: Clone + Copy + PartialEq + Eq + std::hash::Hash + std::fmt::Debug,
{
    let Some(selected_index) = state
        .node_order
        .iter()
        .position(|n| Some(n.node_id) == state.peristant.selected)
    else {
        return;
    };
    let selected_node = state.node_order[selected_index].node_id;
    let selected_depth = state.node_order[selected_index].depth;
    let first_parent = state.node_order[0..selected_index]
        .iter()
        .rev()
        .find(|n| n.depth < selected_depth)
        .map(|n| n.node_id);

    match key {
        Key::ArrowUp => {
            if selected_index > 0 {
                state.peristant.selected = Some(state.node_order[selected_index - 1].node_id);
            }
        }
        Key::ArrowDown => {
            if selected_index < state.node_order.len() - 1 {
                state.peristant.selected = Some(state.node_order[selected_index + 1].node_id);
            }
        }
        Key::ArrowLeft => {
            if let Some(dir_open) = state.peristant.dir_states.get_mut(&selected_node) {
                if *dir_open {
                    *dir_open = false;
                } else {
                    if let Some(first_parent) = first_parent {
                        state.peristant.selected = Some(first_parent);
                    }
                }
            } else {
                if let Some(first_parent) = first_parent {
                    state.peristant.selected = Some(first_parent);
                }
            }
        }
        Key::ArrowRight => {
            if let Some(dir_open) = state.peristant.dir_states.get_mut(&selected_node) {
                if *dir_open {
                    if selected_index < state.node_order.len() - 1 {
                        state.peristant.selected =
                            Some(state.node_order[selected_index + 1].node_id);
                    }
                } else {
                    *dir_open = true;
                }
            }
        }
        _ => (),
    }
}

#[derive(Clone)]
struct TreeViewPersistantState<NodeIdType> {
    /// Id of the node that was selected.
    selected: Option<NodeIdType>,
    /// Id of the node that was dragged.
    dragged: Option<NodeIdType>,
    /// The rectangle the tree view occupied.
    rect: Rect,
    /// Position of the cursor when the drag started.
    _drag_start_pos: Option<Pos2>,
    /// Offset of the row drag overlay.
    _drag_row_offset: Option<Pos2>,
    /// Id of the node to show a context menu for.
    context_menu: Option<NodeIdType>,
    /// Open states of the dirs in this tree.
    dir_states: HashMap<NodeIdType, bool>,
}
impl<NodeIdType> Default for TreeViewPersistantState<NodeIdType> {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            dragged: Default::default(),
            rect: Rect::NOTHING,
            _drag_start_pos: Default::default(),
            _drag_row_offset: Default::default(),
            context_menu: Default::default(),
            dir_states: HashMap::new(),
        }
    }
}

#[derive(Clone)]
struct TreeViewState<NodeIdType>
where
    NodeIdType: Clone,
{
    /// State of the tree that is persistant across frames.
    peristant: TreeViewPersistantState<NodeIdType>,
    /// Response of the interaction.
    response: Response,
    /// NodeId and Drop position of the drop target.
    drop: Option<(NodeIdType, DropPosition<NodeIdType>)>,
    /// Shape index of the drop marker
    drop_marker_idx: ShapeIdx,
    /// Wether or not the tree view has keyboard focus.
    has_focus: bool,
    /// Order of the nodes inside the tree.
    node_order: Vec<NodeOrder<NodeIdType>>,
}
impl<NodeIdType> TreeViewState<NodeIdType>
where
    NodeIdType: Clone + Send + Sync + 'static,
{
    fn load(ui: &mut Ui, id: Id) -> Self {
        let state = ui
            .data_mut(|d| d.get_persisted::<TreeViewPersistantState<NodeIdType>>(id))
            .unwrap_or_default();

        let response = interact_no_expansion(ui, state.rect, id, Sense::click_and_drag());
        let has_focus = ui.memory(|m| m.has_focus(id));

        TreeViewState {
            peristant: state,
            drop: None,
            drop_marker_idx: ui.painter().add(Shape::Noop),
            response,
            has_focus,
            node_order: Vec::new(),
        }
    }

    fn store(self, ui: &mut Ui, id: Id) {
        ui.data_mut(|d| d.insert_persisted(id, self.peristant));
    }
}
impl<NodeIdType> TreeViewState<NodeIdType>
where
    NodeIdType: Clone,
{
    pub fn interact(&self, rect: &Rect) -> Interaction {
        if !self
            .response
            .hover_pos()
            .is_some_and(|pos| rect.contains(pos))
        {
            return Interaction {
                clicked: false,
                double_clicked: false,
                hovered: false,
                drag_started: false,
                right_clicked: false,
            };
        }

        Interaction {
            clicked: self.response.clicked(),
            double_clicked: self.response.double_clicked(),
            hovered: self.response.hovered(),
            drag_started: self.response.drag_started_by(egui::PointerButton::Primary),
            right_clicked: self.response.clicked_by(egui::PointerButton::Secondary),
        }
    }
}

#[derive(Clone)]
struct NodeOrder<NodeIdType> {
    pub depth: usize,
    pub node_id: NodeIdType,
}

struct Interaction {
    pub clicked: bool,
    pub double_clicked: bool,
    pub hovered: bool,
    pub drag_started: bool,
    pub right_clicked: bool,
}

/// Contains information about a drag and drop that the
/// tree view produced.
#[derive(Debug)]
pub struct DragDropAction<NodeIdType> {
    pub drag_id: NodeIdType,
    /// Id of the dragged node.
    /// Id of the drop node where the dragged node is added to.
    pub drop_id: NodeIdType,
    /// Position of the dragged node in the drop node.
    pub position: DropPosition<NodeIdType>,
}

/// Where a dragged item should be dropped to in a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition<NodeIdType> {
    First,
    Last,
    After(NodeIdType),
    Before(NodeIdType),
}

#[derive(Default)]
struct TreeViewSettings {
    override_indent: Option<f32>,
    vline_style: VLineStyle,
    row_layout: RowLayout,
}

/// Style of the vertical line to show the indentation level.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum VLineStyle {
    /// No vline is shown.
    None,
    /// A single vertical line is show for the full hight of the directory.
    VLine,
    /// A vline is show with horizontal hooks to the child nodes of the directory.
    #[default]
    Hook,
}

/// How rows in the tree are layed out.
///
/// Each row in the tree is made up of three elements. A closer,
/// an icon and a label. The layout of these elements is controlled
/// by this value.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum RowLayout {
    /// No icons are displayed.
    /// Directories only show the closer and the label.
    /// Leaves only show the label and allocate no additional space for a closer.
    /// Labels between leaves and directories do not align.
    Compact,
    /// The labels of leaves and directories are aligned.
    /// Icons are displayed for leaves only.
    CompactAlignedLables,
    /// The icons of leaves and directories are aligned.
    /// If a leaf or directory does not show an icon, the label will fill the
    /// space. Lables between leaves and directories can be misaligned.
    #[default]
    AlignedIcons,
    /// The labels of leaves and directories are alignd.
    /// If a leaf or directory does not show an icon, the label will not fill
    /// the space.
    AlignedIconsAndLabels,
}

pub struct TreeViewResponse<NodeIdType> {
    pub response: Response,
    /// If a row was dragged in the tree this will contain information about
    /// who was dragged to who and at what position.
    pub drag_drop_action: Option<DragDropAction<NodeIdType>>,
    /// `true` if a drag and drop was performed
    pub dropped: bool,
    /// Id of the selected node.
    pub selected_node: Option<NodeIdType>,
    /// If od the node for which to show the context menu.
    pub context_menu_node: Option<NodeIdType>,
    drop_marker_idx: ShapeIdx,
}
impl<NodeIdType> TreeViewResponse<NodeIdType> {
    /// Remove the drop marker from the tree view.
    ///
    /// Use this to remove the drop marker if a proposed drag and drop action
    /// is disallowed.
    pub fn remove_drop_marker(&self, ui: &mut Ui) {
        ui.painter().set(self.drop_marker_idx, Shape::Noop);
    }
}

fn load<T: SerializableAny>(ui: &mut Ui, id: Id) -> Option<T> {
    ui.data_mut(|d| d.get_persisted::<T>(id))
}

fn store<T: SerializableAny>(ui: &mut Ui, id: Id, value: T) {
    ui.data_mut(|d| d.insert_persisted(id, value));
}
/// Interact with the ui without egui adding any extra space.
fn interact_no_expansion(ui: &mut Ui, rect: Rect, id: Id, sense: Sense) -> Response {
    let spacing_before = ui.spacing().clone();
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let res = ui.interact(rect, id, sense);
    *ui.spacing_mut() = spacing_before;
    res
}
