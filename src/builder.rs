use egui::{layers::ShapeIdx, pos2, vec2, Pos2, Rangef, Rect, Shape, Ui, UiBuilder, WidgetText};

use crate::{
    node::NodeBuilder, rect_contains_visually, DirPosition, DragState, DropQuarter,
    IndentHintStyle, Input, Node, NodeConfig, NodeId, Output, TreeViewSettings, TreeViewState,
    UiData,
};

#[derive(Clone)]
struct DirectoryState<NodeIdType> {
    /// Id of the directory node.
    id: NodeIdType,
    /// How many children this directory has.
    /// Used for automatically closing the directory after all its children have been added.
    child_count: Option<usize>,
    /// If current directory branch is expanded or collapsed
    branch_expanded: bool,
    /// Whether or not the current branch is being dragged.
    branch_dragged: bool,
    /// The rectangle at which the dir would be visible.
    row_rect: Option<Rect>,
}
struct IndentState<NodeIdType> {
    /// Id of the node that created this indent
    source_node: NodeIdType,
    /// Anchor for the indent hint at the source directory
    anchor: Rangef,
    /// Positions of child nodes for the indent hint.
    positions: Vec<Pos2>,
    /// How far the hint should be indented.
    indent: usize,
    /// If true, this indent hint extends bellow the bottom edge of the clip rect.
    extends_below_clip_rect: bool,
}

/// The builder used to construct the tree.
///
/// Use this to add directories or leaves to the tree.
pub struct TreeViewBuilder<'ui, NodeIdType: NodeId> {
    ui: &'ui mut Ui,
    state: &'ui mut TreeViewState<NodeIdType>,
    settings: &'ui TreeViewSettings,
    ui_data: &'ui mut UiData<NodeIdType>,
    selection_background: Option<(ShapeIdx, Rect)>,
    stack: Vec<DirectoryState<NodeIdType>>,
    indents: Vec<IndentState<NodeIdType>>,
    input: &'ui mut Input<NodeIdType>,
    output: &'ui mut Output<NodeIdType>,
}

impl<'ui, NodeIdType: NodeId> TreeViewBuilder<'ui, NodeIdType> {
    pub(crate) fn new(
        ui: &'ui mut Ui,
        state: &'ui mut TreeViewState<NodeIdType>,
        settings: &'ui TreeViewSettings,
        ui_data: &'ui mut UiData<NodeIdType>,
        input: &'ui mut Input<NodeIdType>,
        output: &'ui mut Output<NodeIdType>,
    ) -> Self {
        Self {
            ui_data,
            state,
            settings,
            ui,
            selection_background: None,
            stack: Vec::new(),
            indents: Vec::new(),
            input,
            output,
        }
    }

    /// Get the current parent id if any.
    pub fn parent_id(&self) -> Option<&NodeIdType> {
        self.stack.last().map(|dir| &dir.id)
    }

    /// Add a leaf directly to the tree with an id and the label text.
    ///
    /// To customize the node that is added to the tree consider using [`TreeViewBuilder::node`]
    pub fn leaf(&mut self, id: NodeIdType, label: impl Into<WidgetText>) {
        let widget_text = label.into();
        self.node(NodeBuilder::leaf(id).label_ui(|ui| {
            ui.add(egui::Label::new(widget_text.clone()).selectable(false));
        }));
    }

    /// Add a directory to the tree.
    ///
    /// Returns `true` if the directory is open and its child are visible.
    /// Returns `false` if the directory is closed.
    ///
    /// Must call [`TreeViewBuilder::close_dir`] to close the directory.
    ///
    /// To customize the node that is added to the tree consider using [`TreeViewBuilder::node`]
    pub fn dir(&mut self, id: NodeIdType, label: impl Into<WidgetText>) -> bool {
        let widget_text = label.into();
        self.node(NodeBuilder::dir(id).label_ui(|ui| {
            ui.add(egui::Label::new(widget_text.clone()).selectable(false));
        }))
    }

    /// Automatically close the current dir after `child_count` many nodes
    /// have been added to the tree.
    /// If this method is called with `0` the current directory will close immediately.
    /// Child nodes that were added before this method was called are not counted.
    pub fn close_dir_in(&mut self, child_count: usize) {
        if child_count == 0 {
            self.close_dir();
        } else if let Some(dir_state) = self.stack.last_mut() {
            dir_state.child_count = Some(child_count);
        }
    }

    /// Close the current directory.
    pub fn close_dir(&mut self) {
        while let Some(dir_state) = self.stack.pop() {
            let indent = self
                .indents
                .pop_if(|indent| indent.source_node == dir_state.id);
            if let Some(indent) = indent {
                self.draw_indent_hint(&indent);
                match self.ui_data.drop_target.as_ref() {
                    Some((target_id, DirPosition::Last)) if target_id == &dir_state.id => {
                        self.draw_drop_marker(indent.anchor, &DirPosition::Last);
                    }
                    _ => (),
                };
            }
            if !self.should_close_current_dir() {
                break;
            }
        }
    }

    fn draw_indent_hint(&mut self, indent: &IndentState<NodeIdType>) {
        let top = self.ui.clip_rect().clamp(pos2(
            self.ui.cursor().min.x
                + self.ui.spacing().item_spacing.x
                + self.ui.spacing().icon_width * 0.5
                + indent.indent as f32
                    * self
                        .settings
                        .override_indent
                        .unwrap_or(self.ui.spacing().indent),
            indent.anchor.center() + self.ui.spacing().icon_width * 0.5 + 2.0,
        ));
        let bottom = self
            .ui
            .clip_rect()
            .clamp(pos2(top.x, self.ui_data.space_used.bottom()));

        match self.settings.indent_hint_style {
            IndentHintStyle::None => (),
            IndentHintStyle::Line => {
                self.ui.painter().line_segment(
                    [top, bottom],
                    self.ui.visuals().widgets.noninteractive.bg_stroke,
                );
            }
            IndentHintStyle::Hook => {
                let bottom = if indent.extends_below_clip_rect {
                    bottom
                } else {
                    let Some(last_child) = indent.positions.last() else {
                        // this dir doesnt have children so we just return
                        return;
                    };
                    self.ui.clip_rect().clamp(pos2(top.x, last_child.y))
                };
                self.ui.painter().line_segment(
                    [top, bottom],
                    self.ui.visuals().widgets.noninteractive.bg_stroke,
                );
                for child_pos in indent.positions.iter() {
                    let p1 = pos2(top.x, child_pos.y);
                    let p2 = *child_pos + vec2(-2.0, 0.0);
                    self.ui
                        .painter()
                        .line_segment([p1, p2], self.ui.visuals().widgets.noninteractive.bg_stroke);
                }
            }
        }
    }

    fn draw_drop_marker(&self, row_y_range: Rangef, dir_position: &DirPosition<NodeIdType>) {
        pub const DROP_LINE_HEIGHT: f32 = 3.0;
        let x_range = self.ui.available_rect_before_wrap().x_range();
        let y_range = match dir_position {
            DirPosition::First => Rangef::point(row_y_range.max).expand(DROP_LINE_HEIGHT * 0.5),
            DirPosition::Last => Rangef::new(row_y_range.min, self.ui_data.space_used.bottom()),
            DirPosition::After(_) => Rangef::point(row_y_range.max).expand(DROP_LINE_HEIGHT * 0.5),
            DirPosition::Before(_) => Rangef::point(row_y_range.min).expand(DROP_LINE_HEIGHT * 0.5),
        };
        self.ui.painter().set(
            self.ui_data.drop_marker_idx,
            Shape::rect_filled(
                Rect::from_x_y_ranges(x_range, y_range),
                self.ui.visuals().widgets.active.corner_radius,
                self.ui
                    .style()
                    .visuals
                    .selection
                    .bg_fill
                    .linear_multiply(0.6),
            ),
        );
    }

    /// Add a node to the tree.
    ///
    /// If the node is a directory this method returns the openness state of the ndode.
    /// Returns `true` if the directory is open and its child are visible.
    /// Returns `false` if the directory is closed.
    ///
    /// If the node is a directory, you must call [`TreeViewBuilder::close_dir`] to close the directory.
    pub fn node(&mut self, mut config: impl NodeConfig<NodeIdType>) -> bool {
        self.decrement_current_dir_child_count();

        let (node_is_open, row_rect) = if self.current_branch_expanded() && !config.flatten() {
            let node = Node::from_config(
                if config.is_dir() {
                    self.state
                        .is_open(config.id())
                        .unwrap_or(config.default_open())
                } else {
                    true
                },
                self.ui.spacing().interact_size.y,
                self.indents.len(),
                &mut config,
            );
            let (node_is_open, row_rect) = self.node_structually_visible(node);
            (node_is_open, Some(row_rect))
        } else {
            (false, None)
        };

        if config.is_dir() {
            self.stack.push(DirectoryState {
                id: config.id().clone(),
                child_count: None,
                branch_expanded: self.current_branch_expanded() && node_is_open,
                branch_dragged: self.current_branch_dragged()
                    || self.state.is_dragged(config.id())
                    || match self.output {
                        Output::SetDragged(drag_state)
                        | Output::SetDraggedSelection(drag_state) => {
                            drag_state.dragged.contains(config.id())
                        }
                        _ => false,
                    },
                row_rect,
            });
        }

        if self.should_close_current_dir() {
            self.close_dir();
        }

        node_is_open
    }

    fn node_structually_visible(&mut self, mut node: Node<NodeIdType>) -> (bool, Rect) {
        let row_rect = Rect::from_min_size(
            self.ui_data.space_used.left_bottom(),
            vec2(
                self.ui_data.interaction.rect.width(),
                node.node_height + self.ui.spacing().item_spacing.y,
            ),
        );

        self.do_input_structually_visible(&node, &row_rect);

        if self.ui.clip_rect().intersects(row_rect) {
            let node_width = self.node_visible_in_clip_rect(&mut node, row_rect);
            if node_width > self.ui_data.space_used.width() {
                self.ui_data.space_used.set_width(node_width);
            }
        } else if self.ui_data.space_used.bottom() > self.ui.clip_rect().bottom() {
            if let Some(indent) = self.indents.last_mut() {
                indent.extends_below_clip_rect = true;
            }
        }
        *self.ui_data.space_used.bottom_mut() += row_rect.height();
        if node.is_dir {
            // If the directory is opened below the clip rect its indent hint is never
            // going to be visible anyways so we dont bother.
            // Therfore we only add the indent hint if the cursor after the node was added
            // is still in the clip rect.
            if self.ui_data.space_used.bottom() < self.ui.clip_rect().bottom() {
                self.indents.push(IndentState {
                    source_node: node.id.clone(),
                    anchor: row_rect.y_range(),
                    positions: Vec::new(),
                    indent: self.indents.len(),
                    extends_below_clip_rect: false,
                });
            }
        }

        self.do_output(&node);

        (node.is_open, row_rect)
    }

    fn node_visible_in_clip_rect(&mut self, node: &mut Node<NodeIdType>, outer_rect: Rect) -> f32 {
        // Draw background
        if self.state.is_selected(&node.id) {
            let (shape_idx, rect) = self
                .selection_background
                .get_or_insert_with(|| (self.ui.painter().add(Shape::Noop), Rect::NOTHING));
            *rect = Rect::from_min_max(rect.min.min(outer_rect.min), rect.max.max(outer_rect.max));
            let visuals = self.ui.visuals();
            let color = if self.ui_data.has_focus {
                visuals.selection.bg_fill
            } else {
                visuals.widgets.inactive.weak_bg_fill.linear_multiply(0.3)
            };
            self.ui.painter().set(
                *shape_idx,
                Shape::rect_filled(*rect, self.ui.visuals().widgets.active.corner_radius, color),
            );
        } else {
            self.selection_background = None;
        }

        // Draw pivot and cursor for debugging
        // if self.state.is_selection_pivot(&node.id) {
        //     self.ui
        //         .painter()
        //         .circle_filled(row_rect.left_center(), 10.0, egui::Color32::BLUE);
        // }
        // if self.state.is_selection_cursor(&node.id) {
        //     self.ui
        //         .painter()
        //         .circle_filled(row_rect.left_center(), 5.0, egui::Color32::RED);
        // }

        // Draw node
        let (closer, icon, label) = node.show_node(
            self.ui,
            &self.ui_data.interaction,
            self.settings,
            outer_rect,
            self.state.is_selected(&node.id),
            self.ui_data.has_focus,
        );

        // Do input
        self.do_input_output(node, &outer_rect, closer.as_ref());

        // Draw node dragged
        if self.state.is_dragged(&node.id) {
            self.ui
                .scope_builder(UiBuilder::new().layer_id(self.ui_data.drag_layer), |ui| {
                    if self.state.is_selected(&node.id) {
                        ui.painter().rect_filled(
                            outer_rect,
                            ui.visuals().widgets.active.corner_radius,
                            ui.visuals().selection.bg_fill.linear_multiply(0.4),
                        );
                    }
                    node.show_node(
                        ui,
                        &self.ui_data.interaction,
                        self.settings,
                        outer_rect,
                        false,
                        true,
                    );
                });
        }

        // Show the context menu.
        let was_right_clicked = self.state.is_secondary_selected(&node.id)
            || matches!(self.output, Output::SetSecondaryClicked(id) if id == &node.id);
        let was_only_target = !self.state.is_selected(&node.id)
            || self.state.is_selected(&node.id) && self.state.selected_count() == 1;
        if was_right_clicked && was_only_target {
            self.ui_data.context_menu_was_open = node.show_context_menu(&self.ui_data.interaction);
        }

        // Draw context menu marker
        if self.state.is_secondary_selected(&node.id) && self.ui_data.context_menu_was_open {
            self.ui.painter().rect_stroke(
                outer_rect,
                self.ui.visuals().widgets.active.corner_radius,
                self.ui.visuals().widgets.inactive.fg_stroke,
                egui::StrokeKind::Inside,
            );
        }

        // Draw selection cursor
        if self.state.is_selection_cursor(&node.id) {
            self.ui.painter().rect_stroke(
                outer_rect,
                self.ui.visuals().widgets.active.corner_radius,
                self.ui.visuals().widgets.inactive.fg_stroke,
                egui::StrokeKind::Inside,
            );
        }

        // Save position for indent hint
        if let Some(indent) = self.indents.last_mut() {
            indent
                .positions
                .push(closer.or(icon).unwrap_or(label).left_center());
        }

        label.right() - outer_rect.left()
    }

    fn do_input_structually_visible(&mut self, node: &Node<NodeIdType>, row_rect: &Rect) {
        match &mut self.input {
            Input::None => (),
            Input::DragStarted { .. } => (),
            Input::Dragged(_) => (),
            Input::Click { .. } => (),
            Input::SecondaryClick(_) => (),
            Input::KeyEnter { activatable_nodes } => {
                if self.state.is_selected(&node.id) && node.activatable {
                    activatable_nodes.push(node.id.clone());
                    *self.output = Output::ActivateSelection(activatable_nodes.clone());
                    *self.input = Input::None;
                }
            }
            Input::KeySpace => {
                if self.state.is_selection_cursor(&node.id) {
                    *self.output = Output::ToggleSelection(node.id.clone(), Some(*row_rect));
                    *self.input = Input::None;
                }
            }
            Input::KeyLeft => {
                if self.state.is_selected(&node.id) {
                    *self.input = Input::None;
                    if self.state.selected_count() == 1 {
                        if node.is_dir && node.is_open {
                            self.state.set_openness(node.id.clone(), !node.is_open);
                        } else if let Some(dir_state) = self.stack.last() {
                            *self.output =
                                Output::SelectOneNode(dir_state.id.clone(), dir_state.row_rect);
                        }
                    }
                }
            }
            Input::KeyRight { select_next } => {
                if *select_next {
                    *self.output = Output::SelectOneNode(node.id.clone(), Some(*row_rect));
                    *self.input = Input::None;
                } else if self.state.is_selected(&node.id) {
                    if self.state.selected_count() == 1 {
                        if node.is_dir && !node.is_open {
                            self.state.set_openness(node.id.clone(), !node.is_open);
                            *self.input = Input::None;
                        } else {
                            *select_next = true;
                        }
                    } else {
                        *self.input = Input::None;
                    }
                }
            }
            Input::KeyUp { previous_node } => 'arm: {
                let current_node_is_cursor = self
                    .state
                    .get_selection_cursor()
                    .or(self.state.get_selection_pivot())
                    .is_some_and(|cursor_id| cursor_id == &node.id);

                if current_node_is_cursor {
                    if let Some((previous_node, prev_rect)) = previous_node {
                        *self.output =
                            Output::SelectOneNode(previous_node.clone(), Some(*prev_rect));
                        *self.input = Input::None;
                        break 'arm;
                    } else {
                        *self.input = Input::None;
                        break 'arm;
                    }
                }

                *previous_node = Some((node.id.clone(), *row_rect));
            }
            Input::KeyUpAndCommand { previous_node } => 'arm: {
                let current_node_is_cursor = self
                    .state
                    .get_selection_cursor()
                    .or(self.state.get_selection_pivot())
                    .is_some_and(|cursor_id| cursor_id == &node.id);
                if current_node_is_cursor {
                    if let Some((previous_node, prev_rect)) = previous_node {
                        *self.output = Output::SetCursor(previous_node.clone(), *prev_rect);
                    }
                    *self.input = Input::None;
                    break 'arm;
                }
                *previous_node = Some((node.id.clone(), *row_rect));
            }
            Input::KeyUpAndShift {
                previous_node,
                nodes_to_select,
                next_cursor,
            } => 'arm: {
                let Some(pivot) = self.state.get_selection_pivot() else {
                    *self.input = Input::None;
                    break 'arm;
                };
                let previous_node = {
                    let mut current_node = Some((node.id.clone(), *row_rect));
                    std::mem::swap(&mut current_node, previous_node);
                    current_node
                };

                let Some((previous_node, previous_rect)) = previous_node else {
                    let current_node_is_cursor = self
                        .state
                        .get_selection_cursor()
                        .or(self.state.get_selection_pivot())
                        .is_some_and(|cursor_id| cursor_id == &node.id);
                    if current_node_is_cursor {
                        *self.input = Input::None;
                        break 'arm;
                    }
                    if pivot == &node.id {
                        *nodes_to_select = Some(vec![node.id.clone()]);
                    }
                    break 'arm;
                };

                let Some(cursor) = self.state.get_selection_cursor() else {
                    if self.state.is_selection_pivot(&node.id) {
                        *self.output = Output::Select {
                            selection: vec![previous_node.clone(), node.id.clone()],
                            pivot: pivot.clone(),
                            cursor: previous_node.clone(),
                            scroll_to_rect: previous_rect,
                        };
                        *self.input = Input::None;
                        break 'arm;
                    };
                    break 'arm;
                };

                if let Some(nodes_to_select) = nodes_to_select {
                    if cursor == &node.id {
                        *self.output = Output::Select {
                            selection: nodes_to_select.clone(),
                            pivot: pivot.clone(),
                            cursor: previous_node.clone(),
                            scroll_to_rect: previous_rect,
                        };
                        *self.input = Input::None;
                        break 'arm;
                    } else if pivot == &node.id {
                        nodes_to_select.push(node.id.clone());
                        let (next_cursor, next_rect) = next_cursor
                            .clone()
                            .expect("The selection should have started on the cursor which would have se this value");
                        *self.output = Output::Select {
                            selection: nodes_to_select.clone(),
                            pivot: pivot.clone(),
                            cursor: next_cursor,
                            scroll_to_rect: next_rect,
                        };
                        *self.input = Input::None;
                        break 'arm;
                    } else {
                        nodes_to_select.push(node.id.clone());
                    }
                } else {
                    if cursor == &node.id && pivot == &node.id {
                        *self.output = Output::Select {
                            selection: vec![previous_node.clone(), node.id.clone()],
                            pivot: pivot.clone(),
                            cursor: previous_node.clone(),
                            scroll_to_rect: previous_rect,
                        };
                        *self.input = Input::None;
                        break 'arm;
                    }
                    if cursor == &node.id {
                        *nodes_to_select = Some(vec![previous_node.clone(), node.id.clone()]);
                        *next_cursor = Some((previous_node.clone(), previous_rect));
                    } else if pivot == &node.id {
                        *nodes_to_select = Some(vec![node.id.clone()]);
                    }
                }
            }
            Input::KeyDown(is_next) => 'arm: {
                if *is_next {
                    *self.output = Output::SelectOneNode(node.id.clone(), Some(*row_rect));
                    *self.input = Input::None;
                    break 'arm;
                }
                *is_next = self
                    .state
                    .get_selection_cursor()
                    .or(self.state.get_selection_pivot())
                    .is_some_and(|cursor_id| cursor_id == &node.id);
            }
            Input::KeyDownAndCommand { is_next } => 'arm: {
                if *is_next {
                    *self.output = Output::SetCursor(node.id.clone(), *row_rect);
                    *self.input = Input::None;
                    break 'arm;
                }
                *is_next = self
                    .state
                    .get_selection_cursor()
                    .or(self.state.get_selection_pivot())
                    .is_some_and(|cursor_id| cursor_id == &node.id);
            }
            Input::KeyDownAndShift {
                nodes_to_select,
                next_cursor,
                is_next,
            } => 'arm: {
                let Some(pivot) = self.state.get_selection_pivot() else {
                    *self.input = Input::None;
                    break 'arm;
                };

                if let Some(nodes_to_select) = nodes_to_select {
                    nodes_to_select.push(node.id.clone());
                    if *is_next {
                        *self.output = Output::Select {
                            selection: nodes_to_select.clone(),
                            pivot: pivot.clone(),
                            cursor: node.id.clone(),
                            scroll_to_rect: *row_rect,
                        };
                        *self.input = Input::None;
                        break 'arm;
                    } else if pivot == &node.id {
                        let (next_cursor, next_rect) = next_cursor
                            .clone()
                            .expect("The selection should have started on the cursor which would have se this value");
                        *self.output = Output::Select {
                            selection: nodes_to_select.clone(),
                            pivot: pivot.clone(),
                            cursor: next_cursor,
                            scroll_to_rect: next_rect,
                        };
                        *self.input = Input::None;
                        break 'arm;
                    }
                } else {
                    if *is_next && pivot == &node.id {
                        *self.output = Output::Select {
                            selection: vec![node.id.clone()],
                            pivot: pivot.clone(),
                            cursor: node.id.clone(),
                            scroll_to_rect: *row_rect,
                        };
                        *self.input = Input::None;
                        break 'arm;
                    }
                    if *is_next {
                        *nodes_to_select = Some(vec![node.id.clone()]);
                        *next_cursor = Some((node.id.clone(), *row_rect));
                    } else if pivot == &node.id {
                        *nodes_to_select = Some(vec![node.id.clone()]);
                    }
                }

                *is_next = self
                    .state
                    .get_selection_cursor()
                    .or(self.state.get_selection_pivot())
                    .is_some_and(|cursor_id| cursor_id == &node.id);
            }
        }
    }

    fn do_input_output(&mut self, node: &Node<NodeIdType>, row_rect: &Rect, closer: Option<&Rect>) {
        // Handle inputs
        let current_branch_dragged = self.current_branch_dragged();
        match &mut self.input {
            Input::DragStarted {
                pos,
                simplified_dragged,
            } => {
                if self.state.is_selected(&node.id) && !current_branch_dragged {
                    simplified_dragged.push(node.id.clone());
                }
                if rect_contains_visually(row_rect, pos) {
                    if self.state.is_selected(&node.id) {
                        *self.output = Output::SetDraggedSelection(DragState {
                            dragged: self.state.get_selection().clone(),
                            simplified: simplified_dragged.clone(),
                        });
                    } else {
                        *self.output = Output::SetDragged(DragState {
                            dragged: vec![node.id.clone()],
                            simplified: vec![node.id.clone()],
                        });
                    }
                }
            }
            Input::Dragged(pos) => {
                if rect_contains_visually(row_rect, pos)
                    && !self.current_branch_dragged()
                    && !self.state.is_dragged(&node.id)
                {
                    self.ui_data.drop_target = self.get_drop_position(row_rect, node);
                    match self.ui_data.drop_target.as_ref() {
                        Some((_, dir_position)) if dir_position != &DirPosition::Last => {
                            self.draw_drop_marker(row_rect.y_range(), dir_position);
                        }
                        _ => (),
                    };
                    *self.input = Input::None;
                }
            }
            Input::Click {
                pos,
                double,
                modifiers,
                activatable_nodes,
                shift_click_nodes,
            } => 'block: {
                // Closer click
                if closer.is_some_and(|closer| rect_contains_visually(closer, pos)) {
                    self.state.set_openness(node.id.clone(), !node.is_open);
                    *self.input = Input::None;
                    break 'block;
                }

                let row_clicked = rect_contains_visually(row_rect, pos);
                let double_click = row_clicked && *double && self.state.was_clicked_last(&node.id);
                if row_clicked {
                    self.state.set_last_clicked(&node.id);
                }

                // upkeep for the activate action
                if self.state.is_selected(&node.id) && node.activatable {
                    activatable_nodes.push(node.id.clone());
                }

                // Double clicked
                if double_click {
                    self.state.set_openness(node.id.clone(), !node.is_open);
                    if node.activatable {
                        if self.state.is_selected(&node.id) {
                            *self.output = Output::ActivateSelection(activatable_nodes.clone());
                        } else {
                            *self.output = Output::ActivateThis(node.id.clone());
                        }
                    }
                    *self.input = Input::None;
                    break 'block;
                }
                // Single click
                if modifiers.shift_only() {
                    if let Some(shift_click_nodes) = shift_click_nodes {
                        shift_click_nodes.push(node.id.clone());
                        if row_clicked || self.state.is_selection_pivot(&node.id) {
                            *self.output = Output::ShiftSelect(shift_click_nodes.clone());
                            *self.input = Input::None;
                            break 'block;
                        }
                    } else if row_clicked || self.state.is_selection_pivot(&node.id) {
                        *shift_click_nodes = Some(vec![node.id.clone()]);
                    }
                } else if modifiers.command_only() {
                    if row_clicked {
                        *self.output = Output::ToggleSelection(node.id.clone(), None);
                        *self.input = Input::None;
                        break 'block;
                    }
                } else if row_clicked {
                    *self.output = Output::SelectOneNode(node.id.clone(), None);
                    *self.input = Input::None;
                    break 'block;
                }
            }
            Input::SecondaryClick(pos) => {
                if rect_contains_visually(row_rect, pos) {
                    *self.output = Output::SetSecondaryClicked(node.id.clone());
                }
            }
            Input::KeyLeft => (),
            Input::KeyRight { .. } => (),
            Input::KeyUp { .. } => (),
            Input::KeyUpAndCommand { .. } => (),
            Input::KeyUpAndShift { .. } => (),
            Input::KeyDown(_) => (),
            Input::KeyDownAndCommand { .. } => (),
            Input::KeyDownAndShift { .. } => (),
            Input::KeySpace => (),
            Input::KeyEnter { .. } => (),
            Input::None => (),
        };
    }
    fn do_output(&mut self, node: &Node<NodeIdType>) {
        let current_branch_dragged = self.current_branch_dragged();
        match self.output {
            Output::ActivateSelection(selection) => {
                if self.state.is_selected(&node.id)
                    && node.activatable
                    && !selection.contains(&node.id)
                {
                    selection.push(node.id.clone());
                }
            }
            Output::SetDraggedSelection(drag_state) => {
                if self.state.is_selected(&node.id) && !current_branch_dragged {
                    drag_state.simplified.push(node.id.clone());
                }
            }
            _ => (),
        }
    }

    fn get_drop_position(
        &self,
        row: &Rect,
        node: &Node<NodeIdType>,
    ) -> Option<(NodeIdType, DirPosition<NodeIdType>)> {
        let drop_quarter = self
            .ui_data
            .interaction
            .hover_pos()
            .and_then(|pos| DropQuarter::new(row.y_range(), pos.y))
            .expect("Cursor is above row so the drop quarter should be known");
        match drop_quarter {
            DropQuarter::Top => {
                if let Some(parent_id) = self.parent_id() {
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
                if let Some(parent_id) = self.parent_id() {
                    return Some((parent_id.clone(), DirPosition::Before(node.id.clone())));
                }
                None
            }
            DropQuarter::MiddleBottom => {
                if node.drop_allowed {
                    return Some((node.id.clone(), DirPosition::Last));
                }
                if let Some(parent_id) = self.parent_id() {
                    return Some((parent_id.clone(), DirPosition::After(node.id.clone())));
                }
                None
            }
            DropQuarter::Bottom => {
                if node.drop_allowed && node.is_open {
                    return Some((node.id.clone(), DirPosition::First));
                }
                if let Some(parent_id) = self.parent_id() {
                    return Some((parent_id.clone(), DirPosition::After(node.id.clone())));
                }
                if node.drop_allowed {
                    return Some((node.id.clone(), DirPosition::Last));
                }
                None
            }
        }
    }

    fn should_close_current_dir(&self) -> bool {
        self.stack
            .last()
            .and_then(|dir| dir.child_count)
            .is_some_and(|count| count == 0)
    }
    fn decrement_current_dir_child_count(&mut self) {
        if let Some(dir_state) = self.stack.last_mut() {
            if let Some(child_count) = &mut dir_state.child_count {
                *child_count -= 1;
            }
        }
    }

    fn current_branch_expanded(&self) -> bool {
        self.stack.last().is_none_or(|state| state.branch_expanded)
    }
    fn current_branch_dragged(&self) -> bool {
        let Some(dir_state) = self.stack.last() else {
            return false;
        };
        dir_state.branch_dragged
    }
}
