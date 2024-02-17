pub mod builder;
mod row;

use std::collections::HashMap;

use egui::{
    self, epaint, layers::ShapeIdx, vec2, Event, EventFilter, Id, Key, Layout, NumExt, Pos2, Rect,
    Response, Sense, Shape, Ui, Vec2,
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

    /// Set whether or not the tree should fill all available horizontal space.
    ///
    /// If the tree is part of a horizontally justified layout, this property has no
    /// effect and the tree will always fill horizontal space.
    ///
    /// Default is `true`.
    pub fn fill_space_horizontal(mut self, fill_space_horizontal: bool) -> Self {
        self.settings.fill_space_horizontal = fill_space_horizontal;
        self
    }

    /// Set whether or not the tree should fill all available vertical space.
    ///
    /// If the tree is part of a vertically justified layout, this property has no
    /// effect and the tree will always fill vertical space.
    ///
    /// Default is `false`.
    pub fn fill_space_vertical(mut self, fill_space_vertical: bool) -> Self {
        self.settings.fill_space_vertical = fill_space_vertical;
        self
    }

    /// Set the maximum width the tree can have.
    ///
    /// If the tree is part of a horizontally justified layout, this property has no
    /// effect and the tree will always fill the available horizontal space.
    pub fn max_width(mut self, width: f32) -> Self {
        self.settings.max_width = width;
        self
    }

    /// Set the maximum hight the tree can have.
    ///
    /// If the tree is part of a vertical justified layout, this property has no
    /// effect and the tree will always fill the available vertical space.
    pub fn max_height(mut self, height: f32) -> Self {
        self.settings.max_height = height;
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
        NodeIdType: Clone + Copy + Send + Sync + std::hash::Hash + PartialEq + Eq + 'static,
    {
        let mut state = TreeViewState::load(ui, self.id);
        let prev_selection = state.peristant.selected;

        // Set the focus filter to get correct keyboard navigation while focused.
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                self.id,
                EventFilter {
                    tab: false,
                    escape: false,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                },
            )
        });

        // Calculate the desired size of the tree view widget.
        let size = vec2(
            if ui.layout().horizontal_justify() {
                ui.available_width()
            } else if self.settings.fill_space_horizontal {
                self.settings.max_width.at_most(ui.available_width())
            } else {
                state.peristant.size.x.at_most(self.settings.max_width)
            },
            if ui.layout().vertical_justify() {
                ui.available_height()
            } else if self.settings.fill_space_vertical {
                self.settings.max_height.at_most(ui.available_height())
            } else {
                state.peristant.size.y.at_most(self.settings.max_height)
            },
        );

        // Run the build tree view closure
        let used_rect = ui
            .allocate_ui_with_layout(size, Layout::top_down(egui::Align::Min), |ui| {
                ui.add_space(ui.spacing().item_spacing.y * 0.5);
                build_tree_view(TreeViewBuilder::new(ui, &mut state, &self.settings));
                // Add negative space because the place will add the item spacing on top of this.
                ui.add_space(-ui.spacing().item_spacing.y * 0.5);
                if self.settings.fill_space_horizontal {
                    ui.allocate_space(vec2(ui.available_width(), 0.0));
                }
                if self.settings.fill_space_vertical {
                    ui.allocate_space(vec2(0.0, ui.available_height()));
                }
            })
            .response
            .rect;

        // If the tree was clicked it should receive focus.
        let tree_view_interact = state.interact(&used_rect);
        if tree_view_interact.clicked || tree_view_interact.drag_started {
            ui.memory_mut(|m| m.request_focus(self.id));
        }

        if ui.memory(|m| m.has_focus(self.id)) {
            // If the widget is focused but no node is selected we want to select any node
            // to allow navigating throught the tree.
            // In case we gain focus from a drag action we select the dragged node directly.
            if state.peristant.selected.is_none() {
                state.peristant.selected = state
                    .peristant
                    .dragged
                    .as_ref()
                    .map(|drag_state| drag_state.node_id)
                    .or(state.node_info.first().map(|n| n.node_id));
            }
            ui.input(|i| {
                for event in i.events.iter() {
                    match event {
                        Event::Key { key, pressed, .. } if *pressed => {
                            handle_input(&mut state, key)
                        }
                        _ => (),
                    }
                }
            });
        }

        // Create a drag or move action.
        if state.drag_valid() {
            if let Some((drag_state, (drop_id, position))) =
                state.peristant.dragged.as_ref().zip(state.drop)
            {
                if ui.ctx().input(|i| i.pointer.any_released()) {
                    state.actions.push(Action::Move {
                        source: drag_state.node_id,
                        target: drop_id,
                        position,
                    })
                } else {
                    state.actions.push(Action::Drag {
                        source: drag_state.node_id,
                        target: drop_id,
                        position,
                    })
                }
            }
        }
        // Create a selection action.
        if state.peristant.selected != prev_selection {
            state
                .actions
                .push(Action::SetSelected(state.peristant.selected));
        }

        // Reset the drag state.
        if state.interaction_response.drag_released() {
            state.peristant.dragged = None;
        }

        // Remember the size of the tree for next frame.
        state.peristant.size = used_rect.size();

        let res = TreeViewResponse {
            response: state.interaction_response,
            drop_marker_idx: state.drop_marker_idx,
            context_menu_marker_idx: state.context_menu_marker_idx,
            selected_node: state.peristant.selected,
            nodes: state.node_info,
            actions: state.actions,
        };

        state.peristant.store(ui, self.id);
        res
    }
}

fn handle_input<NodeIdType>(state: &mut TreeViewState<NodeIdType>, key: &Key)
where
    NodeIdType: Clone + Copy + PartialEq + Eq + std::hash::Hash,
{
    let Some(selected_index) = state
        .node_info
        .iter()
        .position(|n| Some(n.node_id) == state.peristant.selected)
    else {
        return;
    };
    let selected_node = state.node_info[selected_index].node_id;
    let selected_depth = state.node_info[selected_index].depth;
    let first_parent = state.node_info[0..selected_index]
        .iter()
        .rev()
        .find(|n| n.depth < selected_depth)
        .map(|n| n.node_id);

    match key {
        Key::ArrowUp => {
            if selected_index > 0 {
                if let Some(node) =
                    // Search for previous visible node.
                    state.node_info[0..selected_index]
                        .iter()
                        .rev()
                        .find(|node| node.visible)
                {
                    state.peristant.selected = Some(node.node_id);
                }
            }
        }
        Key::ArrowDown => {
            if selected_index < state.node_info.len() - 1 {
                // Search for previous visible node.
                if let Some(node) = state.node_info[(selected_index + 1)..]
                    .iter()
                    .find(|node| node.visible)
                {
                    state.peristant.selected = Some(node.node_id);
                }
            }
        }
        Key::ArrowLeft => {
            if let Some(dir_open) = state.peristant.dir_states.get_mut(&selected_node) {
                if *dir_open {
                    *dir_open = false;
                } else if let Some(first_parent) = first_parent {
                    state.peristant.selected = Some(first_parent);
                }
            } else if let Some(first_parent) = first_parent {
                state.peristant.selected = Some(first_parent);
            }
        }
        Key::ArrowRight => {
            if let Some(dir_open) = state.peristant.dir_states.get_mut(&selected_node) {
                if *dir_open {
                    if selected_index < state.node_info.len() - 1 {
                        // Search for previous visible node.
                        if let Some(node) = state.node_info[(selected_index + 1)..]
                            .iter()
                            .find(|node| node.visible)
                        {
                            state.peristant.selected = Some(node.node_id);
                        }
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
    /// Information about the dragged node.
    dragged: Option<DragState<NodeIdType>>,
    /// The rectangle the tree view occupied.
    size: Vec2,
    /// Open states of the dirs in this tree.
    dir_states: HashMap<NodeIdType, bool>,
}
impl<NodeIdType> Default for TreeViewPersistantState<NodeIdType> {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            dragged: Default::default(),
            size: Vec2::ZERO,
            dir_states: HashMap::new(),
        }
    }
}
impl<NodeIdType> TreeViewPersistantState<NodeIdType>
where
    NodeIdType: Clone + Send + Sync + 'static,
{
    fn store(self, ui: &mut Ui, id: Id) {
        ui.data_mut(|d| d.insert_persisted(id, self));
    }
}

#[derive(Clone)]
struct DragState<NodeIdType> {
    /// Id of the dragged node.
    pub node_id: NodeIdType,
    /// Offset of the drag overlay to the pointer.
    pub drag_row_offset: Vec2,
    /// Position of the pointer when the drag started.
    pub drag_start_pos: Pos2,
    /// A drag only becomes valid after it has been dragged for
    /// a short distance.
    pub drag_valid: bool,
}

#[derive(Clone)]
struct TreeViewState<NodeIdType>
where
    NodeIdType: Clone,
{
    /// State of the tree that is persistant across frames.
    peristant: TreeViewPersistantState<NodeIdType>,
    /// Response of the interaction.
    interaction_response: Response,
    /// NodeId and Drop position of the drop target.
    drop: Option<(NodeIdType, DropPosition<NodeIdType>)>,
    /// Shape index of the drop marker
    drop_marker_idx: ShapeIdx,
    /// Shape index of the right click context menu marker.
    context_menu_marker_idx: ShapeIdx,
    /// Wether or not the tree view has keyboard focus.
    has_focus: bool,
    /// Info about each node in the tree.
    node_info: Vec<NodeInfo<NodeIdType>>,
    /// Actions for the tree view.
    actions: Vec<Action<NodeIdType>>,
}
impl<NodeIdType> TreeViewState<NodeIdType>
where
    NodeIdType: Clone + Send + Sync + 'static,
{
    fn load(ui: &mut Ui, id: Id) -> Self {
        let state = ui
            .data_mut(|d| d.get_persisted::<TreeViewPersistantState<NodeIdType>>(id))
            .unwrap_or_default();

        let interaction_response = interact_no_expansion(
            ui,
            Rect::from_min_size(ui.cursor().min, state.size),
            id,
            Sense::click_and_drag(),
        );
        let has_focus = ui.memory(|m| m.has_focus(id));

        TreeViewState {
            peristant: state,
            drop: None,
            drop_marker_idx: ui.painter().add(Shape::Noop),
            context_menu_marker_idx: ui.painter().add(Shape::Noop),
            interaction_response,
            has_focus,
            node_info: Vec::new(),
            actions: Vec::new(),
        }
    }
}
impl<NodeIdType> TreeViewState<NodeIdType>
where
    NodeIdType: Clone + PartialEq + Eq,
{
    pub fn interact(&self, rect: &Rect) -> Interaction {
        if !self
            .interaction_response
            .hover_pos()
            .is_some_and(|pos| rect.contains(pos))
        {
            return Interaction {
                clicked: false,
                double_clicked: false,
                hovered: false,
                drag_started: false,
            };
        }

        Interaction {
            clicked: self.interaction_response.clicked(),
            double_clicked: self.interaction_response.double_clicked(),
            hovered: self.interaction_response.hovered(),
            drag_started: self
                .interaction_response
                .drag_started_by(egui::PointerButton::Primary),
        }
    }
    /// Is the current drag valid.
    /// `false` if no drag is currently registered.
    pub fn drag_valid(&self) -> bool {
        self.peristant
            .dragged
            .as_ref()
            .is_some_and(|drag_state| drag_state.drag_valid)
    }
    /// Is the given id part of a valid drag.
    pub fn is_dragged(&self, id: &NodeIdType) -> bool {
        self.peristant
            .dragged
            .as_ref()
            .is_some_and(|drag_state| drag_state.drag_valid && &drag_state.node_id == id)
    }

    pub fn is_selected(&self, id: &NodeIdType) -> bool {
        self.peristant.selected.as_ref().is_some_and(|n| n == id)
    }
}

#[derive(Clone)]
struct NodeInfo<NodeIdType> {
    pub depth: usize,
    pub node_id: NodeIdType,
    pub rect: Rect,
    pub parent_node_id: Option<NodeIdType>,
    pub visible: bool,
}

struct Interaction {
    pub clicked: bool,
    pub double_clicked: bool,
    pub hovered: bool,
    pub drag_started: bool,
}

/// Contains information about a drag and drop that the
/// tree view produced.
#[derive(Debug)]
pub struct DragDropAction<NodeIdType> {
    /// Id of the dragged node.
    pub source: NodeIdType,
    /// Id of the node where the dragged node is added to.
    pub target: NodeIdType,
    /// Position of the dragged node in the drop node.
    pub position: DropPosition<NodeIdType>,
    /// Wether or not the dnd is just hovering or should be commited.  
    /// `true` -> The drag and drop should be commited.  
    /// `false` -> The drag and drop is hovering.
    pub commit: bool,
}

/// Where a dragged item should be dropped to in a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition<NodeIdType> {
    First,
    Last,
    After(NodeIdType),
    Before(NodeIdType),
}

struct TreeViewSettings {
    override_indent: Option<f32>,
    vline_style: VLineStyle,
    row_layout: RowLayout,
    max_width: f32,
    max_height: f32,
    fill_space_horizontal: bool,
    fill_space_vertical: bool,
}

impl Default for TreeViewSettings {
    fn default() -> Self {
        Self {
            override_indent: None,
            vline_style: Default::default(),
            row_layout: Default::default(),
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            fill_space_horizontal: true,
            fill_space_vertical: false,
        }
    }
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

/// An action the tree view would like to take as a result
/// of some user input like drag and drop.
#[derive(Clone)]
pub enum Action<NodeIdType> {
    /// Set the selected node to be this.
    SetSelected(Option<NodeIdType>),
    /// Move a node from one place to another.
    Move {
        source: NodeIdType,
        target: NodeIdType,
        position: DropPosition<NodeIdType>,
    },
    /// An inprocess drag and drop action where the node
    /// is currently dragged but not yet dropped.
    Drag {
        source: NodeIdType,
        target: NodeIdType,
        position: DropPosition<NodeIdType>,
    },
}

pub struct TreeViewResponse<NodeIdType> {
    pub response: Response,
    /// Actions this tree view would like to perform.
    pub actions: Vec<Action<NodeIdType>>,
    // /// If a row was dragged in the tree this will contain information about
    // /// who was dragged to who and at what position.
    // pub drag_drop_action: Option<DragDropAction<NodeIdType>>,
    // /// Id of the selected node.
    selected_node: Option<NodeIdType>,
    drop_marker_idx: ShapeIdx,
    context_menu_marker_idx: ShapeIdx,
    nodes: Vec<NodeInfo<NodeIdType>>,
}
impl<NodeIdType> TreeViewResponse<NodeIdType>
where
    NodeIdType: Clone + Copy + PartialEq + Eq,
{
    /// Remove the drop marker from the tree view.
    ///
    /// Use this to remove the drop marker if a proposed drag and drop action
    /// is disallowed.
    pub fn remove_drop_marker(&self, ui: &mut Ui) {
        ui.painter().set(self.drop_marker_idx, Shape::Noop);
    }

    /// Get the parent node id of a node.
    pub fn parent_of(&self, id: NodeIdType) -> Option<NodeIdType> {
        self.nodes
            .iter()
            .find(|n| n.node_id == id)
            .and_then(|node_info| node_info.parent_node_id)
    }

    /// Show a context menu for the tree view.
    ///
    /// Use the provided node id to identify which node was clicked.
    pub fn context_menu(&self, ui: &mut Ui, mut add_context_menu: impl FnMut(&mut Ui, NodeIdType)) {
        let mut clicked_node = None;
        self.response.clone().context_menu(|ui| {
            let has_context_menu_moved = {
                let last_pos_id = Id::new("Tree View context menu last pos");
                let last_pos = ui.data_mut(|d| d.get_persisted::<Pos2>(last_pos_id));
                ui.data_mut(|d| d.insert_persisted(last_pos_id, ui.cursor().min));
                last_pos.map_or(true, |last_pos| last_pos != ui.cursor().min)
            };
            let cursor_position = {
                let cursor_pos_id = Id::new("Tree view context menu cursor pos");
                if has_context_menu_moved {
                    let pos = ui.ctx().pointer_latest_pos();
                    ui.data_mut(|d| d.insert_persisted(cursor_pos_id, pos));
                    pos
                } else {
                    ui.data_mut(|d| d.get_persisted::<Option<Pos2>>(cursor_pos_id))
                        .flatten()
                }
            };
            clicked_node = cursor_position
                .and_then(|pos| self.nodes.iter().find(|node| node.rect.contains(pos)));
            if let Some(node) = clicked_node {
                add_context_menu(ui, node.node_id)
            }
        });
        if let Some(node) = clicked_node {
            if Some(node.node_id) != self.selected_node {
                let stroke = ui.visuals().widgets.inactive.fg_stroke;
                ui.painter().set(
                    self.context_menu_marker_idx,
                    epaint::RectShape::new(
                        node.rect.expand(-stroke.width),
                        ui.visuals().widgets.active.rounding,
                        egui::Color32::TRANSPARENT,
                        stroke,
                    ),
                );
            }
        }
    }
}

/// Interact with the ui without egui adding any extra space.
fn interact_no_expansion(ui: &mut Ui, rect: Rect, id: Id, sense: Sense) -> Response {
    let spacing_before = ui.spacing().clone();
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let res = ui.interact(rect, id, sense);
    *ui.spacing_mut() = spacing_before;
    res
}
