#![warn(missing_docs)]
//! # `egui_ltreeview` is a tree view widget for [egui](https://github.com/emilk/egui)
//!
//! This tree view widget implements all the common features of a tree view to get you
//! up and running as fast as possible.
//!
//! ### Features:
//! * Directory and leaf nodes
//! * Node selection
//! * Select multiple nodes
//! * Keyboard navigation using arrow keys
//! * Frontend for Drag and Drop support
//! * Agnostic to the implementation of your data.
//!
//! # Crate feature flags
//! * `persistence` Adds serde to [`NodeId`] and enabled the `persistence` feature of egui.
//!
//! # Getting started
//! ```
//! let id = ui.make_persistent_id("Names tree view");
//! TreeView::new(id).show(ui, |builder| {
//!     builder.dir(0, "Root");
//!     builder.leaf(1, "Ava");
//!     builder.leaf(2, "Benjamin");
//!     builder.leaf(3, "Charlotte");
//!     builder.close_dir();
//! });
//! ```
//! Create a new [`TreeView`] with its unique id and show it for the current ui.
//! Use the [`builder`](TreeViewBuilder) in the callback to add directories and leaves
//! to the tree. The nodes of the tree must have a unique id which implements the [`NodeId`] trait.
//!
//! # Customizing the tree view
//! To change the basic settings of the tree view you can use the [`TreeViewSettings`] to customize the tree view
//! or use the convenience methods on [`TreeView`] directly.
//!
//! Check out [`TreeViewSettings`] for all settings possible on the tree view.
//!
//! ```
//! TreeView::new(id)
//!     .with_settings(TreeViewSettings{
//!         override_indent: Some(15),
//!         fill_space_horizontal: true,
//!         fill_space_vertical: true,
//!         ..Default::default()
//!     })
//!     .max_height(200)
//!     .show(ui, |builder| {
//!     ...
//! });
//! ```
//!
//! # Customizing nodes, directories and leaves
//! To customize nodes, directories, and leaves you can use the [`NodeBuilder`] before adding the node
//! to the [`builder`](TreeViewBuilder).
//! Here you can add an icon to the node that is shown in-front of the label, for directories you can also
//! show a custom closer. It is also possible to configure the context menu for this node specifically. More
//! about context menus in the context menu section.
//!
//! Look at [`NodeBuilder`] for all configuration options of a node.
//! ```
//! TreeView::new(id).show(ui, |builder| {
//!     builder.node(NodeBuilder::dir(0)
//!         .default_open(false)
//!         .label("Root")
//!         .icon(|ui| {
//!             egui::Image::new(egui::include_image!("settings.png"))
//!                 .tint(ui.visuals().widgets.noninteractive.fg_stroke.color)
//!                 .paint_at(ui, ui.max_rect());
//!         }));
//!     // other leaves or directories
//!     builder.close_dir(); // dont forget to close the root directory at the end.
//! });
//! ```
//! # Multi select
//! The tree view supports selecting multiple nodes at once. This behavior was modeled after the
//! Windows file explorer and supports all the common keyboard navigation behaviors.
//!
//! Clicking on a node selects this node. Shift clicking will select all nodes between the previously selected
//! node (the pivot node) and the newly clicked node. Control clicking (command click on Mac) will add the
//! clicked node to the selection or remove it from the selection if it was already selected.
//!
//! You can use the arrow keys to move the selection through the tree. If you hold either shift or control(command on Mac)
//! while navigating with the arrow keys you will move a cursor through the tree instead. How nodes are selected in this
//! mode depends on the configuration of shift and control being held down.
//! * **shift only** this will select all nodes between the pivot node and the cursor.
//! * **control only** Only moves the cursor. Pressing space will either select or deselect the current node underneath the cursor
//! * **shift and control** Every node the cursor reaches is added to the selection.
//!
//! You can disable multi selection by setting [`allow_multi_select`](TreeView::allow_multi_selection) to
//! false in the [`TreeView`] or the [`TreeViewSettings`].
//!
//! # Context menus
//! You can add a context menu to a node by specifying it in the [`NodeBuilder`].
//! ```
//! treebuilder.node(NodeBuilder::leaf(0)
//!     .context_menu(|ui|{
//!         ui.label("i am the context menu for this node")
//!     }));
//! ```
//! If a node was right-clicked but did not configure a context menu then the [`fallback context menu`](TreeView::fallback_context_menu)
//! will be used.
//!
//! The [`fallback context menu`](TreeView::fallback_context_menu) in the [`TreeView`] also serves as the context menu
//! for right-clicking on multiple nodes in a multi selection. Here the list of all nodes that belong to this context menu is passed in
//!
//! ```
//! TreeView::new(id)
//!     .fallback_context_menu(|ui, nodes| {
//!         for node in nodes{
//!             ui.label(format!("selected node: {}", node));
//!         }
//!     })
//!     .show(ui, |builder| {
//!         builder.dir(0, "Root");
//!         builder.leaf(1, "Ava");
//!         builder.leaf(2, "Benjamin");
//!         builder.leaf(3, "Charlotte");
//!         builder.close_dir();
//!     });
//! ```
//!
//!
//! **A side node about sizing of the context menu:**
//! All nodes and the fallback share the same context menu. In egui, the size of a context menu
//! is determined the first time the context menu becomes visible. For this reason, you might have
//! to set the size of the context menu manually with `ui.set_width` if you plan on having multiple
//! differently sized context menus in your tree.
//!
//! # Actions generated by the tree view
//! Since the tree view is agnostic to the implementation of the data there are some interactions
//! that the tree view would like to acomplish but it cannot do them on its own.
//! The tree view will instead create a list of [`Action`]s as part of its reponse. It is up to the
//! user to implement these actions correctly for the interaction to work correctly.
//!
//! Actions generated by the tree view are:
//! * [`Drag`](`Action::Drag`) and [`Move`](`Action::Move`) actions to create drag and drop interactions.
//! * [`Activate`](`Action::Activate`) to activate a set of nodes. This action can be used to open files
//! by double clicking for example.
//! * [`SetSelected`](`Action::SetSelected`) action to change the selected nodes of the tree.
//!
//! ### Drag and Move actions
//! The [`Drag`](`Action::Drag`) and [`Move`](`Action::Move`) can be used to implement a drag and drop
//! interaction of the tree.
//! A drag that has not yet been dropped is represented by the [`Drag`](`Action::Drag`) action. With
//! this action you can test if the drag is valid or not. If the drag invalid you may decide to remove the drop
//! marker from the tree by calling the [`DragAndDrop::remove_drop_marker`] method of the action.
//!
//! The [`Move`](`Action::Move`) is used to indicated that the node has been dropped to a new position in the tree
//! and it now needs to be moved in the underlying data strucuter.
//!
//! A node can control if it wants to be a valid target of a `Drag` or `Move` action by setting
//! its [`drop_allowed`](NodeBuilder::drop_allowed) property.
//!
//! ### Activating nodes
//! The tree view will generate the [`Activate`](`Action::Activate`) action either when the enter key is pressed
//! or when a node is double clicked and will contain all activatable nodes from the currently selected nodes.
//!
//! What exactly "activate" means is up to the implementation of the action. If for example your tree view
//! contains files you can use the activate action to open these files in a new window or tab.
//!
//! By default only leaf nodes are activatable. You can override this by setting the [`activatable`](`NodeBuilder::activatable`)
//! property of a node.

mod builder;
mod builder_state;
mod node;
mod node_states;
mod state;

use egui::{
    self, emath, epaint, layers::ShapeIdx, vec2, Event, EventFilter, Id, Key, LayerId, Layout,
    Modifiers, NumExt, Order, Rangef, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};
use std::{cmp::Ordering, collections::HashMap, hash::Hash};

pub use builder::*;
pub use node::*;
pub use state::*;

/// A node in the tree is identified by an id that must implement this trait.
///
/// This is just a trait alias for the collection of necessary traits that a node id
/// must implement.
#[cfg(not(feature = "persistence"))]
pub trait NodeId: Clone + PartialEq + Eq + Hash + std::fmt::Debug {}
#[cfg(not(feature = "persistence"))]
impl<T> NodeId for T where T: Clone + PartialEq + Eq + Hash + std::fmt::Debug {}

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
pub struct TreeView<NodeIdType> {
    id: Id,
    settings: TreeViewSettings,
    #[allow(clippy::type_complexity)]
    fallback_context_menu: Option<Box<dyn FnOnce(&mut Ui, &Vec<NodeIdType>)>>,
}
impl<NodeIdType: NodeId> TreeView<NodeIdType> {
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
    /// By default, this value is 'None' which means that the indent value from the
    /// current ui is used. If this value is set, this value will be used as the indent
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

    /// Controls whether the tree should fill all available horizontal space.
    ///
    /// If the tree is part of a horizontally justified layout, this property has no
    /// effect and the tree will always fill horizontal space.
    ///
    /// Default is `true`.
    pub fn fill_space_horizontal(mut self, fill_space_horizontal: bool) -> Self {
        self.settings.fill_space_horizontal = fill_space_horizontal;
        self
    }

    /// Controls whether the tree should fill all available vertical space.
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

    /// Set the maximum height the tree can have.
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

    /// Set the minimum height the tree can have.
    pub fn min_height(mut self, height: f32) -> Self {
        self.settings.min_height = height;
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
    /// If the node did not configure a context menu directly or
    /// if multiple nodes were selected and right-clicked, then
    /// this fallback context menu will be opened.
    ///
    /// A context menu in egui gets its size the first time it becomes visible.
    /// Since all nodes in the tree view share the same context menu you must set
    /// the size of the context menu manually for each node if you want to have differently
    /// sized context menus.
    pub fn fallback_context_menu(
        mut self,
        context_menu: impl FnOnce(&mut Ui, &Vec<NodeIdType>) + 'static,
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

        if !settings.allow_multi_select {
            state.prune_selection_to_single_id();
        }

        let (ui_data, response) = draw_foreground(
            ui,
            id,
            &settings,
            state,
            build_tree_view,
            &mut fallback_context_menu,
        );
        // Remember the size of the tree for next frame.
        state.size = response.rect.size();
        state.prune_selection_to_known_ids();

        draw_background(ui, state, &ui_data);

        let input_result = handle_input(ui, id, &settings, &ui_data, state);

        let mut actions = Vec::new();
        // Create a drag or move action.
        if state.drag_valid() {
            let drag_and_drop_position = get_drop_position(&ui_data, state);
            draw_drop(ui, &ui_data, state, drag_and_drop_position.as_ref());

            if let Some((drag_state, (drop_id, position))) =
                state.dragged.as_ref().zip(drag_and_drop_position)
            {
                if ui.ctx().input(|i| i.pointer.primary_released()) {
                    actions.push(Action::Move(DragAndDrop {
                        source: simplify_selection_for_dnd(state, &drag_state.node_ids),
                        target: drop_id,
                        position,
                        drop_marker_idx: ui_data.drop_marker_idx,
                    }))
                } else {
                    actions.push(Action::Drag(DragAndDrop {
                        source: simplify_selection_for_dnd(state, &drag_state.node_ids),
                        target: drop_id,
                        position,
                        drop_marker_idx: ui_data.drop_marker_idx,
                    }))
                }
            } else {
                if let Some(cursor_pos) = ui.ctx().pointer_latest_pos() {
                    if !response.rect.contains(cursor_pos) {
                        if let Some(dragged) = &state.dragged {
                            if ui.ctx().input(|i| i.pointer.primary_released()) {
                                actions.push(Action::MoveExternal(DragAndDropExternal {
                                    position: cursor_pos,
                                    source: dragged.node_ids.clone(),
                                }));
                            } else {
                                actions.push(Action::DragExternal(DragAndDropExternal {
                                    position: cursor_pos,
                                    source: dragged.node_ids.clone(),
                                }));
                            }
                        }
                    }
                }
            }
        }
        // Create a selection action.
        if input_result.selection_changed {
            actions.push(Action::SetSelected(state.selected().clone()));
        }

        if input_result.should_activate {
            actions.push(Action::Activate(Activate {
                selected: state
                    .selected()
                    .iter()
                    .filter(|node_id| {
                        state
                            .node_state_of(node_id)
                            .is_some_and(|ns| ns.activatable)
                    })
                    .cloned()
                    .collect::<Vec<_>>(),
                modifiers: ui.ctx().input(|i| i.modifiers),
            }));
        }

        // Reset the drag state.
        if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            state.dragged = None;
        }

        (ui_data.interaction, actions)
    }
}

fn draw_foreground<'context_menu, NodeIdType: NodeId>(
    ui: &mut Ui,
    id: Id,
    settings: &TreeViewSettings,
    state: &mut TreeViewState<NodeIdType>,
    build_tree_view: impl FnOnce(&mut TreeViewBuilder<'_, NodeIdType>),
    fall_back_context_menu: &mut Option<Box<dyn FnOnce(&mut Ui, &Vec<NodeIdType>)>>,
) -> (UiData<NodeIdType>, Response) {
    // Calculate the desired size of the tree view widget.
    let size = vec2(
        if settings.fill_space_horizontal || ui.layout().horizontal_justify() {
            ui.available_width()
        } else {
            state.size.x.at_most(settings.max_width)
        }
        .at_least(settings.min_width),
        if settings.fill_space_vertical || ui.layout().vertical_justify() {
            ui.available_height()
        } else {
            state.size.y.at_most(settings.max_height)
        }
        .at_least(settings.min_height),
    );

    let mut ui_data = UiData {
        row_rectangles: HashMap::new(),
        seconday_click: None,
        interaction: interact_no_expansion(
            ui,
            Rect::from_min_size(ui.cursor().min, size),
            id,
            Sense::click_and_drag(),
        ),
        context_menu_was_open: false,
        drag_layer: LayerId::new(Order::Tooltip, ui.make_persistent_id("ltreeviw drag layer")),
        has_focus: ui.memory(|m| m.has_focus(id)) || state.context_menu_was_open,
        background_idx: (0..(state.selected().len() + 1))
            .map(|_| ui.painter().add(Shape::Noop))
            .collect(),
        secondary_selection_idx: ui.painter().add(Shape::Noop),
        selection_cursor_idx: ui.painter().add(Shape::Noop),
        drop_marker_idx: ui.painter().add(Shape::Noop),
    };

    // Run the build tree view closure
    let response = ui
        .allocate_ui_with_layout(size, Layout::top_down(egui::Align::Min), |ui| {
            ui.set_min_size(vec2(settings.min_width, settings.min_height));
            ui.add_space(ui.spacing().item_spacing.y * 0.5);

            let mut tree_builder = TreeViewBuilder::new(ui, state, settings, &mut ui_data);
            build_tree_view(&mut tree_builder);

            // Add negative space because the place will add the item spacing on top of this.
            ui.add_space(-ui.spacing().item_spacing.y * 0.5);

            if settings.fill_space_horizontal {
                ui.set_min_width(ui.available_width());
            }
            if settings.fill_space_vertical {
                ui.set_min_height(ui.available_height());
            }
        })
        .response;

    // Transfer the secondary click
    if ui_data.seconday_click.is_some() {
        state.secondary_selection = ui_data.seconday_click.clone();
    }
    // Do context menu
    if !ui_data.context_menu_was_open {
        if let Some(fallback_context_menu) = fall_back_context_menu.take() {
            ui_data.interaction.context_menu(|ui| {
                fallback_context_menu(ui, state.selected());
            });
        }
    }

    state.context_menu_was_open = ui_data.interaction.context_menu_opened();

    (ui_data, response)
}

fn handle_input<NodeIdType: NodeId>(
    ui: &mut Ui,
    id: Id,
    settings: &TreeViewSettings,
    tree_view_result: &UiData<NodeIdType>,
    state: &mut TreeViewState<NodeIdType>,
) -> InputResult {
    let UiData {
        row_rectangles,
        interaction,
        ..
    } = tree_view_result;

    if interaction.clicked() || interaction.drag_started() {
        ui.memory_mut(|m| m.request_focus(id));
    }

    let mut selection_changed = false;
    let mut should_activate = false;

    for (node_id, row) in row_rectangles {
        let RowRectangles {
            row_rect,
            closer_rect,
        } = row;

        // Closer interactions
        let closer_clicked = closer_rect
            .zip(interaction.hover_pos())
            .is_some_and(|(closer_rect, hover_pos)| closer_rect.contains(hover_pos))
            && interaction.clicked();
        if closer_clicked {
            let node_state = state.node_state_of_mut(&node_id).unwrap();
            node_state.open = !node_state.open;
        }

        // Row interaction
        let cursor_above_row = interaction
            .hover_pos()
            .is_some_and(|pos| row_rect.contains(pos));
        if cursor_above_row && !closer_clicked {
            let was_last_clicked = state
                .last_clicked_node
                .as_ref()
                .is_some_and(|last| last == node_id);

            if interaction.double_clicked() && was_last_clicked {
                let node_state = state.node_state_of_mut(&node_id).unwrap();
                // directories should only switch their opened state by double clicking if no modifiers
                // are pressed. If any modifier is pressed then the closer should be used.
                if node_state.dir && ui.ctx().input(|i| i.modifiers).is_none() {
                    node_state.open = !node_state.open;
                }

                if node_state.activatable {
                    // This has the potential to clash with the previous action.
                    // If a directory is activatable then double clicking it will toggle its
                    // open state and activate the directory. Usually we would want one input
                    // to have one effect but in this case it is impossible for us to know if the
                    // user wants to activate the directory or toggle it.
                    // We could add a configuration option to choose either toggle or activate
                    // but in this case i think that doing both has the biggest chance to achieve
                    // what the user wanted.
                    should_activate = true;
                }
            } else if interaction.clicked_by(egui::PointerButton::Primary) {
                // must be handled after double-clicking to prevent the second click of the double-click
                // performing 'click' actions.

                // React to primary clicking
                selection_changed = true;
                state.handle_click(
                    node_id.clone(),
                    ui.ctx().input(|i| i.modifiers),
                    settings.allow_multi_select,
                );
            }

            if interaction.clicked() {
                state.last_clicked_node = Some(node_id.clone());
            }

            // React to a dragging
            // An egui drag only starts after the pointer has moved but with that first movement
            // the pointer may have moved to a different node. Instead we want to update
            // the drag state right when the priamry button was pressed.
            // We also want to have our own rules when a drag really becomes valid to avoid
            // graphical artifacts. Sometimes the user is a little fast with the mouse and
            // it creates the drag overlay when it really shouldn't have.
            let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
            if primary_pressed && settings.allow_drag_and_drop {
                let pointer_pos = ui.ctx().pointer_latest_pos().unwrap_or_default();
                let node_ids = if state.is_selected(&node_id) {
                    state.selected().clone()
                } else {
                    vec![node_id.clone()]
                };
                state.dragged = Some(DragState {
                    node_ids,
                    drag_start_pos: pointer_pos,
                    drag_valid: false,
                });
            }
        }
    }

    if ui.memory(|m| m.has_focus(id)) {
        // If the widget is focused but no node is selected we want to select any node
        // to allow navigating throught the tree.
        // In case we gain focus from a drag action we select the dragged node directly.
        if state.selected().is_empty() {
            let fallback_selection = state
                .dragged
                .as_ref()
                .map(|drag_state| drag_state.node_ids.clone())
                .or(state.node_states().first().map(|(id, _)| vec![id.clone()]));
            if let Some(fallback_selection) = fallback_selection {
                state.set_selected(fallback_selection);
                selection_changed = true;
            }
        }
        ui.input(|i| {
            for event in i.events.iter() {
                match event {
                    Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        ..
                    } => {
                        should_activate = true;
                    }
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        state.handle_key(key, modifiers, settings.allow_multi_select);
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
        selection_changed,
        should_activate,
    }
}

fn get_drop_position<NodeIdType: NodeId>(
    tree_view_result: &UiData<NodeIdType>,
    state: &mut TreeViewState<NodeIdType>,
) -> Option<(NodeIdType, DirPosition<NodeIdType>)> {
    if !state.drag_valid() {
        return None;
    }
    let drag_state = state.dragged.as_ref()?;
    let hover_pos = tree_view_result.interaction.hover_pos()?;
    let (hovered_id, drop_quarter) = tree_view_result
        .row_rectangles
        .iter()
        .filter_map(|(id, row)| {
            let drop_quarter = DropQuarter::new(row.row_rect.y_range(), hover_pos.y)?;
            Some((id, drop_quarter))
        })
        .next()?;

    // At this point we know that the drag is valid and the cursor
    // is hovering above a row in the tree.

    // Dropping a node on itself is technically a fine thing to do
    // but it causes all sorts of problems for the implementer of the drop action.
    // They would have to remove a node and then somehow insert it after itself.
    // For that reason it is easier to disallow dropping on itself altogether.
    if drag_state.node_ids.contains(hovered_id) {
        return None;
    }

    // If the hovered node is a child of one of the dragged node then this
    // is not a valid drop target

    let is_hovering_on_child_of_dragged_node = drag_state
        .node_ids
        .iter()
        .any(|id| state.node_states().is_child_of(hovered_id, id));
    if is_hovering_on_child_of_dragged_node {
        return None;
    }

    let hovered_node = state
        .node_state_of(hovered_id)
        .expect("If the node exists in the row rectangles it should also exist in the node states");
    return get_drop_position_node(hovered_node, &drop_quarter);
}

fn draw_drop<NodeIdType: NodeId>(
    ui: &mut Ui,
    ui_data: &UiData<NodeIdType>,
    state: &TreeViewState<NodeIdType>,
    drop_position: Option<&(NodeIdType, DirPosition<NodeIdType>)>,
) {
    pub const DROP_LINE_HEIGHT: f32 = 3.0;
    if let Some((parent_id, drop_position)) = drop_position {
        let drop_marker =
            match drop_position {
                DirPosition::Before(target_id) => {
                    let row_rectangles = ui_data.row_rectangles.get(&target_id).expect(
                        "Drop target must have a rectangle or it could not be a drop target",
                    );
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.min.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DirPosition::After(target_id) => {
                    let row_rectangles = ui_data.row_rectangles.get(&target_id).expect(
                        "Drop target must have a rectangle or it could not be a drop target",
                    );
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.max.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DirPosition::First => {
                    let row_rectangles = ui_data.row_rectangles.get(&parent_id).expect(
                        "Drop target must have a rectangle or it could not be a drop target",
                    );
                    Rect::from_x_y_ranges(
                        row_rectangles.row_rect.x_range(),
                        Rangef::point(row_rectangles.row_rect.max.y).expand(DROP_LINE_HEIGHT * 0.5),
                    )
                }
                DirPosition::Last => {
                    let row_rectangles_start = ui_data.row_rectangles.get(&parent_id).expect(
                        "Drop target must have a rectangle or it could not be a drop target",
                    );
                    // For directories the drop marker should expand its height to include all
                    // its child nodes. To do this, first we have to find its last child node,
                    // then we can get the correct y range.
                    let last_visible_child_row = ui_data
                        .row_rectangles
                        .iter()
                        .filter(|(id, _row)| state.node_states().is_child_of(&id, &parent_id))
                        .map(|(_, value)| value)
                        .max_by_key(|row| row.row_rect.max.y as i32);

                    let y_range = match last_visible_child_row {
                        Some(last_visible_child_row) => {
                            let y = last_visible_child_row.row_rect.max.y;
                            Rangef::new(row_rectangles_start.row_rect.min.y, y)
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
        ui.painter().set(ui_data.drop_marker_idx, shape);
    }
}

fn draw_background<NodeIdType: NodeId>(
    ui: &mut Ui,
    state: &TreeViewState<NodeIdType>,
    ui_data: &UiData<NodeIdType>,
) {
    if !state.selected().is_empty() {
        let mut selected_rects = state
            .selected()
            .iter()
            .filter_map(|id| ui_data.row_rectangles.get(id).map(|r| r.row_rect))
            .collect::<Vec<_>>();
        if !selected_rects.is_empty() {
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

            for (rect, shape_idx) in combined_rects.iter().zip(&ui_data.background_idx) {
                ui.painter().set(
                    *shape_idx,
                    epaint::RectShape::new(
                        *rect,
                        ui.visuals().widgets.active.corner_radius,
                        if ui_data.has_focus {
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
    }

    if state.context_menu_was_open {
        if let Some(row_rectangles) = state
            .secondary_selection
            .as_ref()
            .and_then(|id| ui_data.row_rectangles.get(&id))
        {
            ui.painter().set(
                ui_data.secondary_selection_idx,
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

    if ui_data.has_focus {
        if let Some(row_rectangles) = state
            .selection_cursor()
            .and_then(|id| ui_data.row_rectangles.get(&id))
        {
            ui.painter().set(
                ui_data.selection_cursor_idx,
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

    if let Some(drag_state) = &state.dragged {
        if drag_state.drag_valid {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos.to_vec2() - drag_state.drag_start_pos.to_vec2();
                let transform = emath::TSTransform::from_translation(delta);
                ui.ctx()
                    .transform_layer_shapes(ui_data.drag_layer, transform);
            }
        }
    }
}

fn simplify_selection_for_dnd<NodeIdType: NodeId>(
    state: &TreeViewState<NodeIdType>,
    nodes: &[NodeIdType],
) -> Vec<NodeIdType> {
    // When multiple nodes are selected it is possible that a folder is selected aswell as a
    // leaf inside that folder. In that case, a drag and drop action should only include the folder and not the leaf.
    let mut result = Vec::new();
    let mut node_states = nodes
        .into_iter()
        .filter_map(|id| state.node_states().get(id))
        .collect::<Vec<_>>();
    node_states.sort_by_key(|ns| ns.position);

    'i: for i in 0..node_states.len() {
        for j in 0..i {
            let i_is_child_of_j = state
                .node_states()
                .is_child_of(&node_states[i].id, &node_states[j].id);
            if i_is_child_of_j {
                continue 'i;
            }
        }
        // is is not a child of any of 0..j
        result.push(node_states[i].id.clone());
    }
    result
}

fn get_drop_position_node<NodeIdType: NodeId>(
    node: &NodeState<NodeIdType>,
    drop_quater: &DropQuarter,
) -> Option<(NodeIdType, DirPosition<NodeIdType>)> {
    match drop_quater {
        DropQuarter::Top => {
            if let Some(parent_id) = node.parent_id.as_ref() {
                return Some((parent_id.clone(), DirPosition::Before(node.id.clone())));
            }
            if node.drop_allowed {
                return Some((node.id.clone(), DirPosition::Last));
            }
            None
        }
        DropQuarter::MiddleTop => {
            if node.drop_allowed {
                return Some((node.id.clone(), DirPosition::Last));
            }
            if let Some(parent_id) = node.parent_id.as_ref() {
                return Some((parent_id.clone(), DirPosition::Before(node.id.clone())));
            }
            None
        }
        DropQuarter::MiddleBottom => {
            if node.drop_allowed {
                return Some((node.id.clone(), DirPosition::Last));
            }
            if let Some(parent_id) = node.parent_id.as_ref() {
                return Some((parent_id.clone(), DirPosition::After(node.id.clone())));
            }
            None
        }
        DropQuarter::Bottom => {
            if node.drop_allowed && node.open {
                return Some((node.id.clone(), DirPosition::First));
            }
            if let Some(parent_id) = node.parent_id.as_ref() {
                return Some((parent_id.clone(), DirPosition::After(node.id.clone())));
            }
            if node.drop_allowed {
                return Some((node.id.clone(), DirPosition::Last));
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
    /// By default, this value is 'None' which means that the indent value from the
    /// current UI is used. If this value is set, this value will be used as the indent
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
    /// The maximum height the tree can have.
    ///
    /// If the tree is part of a vertical justified layout, this property has no effect and the tree will always fill the available vertical space.
    pub max_height: f32,
    /// The minimum width the tree can have.
    pub min_width: f32,
    /// The minimum height the tree can have.
    pub min_height: f32,
    /// Whether the tree should fill all available horizontal space.
    ///
    /// If the tree is part of a horizontally justified layout, this property has no effect and the tree will always fill horizontal space.
    /// Default is true.
    pub fill_space_horizontal: bool,
    /// Whether the tree should fill all available vertical space.
    ///
    /// If the tree is part of a vertically justified layout, this property has no effect and the tree will always fill vertical space.
    /// Default is false.
    pub fill_space_vertical: bool,
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
            indent_hint_style: Default::default(),
            row_layout: Default::default(),
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            min_width: 0.0,
            min_height: 0.0,
            fill_space_horizontal: true,
            fill_space_vertical: false,
            allow_multi_select: true,
            allow_drag_and_drop: true,
            default_node_height: None,
        }
    }
}

/// Style of the vertical line to show the indentation level.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum IndentHintStyle {
    /// No indent hint is shown.
    None,
    /// A single vertical line is show for the full height of the directory.
    Line,
    /// A vertical line is show with horizontal hooks to the child nodes of the directory.
    #[default]
    Hook,
}

/// How rows in the tree are laid out.
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
#[derive(Clone)]
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
#[derive(Clone)]
pub struct DragAndDropExternal<NodeIdType> {
    /// The nodes that are being dragged
    pub source: Vec<NodeIdType>,
    /// The position where the dragged nodes are dropped outside of the TreeView.
    pub position: egui::Pos2,
}

/// Information about drag and drop action that is currently
/// happening on the tree.
#[derive(Clone)]
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
#[derive(Clone)]
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

struct InputResult {
    selection_changed: bool,
    should_activate: bool,
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
    row_rectangles: HashMap<NodeIdType, RowRectangles>,
    seconday_click: Option<NodeIdType>,
    context_menu_was_open: bool,
    interaction: Response,
    drag_layer: LayerId,
    has_focus: bool,
    background_idx: Vec<ShapeIdx>,
    secondary_selection_idx: ShapeIdx,
    selection_cursor_idx: ShapeIdx,
    drop_marker_idx: ShapeIdx,
}

#[derive(Debug)]
struct RowRectangles {
    row_rect: Rect,
    closer_rect: Option<Rect>,
}
