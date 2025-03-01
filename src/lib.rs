pub mod builder;
pub mod node;

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use builder::{RowRectangles, TreeViewBuilderResult};
use egui::{
    self, epaint, layers::ShapeIdx, vec2, Event, EventFilter, Id, InnerResponse, Key, Layout,
    NumExt, Pos2, Rangef, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};

pub use builder::TreeViewBuilder;
use node::DropQuarter;

pub trait TreeViewId: Clone + Copy + PartialEq + Eq + Hash + std::fmt::Debug {}
impl<T> TreeViewId for T where T: Clone + Copy + PartialEq + Eq + Hash + std::fmt::Debug {}

#[cfg(feature = "persistence")]
pub trait NodeId:
    TreeViewId + Send + Sync + 'static + serde::de::DeserializeOwned + serde::Serialize
{
}
#[cfg(feature = "persistence")]
impl<T> NodeId for T where
    T: TreeViewId + Send + Sync + 'static + serde::de::DeserializeOwned + serde::Serialize
{
}

#[cfg(not(feature = "persistence"))]
pub trait NodeId: TreeViewId + Send + Sync + 'static {}
#[cfg(not(feature = "persistence"))]
impl<T> NodeId for T where T: TreeViewId + Send + Sync + 'static {}

/// Represents the state of the tree view.
///
/// This holds which node is selected and the open/close
/// state of the directories.
#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeViewState<NodeIdType> {
    /// Id of the node that was selected.
    selected: Vec<NodeIdType>,
    /// Information about the dragged node.
    dragged: Option<DragState<NodeIdType>>,
    /// Id of the node that was right clicked.
    secondary_selection: Option<NodeIdType>,
    /// The rectangle the tree view occupied.
    size: Vec2,
    /// Open states of the dirs in this tree.
    node_states: Vec<NodeState<NodeIdType>>,
}
impl<NodeIdType> Default for TreeViewState<NodeIdType> {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            dragged: Default::default(),
            secondary_selection: Default::default(),
            size: Vec2::ZERO,
            node_states: Vec::new(),
        }
    }
}
impl<NodeIdType: TreeViewId> TreeViewState<NodeIdType> {
    /// Return the list of selected nodes
    pub fn selected(&self) -> &Vec<NodeIdType> {
        &self.selected
    }

    /// Set which nodes are selected in the tree
    pub fn set_selected(&mut self, selected: Vec<NodeIdType>) {
        self.selected = selected;
    }

    /// Expand all parent nodes of the node with the given id.
    pub fn expand_parents_of(&mut self, id: NodeIdType) {
        if let Some(parent_id) = self.parent_id_of(id) {
            self.expand_node(parent_id);
        }
    }

    /// Expand the node and all its parent nodes.
    /// Effectively this makes the node visible in the tree.
    pub fn expand_node(&mut self, mut id: NodeIdType) {
        loop {
            if let Some(node_state) = self.node_state_of_mut(&id) {
                node_state.open = true;
                id = match node_state.parent_id {
                    Some(id) => id,
                    None => break,
                }
            } else {
                break;
            }
        }
    }

    /// Get the parent id of a node.
    pub fn parent_id_of(&self, id: NodeIdType) -> Option<NodeIdType> {
        self.node_state_of(&id)
            .and_then(|node_state| node_state.parent_id)
    }

    /// Get the node state for an id.
    pub(crate) fn node_state_of(&self, id: &NodeIdType) -> Option<&NodeState<NodeIdType>> {
        self.node_states.iter().find(|ns| &ns.id == id)
    }
    /// Get the node state for an id.
    pub(crate) fn node_state_of_mut(
        &mut self,
        id: &NodeIdType,
    ) -> Option<&mut NodeState<NodeIdType>> {
        self.node_states.iter_mut().find(|ns| &ns.id == id)
    }

    /// Is the current drag valid.
    /// `false` if no drag is currently registered.
    pub(crate) fn drag_valid(&self) -> bool {
        self.dragged
            .as_ref()
            .is_some_and(|drag_state| drag_state.drag_valid)
    }
    /// Is the given id part of a valid drag.
    pub(crate) fn is_dragged(&self, id: &NodeIdType) -> bool {
        self.dragged
            .as_ref()
            .is_some_and(|drag_state| drag_state.drag_valid && &drag_state.node_id == id)
    }

    pub(crate) fn is_selected(&self, id: &NodeIdType) -> bool {
        self.selected.contains(id)
    }

    pub(crate) fn is_secondary_selected(&self, id: &NodeIdType) -> bool {
        self.secondary_selection.as_ref().is_some_and(|n| n == id)
    }
}

impl<NodeIdType> TreeViewState<NodeIdType>
where
    NodeIdType: NodeId,
{
    pub fn load(ui: &mut Ui, id: Id) -> Option<Self> {
        ui.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ui: &mut Ui, id: Id) {
        ui.data_mut(|d| d.insert_persisted(id, self));
    }
}
/// State of the dragged node.
#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
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
/// State of each node in the tree.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
struct NodeState<NodeIdType> {
    /// Id of this node.
    id: NodeIdType,
    /// The parent node of this node.
    parent_id: Option<NodeIdType>,
    /// Wether the node is open or not.
    open: bool,
    /// Wether the node is visible or not.
    visible: bool,
    /// Wether this node is a valid target for drag and drop.
    drop_allowed: bool,
}

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

    /// Set the minimum width the tree can have.
    pub fn min_width(mut self, width: f32) -> Self {
        self.settings.min_width = width;
        self
    }

    /// Set the minimum hight the tree can have.
    pub fn min_height(mut self, height: f32) -> Self {
        self.settings.min_height = height;
        self
    }

    /// Start displaying the tree view.
    ///
    /// Construct the tree view using the [`TreeViewBuilder`] by adding
    /// directories or leaves to the tree.
    pub fn show<NodeIdType>(
        self,
        ui: &mut Ui,
        build_tree_view: impl FnMut(&mut TreeViewBuilder<'_, NodeIdType>),
    ) -> (Response, Vec<Action<NodeIdType>>)
    where
        NodeIdType: NodeId,
    {
        let id = self.id;
        let mut state = TreeViewState::load(ui, id).unwrap_or_default();
        let res = self.show_state(ui, &mut state, build_tree_view);
        state.store(ui, id);
        res
    }

    /// Start displaying the tree view with a [`TreeViewState`].
    ///
    /// Construct the tree view using the [`TreeViewBuilder`] by addind
    /// directories or leaves to the tree.
    pub fn show_state<NodeIdType>(
        mut self,
        ui: &mut Ui,
        state: &mut TreeViewState<NodeIdType>,
        build_tree_view: impl FnMut(&mut TreeViewBuilder<'_, NodeIdType>),
    ) -> (Response, Vec<Action<NodeIdType>>)
    where
        NodeIdType: TreeViewId + Send + Sync + 'static,
    {
        // Justified layouts override these settings
        if ui.layout().horizontal_justify() {
            self.settings.fill_space_horizontal = true;
            self.settings.max_width = f32::INFINITY;
        }
        if ui.layout().vertical_justify() {
            self.settings.fill_space_vertical = true;
            self.settings.max_height = f32::INFINITY;
        }

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

        let background_shapes = BackgroundShapes::new(ui, state);

        let InnerResponse {
            inner: tree_builder_result,
            response,
        } = self.draw_foreground(ui, state, build_tree_view);

        state.node_states = tree_builder_result.new_node_states.clone();
        let input_result = self.handle_input(ui, &tree_builder_result, state);

        self.draw_background(
            ui,
            state,
            &tree_builder_result,
            &background_shapes,
            input_result.drag_and_drop,
        );

        // Remember the size of the tree for next frame.
        state.size = response.rect.size();

        let mut actions = Vec::new();
        // Create a drag or move action.
        if state.drag_valid() {
            if let Some((drag_state, (drop_id, position))) =
                state.dragged.as_ref().zip(input_result.drag_and_drop)
            {
                if ui.ctx().input(|i| i.pointer.primary_released()) {
                    actions.push(Action::Move(DragAndDrop {
                        source: drag_state.node_id,
                        target: drop_id,
                        position,
                        drop_marker_idx: background_shapes.drop_marker_idx,
                    }))
                } else {
                    actions.push(Action::Drag(DragAndDrop {
                        source: drag_state.node_id,
                        target: drop_id,
                        position,
                        drop_marker_idx: background_shapes.drop_marker_idx,
                    }))
                }
            }
        }
        // Create a selection action.
        if input_result.selection_changed {
            actions.push(Action::SetSelected(state.selected.clone()));
        }

        // Reset the drag state.
        if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            state.dragged = None;
        }

        (tree_builder_result.interaction, actions)
    }

    fn draw_foreground<NodeIdType: TreeViewId>(
        &mut self,
        ui: &mut Ui,
        state: &mut TreeViewState<NodeIdType>,
        mut build_tree_view: impl FnMut(&mut TreeViewBuilder<'_, NodeIdType>),
    ) -> InnerResponse<TreeViewBuilderResult<NodeIdType>> {
        // Calculate the desired size of the tree view widget.
        let size = vec2(
            if self.settings.fill_space_horizontal {
                ui.available_width().at_most(self.settings.max_width)
            } else {
                state.size.x.at_most(self.settings.max_width)
            }
            .at_least(self.settings.min_width),
            if self.settings.fill_space_vertical {
                ui.available_height().at_most(self.settings.max_height)
            } else {
                state.size.y.at_most(self.settings.max_height)
            }
            .at_least(self.settings.min_height),
        );

        let interaction_response = interact_no_expansion(
            ui,
            Rect::from_min_size(ui.cursor().min, size),
            self.id,
            Sense::click_and_drag(),
        );

        // Run the build tree view closure
        let response = ui.allocate_ui_with_layout(size, Layout::top_down(egui::Align::Min), |ui| {
            ui.set_min_size(vec2(self.settings.min_width, self.settings.min_height));
            ui.add_space(ui.spacing().item_spacing.y * 0.5);

            let mut tree_builder = TreeViewBuilder::new(
                ui,
                interaction_response,
                state,
                &self.settings,
                ui.memory(|m| m.has_focus(self.id)),
            );
            build_tree_view(&mut tree_builder);
            let tree_builder_response = tree_builder.get_result();

            // Add negative space because the place will add the item spacing on top of this.
            ui.add_space(-ui.spacing().item_spacing.y * 0.5);

            if self.settings.fill_space_horizontal {
                ui.set_min_width(ui.available_width());
            }
            if self.settings.fill_space_vertical {
                ui.set_min_height(ui.available_height());
            }
            tree_builder_response
        });
        response
    }

    fn handle_input<NodeIdType: TreeViewId>(
        &mut self,
        ui: &mut Ui,
        tree_view_result: &TreeViewBuilderResult<NodeIdType>,
        state: &mut TreeViewState<NodeIdType>,
    ) -> InputResult<NodeIdType> {
        let TreeViewBuilderResult {
            row_rectangles,
            seconday_click,
            interaction,
            ..
        } = tree_view_result;

        if interaction.clicked() || interaction.drag_started() {
            ui.memory_mut(|m| m.request_focus(self.id));
        }

        // Transfer the secondary click
        if seconday_click.is_some() {
            state.secondary_selection = *seconday_click;
        }
        if !interaction.context_menu_opened() {
            state.secondary_selection = None;
        }

        let mut selection_changed = false;

        let node_ids = state.node_states.iter().map(|ns| ns.id).collect::<Vec<_>>();
        for node_id in node_ids {
            let RowRectangles {
                row_rect,
                closer_rect,
            } = row_rectangles
                .get(&node_id)
                .expect("A node_state must have row rectangles");

            // Closer interactions
            if let Some(closer_rect) = closer_rect {
                if interaction
                    .hover_pos()
                    .is_some_and(|pos| closer_rect.contains(pos))
                {
                    // was closed clicked
                    if interaction.clicked() {
                        let node_state = state.node_state_of_mut(&node_id).unwrap();
                        node_state.open = !node_state.open;
                    }
                }
            }
            // Row interaction
            if interaction
                .hover_pos()
                .is_some_and(|pos| row_rect.contains(pos))
            {
                // was clicked
                if interaction.clicked() {
                    // React to primary clicking
                    if ui.ctx().input(|is| is.modifiers.ctrl) {
                        selection_changed = true;
                        state.selected.push(node_id);
                    } else {
                        selection_changed = true;
                        state.selected = vec![node_id];
                    }
                }
                // was row double clicked
                if interaction.double_clicked() {
                    let node_state = state.node_state_of_mut(&node_id).unwrap();
                    node_state.open = !node_state.open;
                }
                // React to a dragging
                // An egui drag only starts after the pointer has moved but with that first movement
                // the pointer may have moved to a different node. Instead we want to update
                // the drag state right when the priamry button was pressed.
                // We also want to have our own rules when a drag really becomes valid to avoid
                // graphical artifacts. Sometimes the user is a little fast with the mouse and
                // it creates the drag overlay when it really shouldn't have.
                let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
                if primary_pressed {
                    let pointer_pos = ui.ctx().pointer_latest_pos().unwrap_or_default();
                    state.dragged = Some(DragState {
                        node_id: node_id,
                        drag_row_offset: row_rect.min - pointer_pos,
                        drag_start_pos: pointer_pos,
                        drag_valid: false,
                    });
                }
            }
        }

        let mut drop_position = None;
        if state.drag_valid() {
            // Search the node states for the correct drop target.
            // If a node is dragged to a child node then that drop target is invalid.
            let mut invalid_drop_targets = HashSet::new();
            if let Some(drag_state) = &state.dragged {
                invalid_drop_targets.insert(drag_state.node_id);
            }
            for node_state in &state.node_states {
                // Dropping a node on itself is technically a fine thing to do
                // but it causes all sorts of problems for the implementer of the drop action.
                // They would have to remove a node and then somehow insert it after itself.
                // For that reason it is easier to disallow dropping on itself altogether.
                if invalid_drop_targets.contains(&node_state.id) {
                    continue;
                }
                // If the parent of a node is in the list of invalid drop targets that means
                // it is a distant child of the dragged node. This is not allowed
                if let Some(parent_id) = node_state.parent_id {
                    if invalid_drop_targets.contains(&parent_id) {
                        invalid_drop_targets.insert(node_state.id);
                        continue;
                    }
                }
                // At this point we have a potentially valid node to drop on.
                // Now we only need to check if the mouse is over the node, get the correct
                // drop quarter and then get the correct drop position.
                let row_rectangles = row_rectangles.get(&node_state.id).unwrap();
                let drop_quarter = interaction
                    .hover_pos()
                    .and_then(|pos| DropQuarter::new(row_rectangles.row_rect.y_range(), pos.y));
                if let Some(drop_quarter) = drop_quarter {
                    drop_position = get_drop_position_node(node_state, &drop_quarter);
                    break;
                }
            }
        }

        if ui.memory(|m| m.has_focus(self.id)) {
            // If the widget is focused but no node is selected we want to select any node
            // to allow navigating throught the tree.
            // In case we gain focus from a drag action we select the dragged node directly.
            if state.selected.is_empty() {
                // todo: fix this
                state.selected = state
                    .dragged
                    .as_ref()
                    .map(|drag_state| vec![drag_state.node_id])
                    .or(state.node_states.first().map(|n| vec![n.id]))
                    .unwrap();
                selection_changed = true;
            }
            ui.input(|i| {
                for event in i.events.iter() {
                    match event {
                        Event::Key { key, pressed, .. } if *pressed => {
                            selection_changed |= handle_input(state, key);
                        }
                        _ => (),
                    }
                }
            });
        }
        // Update the drag state
        // A drag only becomes a valid drag after the pointer has traveled some distance.
        if let Some(drag_state) = state.dragged.as_mut() {
            if !drag_state.drag_valid {
                drag_state.drag_valid = drag_state
                    .drag_start_pos
                    .distance(ui.ctx().pointer_latest_pos().unwrap_or_default())
                    > 5.0;
            }
        }

        InputResult {
            drag_and_drop: drop_position,
            selection_changed,
        }
    }

    fn draw_background<NodeIdType: TreeViewId>(
        &self,
        ui: &mut Ui,
        state: &TreeViewState<NodeIdType>,
        result: &TreeViewBuilderResult<NodeIdType>,
        background: &BackgroundShapes<NodeIdType>,
        drop_position: Option<(NodeIdType, DropPosition<NodeIdType>)>,
    ) {
        pub const DROP_LINE_HEIGHT: f32 = 3.0;
        if let Some((parent_id, drop_position)) = drop_position {
            let drop_marker = match drop_position {
                DropPosition::Before(target_id) => {
                    let row_rectangles = result.row_rectangles.get(&target_id).unwrap();
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.min.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DropPosition::After(target_id) => {
                    let row_rectangles = result.row_rectangles.get(&target_id).unwrap();
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.max.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DropPosition::First => {
                    let row_rectangles = result.row_rectangles.get(&parent_id).unwrap();
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.max.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DropPosition::Last => {
                    let row_rectangles_start = result.row_rectangles.get(&parent_id).unwrap();
                    // For directories the drop marker should expand its height to include all
                    // its child nodes. To do this, first we have to find its last child node,
                    // then we can get the correct y range.
                    let mut last_child = None;
                    let mut child_nodes = HashSet::<NodeIdType>::new();
                    child_nodes.insert(parent_id);
                    for node in &state.node_states {
                        if let Some(parent_id) = node.parent_id {
                            if child_nodes.contains(&parent_id) {
                                child_nodes.insert(node.id);
                                last_child = Some(node.id);
                            }
                        }
                    }
                    let y_range = match last_child {
                        Some(last_child_id) => {
                            let row_rectangles_end =
                                result.row_rectangles.get(&last_child_id).unwrap();
                            Rangef::new(
                                row_rectangles_start.row_rect.min.y,
                                row_rectangles_end.row_rect.max.y,
                            )
                        }
                        None => row_rectangles_start.row_rect.y_range(),
                    };
                    Rect::from_x_y_ranges(row_rectangles_start.row_rect.x_range(), y_range)
                }
            };

            let shape = epaint::RectShape::new(
                drop_marker,
                ui.visuals().widgets.active.corner_radius,
                ui.style().visuals.selection.bg_fill.linear_multiply(0.6),
                Stroke::NONE,
                egui::StrokeKind::Inside,
            );
            ui.painter().set(background.drop_marker_idx, shape);
        }

        for selected_node in state.selected() {
            let row_rectangles = result.row_rectangles.get(selected_node).unwrap();
            ui.painter().set(
                *background
                    .background_idx
                    .get(selected_node)
                    .unwrap_or(&background.background_idx_backup),
                epaint::RectShape::new(
                    row_rectangles.row_rect,
                    ui.visuals().widgets.active.corner_radius,
                    if ui.memory(|m| m.has_focus(self.id)) {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals()
                            .widgets
                            .inactive
                            .weak_bg_fill
                            .linear_multiply(0.3)
                    },
                    Stroke::NONE,
                    egui::StrokeKind::Inside,
                ),
            );
        }
        if let Some(seconday_selected_id) = state.secondary_selection {
            let row_rectangles = result.row_rectangles.get(&seconday_selected_id).unwrap();
            ui.painter().set(
                background.secondary_selection_idx,
                epaint::RectShape::new(
                    row_rectangles.row_rect,
                    ui.visuals().widgets.active.corner_radius,
                    egui::Color32::TRANSPARENT,
                    ui.visuals().widgets.inactive.fg_stroke,
                    egui::StrokeKind::Inside,
                ),
            );
        }
    }
}

fn get_drop_position_node<NodeIdType: TreeViewId>(
    node: &NodeState<NodeIdType>,
    drop_quater: &DropQuarter,
) -> Option<(NodeIdType, DropPosition<NodeIdType>)> {
    match drop_quater {
        DropQuarter::Top => {
            if let Some(parent_id) = node.parent_id {
                return Some((parent_id, DropPosition::Before(node.id)));
            }
            if node.drop_allowed {
                return Some((node.id, DropPosition::Last));
            }
            None
        }
        DropQuarter::MiddleTop => {
            if node.drop_allowed {
                return Some((node.id, DropPosition::Last));
            }
            if let Some(parent_id) = node.parent_id {
                return Some((parent_id, DropPosition::Before(node.id)));
            }
            None
        }
        DropQuarter::MiddleBottom => {
            if node.drop_allowed {
                return Some((node.id, DropPosition::Last));
            }
            if let Some(parent_id) = node.parent_id {
                return Some((parent_id, DropPosition::After(node.id)));
            }
            None
        }
        DropQuarter::Bottom => {
            if node.drop_allowed && node.open {
                return Some((node.id, DropPosition::First));
            }
            if let Some(parent_id) = node.parent_id {
                return Some((parent_id, DropPosition::After(node.id)));
            }
            if node.drop_allowed {
                return Some((node.id, DropPosition::Last));
            }
            None
        }
    }
}

fn handle_input<NodeIdType: TreeViewId>(state: &mut TreeViewState<NodeIdType>, key: &Key) -> bool {
    if state.selected.is_empty() {
        return false;
    }
    let mut selection_changed = false;

    let first_selected_node_index = state
        .selected
        .first()
        .and_then(|first_selected_node| {
            state
                .node_states
                .iter()
                .position(|ns| &ns.id == first_selected_node)
        })
        .expect("List is not empty to this must exists");
    let last_selected_node_index = state
        .selected
        .last()
        .and_then(|last_selected_node| {
            state
                .node_states
                .iter()
                .position(|ns| &ns.id == last_selected_node)
        })
        .expect("List is not empty to this must exists");

    match key {
        Key::ArrowUp => {
            if first_selected_node_index > 0 {
                if let Some(node) =
                    // Search for previous visible node.
                    state.node_states[0..first_selected_node_index]
                        .iter()
                        .rev()
                        .find(|node| node.visible)
                {
                    state.selected = vec![node.id];
                    selection_changed = true;
                }
            }
        }
        Key::ArrowDown => {
            if last_selected_node_index < state.node_states.len() - 1 {
                // Search for next visible node.
                if let Some(node) = state.node_states[(last_selected_node_index + 1)..]
                    .iter()
                    .find(|node| node.visible)
                {
                    state.selected = vec![node.id];
                    selection_changed = true;
                }
            }
        }
        Key::ArrowLeft => {
            if state.selected.len() == 1 {
                let node_state = &mut state.node_states[first_selected_node_index];
                if node_state.open {
                    node_state.open = false;
                } else if let Some(parent_id) = node_state.parent_id {
                    state.selected = vec![parent_id];
                    selection_changed = true;
                }
            }
        }
        Key::ArrowRight => {
            if state.selected.len() == 1 {
                let node_state = &mut state.node_states[first_selected_node_index];
                if node_state.open {
                    if first_selected_node_index < state.node_states.len() - 1 {
                        // Search for next visible node.
                        if let Some(node) = state.node_states[(first_selected_node_index + 1)..]
                            .iter()
                            .find(|node| node.visible)
                        {
                            state.selected = vec![node.id];
                            selection_changed = true;
                        }
                    }
                } else {
                    node_state.open = true;
                }
            }
        }
        _ => (),
    };
    selection_changed
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
    indent_hint_style: IndentHintStyle,
    row_layout: RowLayout,
    max_width: f32,
    max_height: f32,
    min_width: f32,
    min_height: f32,
    fill_space_horizontal: bool,
    fill_space_vertical: bool,
}

impl Default for TreeViewSettings {
    fn default() -> Self {
        Self {
            override_indent: None,
            indent_hint_style: Default::default(),
            row_layout: Default::default(),
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            min_width: 0.0,
            min_height: 0.0,
            fill_space_horizontal: true,
            fill_space_vertical: false,
        }
    }
}

/// Style of the vertical line to show the indentation level.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum IndentHintStyle {
    /// No indent hint is shown.
    None,
    /// A single vertical line is show for the full hight of the directory.
    Line,
    /// A vertical line is show with horizontal hooks to the child nodes of the directory.
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
    SetSelected(Vec<NodeIdType>),
    /// Move a node from one place to another.
    Move(DragAndDrop<NodeIdType>),
    /// An inprocess drag and drop action where the node
    /// is currently dragged but not yet dropped.
    Drag(DragAndDrop<NodeIdType>),
}

/// Information about drag and drop action that is currently
/// happening on the tree.
#[derive(Clone)]
pub struct DragAndDrop<NodeIdType> {
    /// The node that is beeing dragged
    pub source: NodeIdType,
    /// The node where the dragged node is dropped on.
    pub target: NodeIdType,
    /// The position where the dragged node is dropped inside the target node.
    pub position: DropPosition<NodeIdType>,
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

/// Interact with the ui without egui adding any extra space.
fn interact_no_expansion(ui: &mut Ui, rect: Rect, id: Id, sense: Sense) -> Response {
    let spacing_before = ui.spacing().clone();
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let res = ui.interact(rect, id, sense);
    *ui.spacing_mut() = spacing_before;
    res
}

struct BackgroundShapes<NodeIdType> {
    background_idx: HashMap<NodeIdType, ShapeIdx>,
    background_idx_backup: ShapeIdx,
    secondary_selection_idx: ShapeIdx,
    drop_marker_idx: ShapeIdx,
}
impl<NodeIdType: TreeViewId> BackgroundShapes<NodeIdType> {
    fn new(ui: &mut Ui, state: &TreeViewState<NodeIdType>) -> Self {
        let mut background_indices = HashMap::new();
        state.node_states.iter().for_each(|ns| {
            background_indices.insert(ns.id, ui.painter().add(Shape::Noop));
        });
        Self {
            background_idx: background_indices,
            background_idx_backup: ui.painter().add(Shape::Noop),
            secondary_selection_idx: ui.painter().add(Shape::Noop),
            drop_marker_idx: ui.painter().add(Shape::Noop),
        }
    }
}

struct InputResult<NodeIdType> {
    drag_and_drop: Option<(NodeIdType, DropPosition<NodeIdType>)>,
    selection_changed: bool,
}
