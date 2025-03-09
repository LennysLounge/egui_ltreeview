#![warn(missing_docs)]

#![doc = include_str!("../README.md")]

mod builder;
mod node;
mod state;

use egui::{
    self, emath, epaint, layers::ShapeIdx, vec2, Event, EventFilter, Id, InnerResponse, Layout,
    NumExt, Rangef, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};
use std::{cmp::Ordering, collections::HashSet, hash::Hash};

pub use builder::*;
pub use node::*;
pub use state::*;

/// A node in the tree is identified by an id that must implement this trait.
///
/// This is just a trait alias for the collection of necessary traits that a node id
/// must implement.
#[cfg(not(feature = "persistence"))]
pub trait NodeId: Clone + Copy + PartialEq + Eq + Hash {}
#[cfg(not(feature = "persistence"))]
impl<T> NodeId for T where T: Clone + Copy + PartialEq + Eq + Hash {}

#[cfg(feature = "persistence")]
/// A node in the tree is identified by an id that must implement this trait.
///
/// This is just a trait alias for the collection of necessary traits that a node id
/// must implement.
pub trait NodeId:
    Clone + Copy + PartialEq + Eq + Hash + serde::de::DeserializeOwned + serde::Serialize
{
}
#[cfg(feature = "persistence")]
impl<T> NodeId for T where
    T: Clone + Copy + PartialEq + Eq + Hash + serde::de::DeserializeOwned + serde::Serialize
{
}

/// A tree view widget.
pub struct TreeView<'context_menu, NodeIdType> {
    id: Id,
    settings: TreeViewSettings,
    fallback_context_menu: Option<Box<dyn FnMut(&mut Ui, &Vec<NodeIdType>) + 'context_menu>>,
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

    /// Set the settings for this tree view with the [`TreeViewSettings`] struct.
    pub fn with_settings(mut self, settings: TreeViewSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Override the indent value for this tree view.
    ///
    /// By default this value is 'None' which means that the indent value from the
    /// current ui is used. If this value is set, this value will used as the indent
    /// value without affecting the ui's indent value.
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

    /// Set if the tree view is allowed to select multiple nodes at once.
    pub fn allow_multi_selection(mut self, allow_multi_select: bool) -> Self {
        self.settings.allow_multi_select = allow_multi_select;
        self
    }

    /// Add a fallback context menu to the tree.
    ///
    /// If the node did not configure a context menu directly or
    /// if multiple nodes were selected and right clicked, then
    /// this fallback context menu will be opened.
    ///
    /// A context menu in egui gets its size the first time it becomes visible.
    /// Since all nodes in the tree view share the same context menu you must set
    /// the size of the context menu manually for each node if you want to have differently
    /// sized context menus.
    pub fn fallback_context_menu(
        mut self,
        context_menu: impl FnMut(&mut Ui, &Vec<NodeIdType>) + 'context_menu,
    ) -> Self {
        self.fallback_context_menu = Some(Box::new(context_menu));
        self
    }

    /// Start displaying the tree view.
    ///
    /// Construct the tree view using the [`TreeViewBuilder`] by adding
    /// directories or leaves to the tree.
    pub fn show(
        self,
        ui: &mut Ui,
        build_tree_view: impl FnMut(&mut TreeViewBuilder<'_, NodeIdType>),
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
    /// Construct the tree view using the [`TreeViewBuilder`] by addind
    /// directories or leaves to the tree.
    pub fn show_state(
        mut self,
        ui: &mut Ui,
        state: &mut TreeViewState<NodeIdType>,
        build_tree_view: impl FnMut(&mut TreeViewBuilder<'_, NodeIdType>),
    ) -> (Response, Vec<Action<NodeIdType>>)
    where
        NodeIdType: NodeId + Send + Sync + 'static,
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

        state.prepare(self.settings.allow_multi_select);

        let background_shapes = BackgroundShapes::new(ui, state);

        let InnerResponse {
            inner: tree_builder_result,
            response,
        } = self.draw_foreground(ui, state, build_tree_view);

        state.node_states = tree_builder_result.new_node_states.clone();
        self.handle_fallback_context_menu(&tree_builder_result, state);
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
                        source: simplify_selection_for_dnd(state, &drag_state.node_ids),
                        target: drop_id,
                        position,
                        drop_marker_idx: background_shapes.drop_marker_idx,
                    }))
                } else {
                    actions.push(Action::Drag(DragAndDrop {
                        source: simplify_selection_for_dnd(state, &drag_state.node_ids),
                        target: drop_id,
                        position,
                        drop_marker_idx: background_shapes.drop_marker_idx,
                    }))
                }
            }
        }
        // Create a selection action.
        if input_result.selection_changed {
            actions.push(Action::SetSelected(state.selected().clone()));
        }

        // Reset the drag state.
        if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            state.dragged = None;
        }

        (tree_builder_result.interaction, actions)
    }

    fn draw_foreground(
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
                ui.memory(|m| m.has_focus(self.id)) || state.context_menu_was_open,
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

    fn handle_fallback_context_menu(
        &mut self,
        tree_view_result: &TreeViewBuilderResult<NodeIdType>,
        state: &mut TreeViewState<NodeIdType>,
    ) {
        // Transfer the secondary click
        if tree_view_result.seconday_click.is_some() {
            state.secondary_selection = tree_view_result.seconday_click.clone();
        }

        if !tree_view_result.context_menu_was_open {
            if let Some(fallback_context_menu) = &mut self.fallback_context_menu {
                tree_view_result.interaction.context_menu(|ui| {
                    fallback_context_menu(ui, state.selected());
                });
            }
        }

        state.context_menu_was_open = tree_view_result.interaction.context_menu_opened();
    }

    fn handle_input(
        &mut self,
        ui: &mut Ui,
        tree_view_result: &TreeViewBuilderResult<NodeIdType>,
        state: &mut TreeViewState<NodeIdType>,
    ) -> InputResult<NodeIdType> {
        let TreeViewBuilderResult {
            row_rectangles,
            interaction,
            ..
        } = tree_view_result;

        if interaction.clicked() || interaction.drag_started() {
            ui.memory_mut(|m| m.request_focus(self.id));
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
                if interaction.clicked_by(egui::PointerButton::Primary) {
                    // React to primary clicking
                    selection_changed = true;
                    state.handle_click(
                        node_id,
                        ui.ctx().input(|i| i.modifiers),
                        self.settings.allow_multi_select,
                    );
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
                    let node_ids = if state.is_selected(&node_id) {
                        state.selected().clone()
                    } else {
                        vec![node_id]
                    };
                    state.dragged = Some(DragState {
                        node_ids,
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
                drag_state
                    .node_ids
                    .iter()
                    .for_each(|id| _ = invalid_drop_targets.insert(*id));
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
            if state.selected().is_empty() {
                // todo: fix this
                state.set_selected(
                    state
                        .dragged
                        .as_ref()
                        .map(|drag_state| drag_state.node_ids.clone())
                        .or(state.node_states.first().map(|n| vec![n.id]))
                        .unwrap(),
                );
                selection_changed = true;
            }
            ui.input(|i| {
                for event in i.events.iter() {
                    match event {
                        Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } => {
                            state.handle_key(key, modifiers, self.settings.allow_multi_select);
                            selection_changed = true;
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

    fn draw_background(
        &self,
        ui: &mut Ui,
        state: &TreeViewState<NodeIdType>,
        result: &TreeViewBuilderResult<NodeIdType>,
        background: &BackgroundShapes,
        drop_position: Option<(NodeIdType, DirPosition<NodeIdType>)>,
    ) {
        let has_focus = ui.memory(|m| m.has_focus(self.id)) || state.context_menu_was_open;

        pub const DROP_LINE_HEIGHT: f32 = 3.0;
        if let Some((parent_id, drop_position)) = drop_position {
            let drop_marker = match drop_position {
                DirPosition::Before(target_id) => {
                    let row_rectangles = result.row_rectangles.get(&target_id).unwrap();
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.min.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DirPosition::After(target_id) => {
                    let row_rectangles = result.row_rectangles.get(&target_id).unwrap();
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.max.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DirPosition::First => {
                    let row_rectangles = result.row_rectangles.get(&parent_id).unwrap();
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.max.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DirPosition::Last => {
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

        if !state.selected().is_empty() {
            let mut selected_rects = state
                .selected()
                .iter()
                .map(|id| result.row_rectangles.get(id).unwrap().row_rect)
                .collect::<Vec<_>>();
            selected_rects.sort_by(|a, b| {
                if a.min.y > b.min.y {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            });

            let mut combined_rects = Vec::new();
            let mut current_rect = selected_rects[0];
            for rect in selected_rects.iter().skip(1) {
                if (rect.min.y - current_rect.max.y).abs() < 1.0 {
                    current_rect = Rect::from_min_max(current_rect.min, rect.max)
                } else {
                    combined_rects.push(current_rect);
                    current_rect = *rect;
                }
            }
            combined_rects.push(current_rect);

            for (rect, shape_idx) in combined_rects.iter().zip(&background.background_idx) {
                ui.painter().set(
                    *shape_idx,
                    epaint::RectShape::new(
                        *rect,
                        ui.visuals().widgets.active.corner_radius,
                        if has_focus {
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
        }

        if state.context_menu_was_open {
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

        if has_focus {
            if let Some(selection_cursor_id) = state.selection_cursor() {
                let row_rectangles = result.row_rectangles.get(&selection_cursor_id).unwrap();
                ui.painter().set(
                    background.selection_cursor_idx,
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

        if state.drag_valid() {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let drag_state = state.dragged.as_ref().unwrap();
                let delta = pointer_pos.to_vec2() - drag_state.drag_start_pos.to_vec2();
                let transform = emath::TSTransform::from_translation(delta);
                ui.ctx()
                    .transform_layer_shapes(result.drag_layer, transform);
            }
        }
    }
}
fn simplify_selection_for_dnd<NodeIdType: NodeId>(
    state: &TreeViewState<NodeIdType>,
    nodes: &Vec<NodeIdType>,
) -> Vec<NodeIdType> {
    // When multiple nodes are selected it is possible that a folder is selected aswell as a
    // leaf inside that folder. In that case, a drag and drop action should only include the folder and not the leaf.
    let mut result = Vec::new();
    let mut known_nodes = HashSet::new();
    for node in &state.node_states {
        if !nodes.contains(&node.id) {
            continue;
        }

        let is_unknown_node = node
            .parent_id
            .as_ref()
            .map_or(true, |parent_id| !known_nodes.contains(parent_id));
        if is_unknown_node {
            result.push(node.id);
        }
        known_nodes.insert(node.id);
    }

    result
}

fn get_drop_position_node<NodeIdType: NodeId>(
    node: &NodeState<NodeIdType>,
    drop_quater: &DropQuarter,
) -> Option<(NodeIdType, DirPosition<NodeIdType>)> {
    match drop_quater {
        DropQuarter::Top => {
            if let Some(parent_id) = node.parent_id {
                return Some((parent_id, DirPosition::Before(node.id)));
            }
            if node.drop_allowed {
                return Some((node.id, DirPosition::Last));
            }
            None
        }
        DropQuarter::MiddleTop => {
            if node.drop_allowed {
                return Some((node.id, DirPosition::Last));
            }
            if let Some(parent_id) = node.parent_id {
                return Some((parent_id, DirPosition::Before(node.id)));
            }
            None
        }
        DropQuarter::MiddleBottom => {
            if node.drop_allowed {
                return Some((node.id, DirPosition::Last));
            }
            if let Some(parent_id) = node.parent_id {
                return Some((parent_id, DirPosition::After(node.id)));
            }
            None
        }
        DropQuarter::Bottom => {
            if node.drop_allowed && node.open {
                return Some((node.id, DirPosition::First));
            }
            if let Some(parent_id) = node.parent_id {
                return Some((parent_id, DirPosition::After(node.id)));
            }
            if node.drop_allowed {
                return Some((node.id, DirPosition::Last));
            }
            None
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
pub struct TreeViewSettings {
    /// Override the indent value for the tree view.
    ///
    /// By default this value is 'None' which means that the indent value from the
    /// current ui is used. If this value is set, this value will used as the indent
    /// value without affecting the ui's indent value.
    pub override_indent: Option<f32>,
    /// The style of the indent hint to show the indentation level.
    pub indent_hint_style: IndentHintStyle,
    /// The row layout for this tree.
    pub row_layout: RowLayout,
    /// The maximum width the tree can have.
    ///
    /// If the tree is part of a horizontally justified layout, this property has no effect and the tree will always fill the available horizontal space.
    pub max_width: f32,
    /// The maximum hight the tree can have.
    ///
    /// If the tree is part of a vertical justified layout, this property has no effect and the tree will always fill the available vertical space.
    pub max_height: f32,
    /// The minimum width the tree can have.
    pub min_width: f32,
    /// The minimum hight the tree can have.
    pub min_height: f32,
    /// Whether or not the tree should fill all available horizontal space.
    ///
    /// If the tree is part of a horizontally justified layout, this property has no effect and the tree will always fill horizontal space.
    /// Default is true.
    pub fill_space_horizontal: bool,
    /// Whether or not the tree should fill all available vertical space.
    ///
    /// If the tree is part of a vertically justified layout, this property has no effect and the tree will always fill vertical space.
    /// Default is false.
    pub fill_space_vertical: bool,
    /// If the tree view is allowed to select multiple nodes at once.
    /// Default is true.
    pub allow_multi_select: bool,
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
            allow_multi_select: true,
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
    /// The nodes that are beeing dragged
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

/// Interact with the ui without egui adding any extra space.
fn interact_no_expansion(ui: &mut Ui, rect: Rect, id: Id, sense: Sense) -> Response {
    let spacing_before = ui.spacing().clone();
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let res = ui.interact(rect, id, sense);
    *ui.spacing_mut() = spacing_before;
    res
}

struct BackgroundShapes {
    background_idx: Vec<ShapeIdx>,
    secondary_selection_idx: ShapeIdx,
    selection_cursor_idx: ShapeIdx,
    drop_marker_idx: ShapeIdx,
}
impl BackgroundShapes {
    fn new<NodeIdType: NodeId>(ui: &mut Ui, state: &TreeViewState<NodeIdType>) -> Self {
        Self {
            background_idx: (0..(state.selected().len() + 1))
                .map(|_| ui.painter().add(Shape::Noop))
                .collect(),
            secondary_selection_idx: ui.painter().add(Shape::Noop),
            selection_cursor_idx: ui.painter().add(Shape::Noop),
            drop_marker_idx: ui.painter().add(Shape::Noop),
        }
    }
}

struct InputResult<NodeIdType> {
    drag_and_drop: Option<(NodeIdType, DirPosition<NodeIdType>)>,
    selection_changed: bool,
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
        let h2 = (range.min + range.max) / 2.0;
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
