use egui::{layers::ShapeIdx, pos2, vec2, Pos2, Rangef, Rect, Shape, Ui, WidgetText};

use crate::{
    builder_state::BuilderState, node::NodeBuilder, rect_contains_visually, DirPosition, Dragged,
    DropQuarter, IndentHintStyle, NodeId, PartialTreeViewState, RowRectangles, TreeViewSettings,
    TreeViewState, UiData,
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
}
struct IndentState<NodeIdType> {
    /// Id of the node that created this indent
    source_node: NodeIdType,
    /// Anchor for the indent hint at the source directory
    anchor: Rangef,
    /// Positions of child nodes for the indent hint.
    positions: Vec<Pos2>,
}

/// The builder used to construct the tree.
///
/// Use this to add directories or leaves to the tree.
pub struct TreeViewBuilder<'ui, NodeIdType> {
    ui: &'ui mut Ui,
    state: PartialTreeViewState<'ui, NodeIdType>,
    settings: &'ui TreeViewSettings,
    ui_data: &'ui mut UiData<NodeIdType>,
    builder_state: BuilderState<'ui, NodeIdType>,
    selection_background: Option<(ShapeIdx, Rect)>,
    stack: Vec<DirectoryState<NodeIdType>>,
    indents: Vec<IndentState<NodeIdType>>,
}

impl<'ui, NodeIdType: NodeId> TreeViewBuilder<'ui, NodeIdType> {
    pub(crate) fn new(
        ui: &'ui mut Ui,
        state: &'ui mut TreeViewState<NodeIdType>,
        settings: &'ui TreeViewSettings,
        ui_data: &'ui mut UiData<NodeIdType>,
    ) -> Self {
        let (node_states, state) = state.split();
        Self {
            ui_data,
            state,
            settings,
            ui,
            builder_state: BuilderState::new(node_states),
            selection_background: None,
            stack: Vec::new(),
            indents: Vec::new(),
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
        } else {
            if let Some(dir_state) = self.stack.last_mut() {
                dir_state.child_count = Some(child_count);
            }
        }
    }

    /// Close the current directory.
    pub fn close_dir(&mut self) {
        while let Some(dir_state) = self.stack.pop() {
            let indent = self
                .indents
                .pop_if(|indent| indent.source_node == dir_state.id);
            if let Some(indent) = indent {
                self.draw_indent_hint(indent.anchor, indent.positions, self.indents.len());
                self.draw_directory_drop_marker(indent.anchor, &dir_state.id);
            }
            if !self.should_close_current_dir() {
                break;
            }
        }
    }

    fn draw_indent_hint(&mut self, anchor: Rangef, positions: Vec<Pos2>, level: usize) {
        let top = pos2(
            self.ui.cursor().min.x
                + self.ui.spacing().item_spacing.x
                + self.ui.spacing().icon_width * 0.5
                + level as f32
                    * self
                        .settings
                        .override_indent
                        .unwrap_or(self.ui.spacing().indent),
            anchor.center() + self.ui.spacing().icon_width * 0.5 + 2.0,
        );

        match self.settings.indent_hint_style {
            IndentHintStyle::None => return,
            IndentHintStyle::Line => {
                let bottom = pos2(
                    top.x,
                    self.ui.cursor().min.y - self.ui.spacing().item_spacing.y,
                );
                self.ui.painter().line_segment(
                    [top, bottom],
                    self.ui.visuals().widgets.noninteractive.bg_stroke,
                );
            }
            IndentHintStyle::Hook => {
                let Some(last_child) = positions.last() else {
                    // this dir doesnt have children so we just return
                    return;
                };
                let bottom = pos2(top.x, last_child.y);
                self.ui.painter().line_segment(
                    [top, bottom],
                    self.ui.visuals().widgets.noninteractive.bg_stroke,
                );
                for child_pos in positions.iter() {
                    let p1 = pos2(top.x, child_pos.y);
                    let p2 = *child_pos + vec2(-2.0, 0.0);
                    self.ui
                        .painter()
                        .line_segment([p1, p2], self.ui.visuals().widgets.noninteractive.bg_stroke);
                }
            }
        }
    }

    fn draw_directory_drop_marker(&mut self, row_range: Rangef, closed_id: &NodeIdType) {
        let draw_drop_marker = matches!(
            self.ui_data.drop_target.as_ref(),
            Some((id, DirPosition::Last)) if id == closed_id
        );
        if draw_drop_marker {
            let rect = Rect::from_min_max(
                pos2(self.ui.cursor().min.x, row_range.min),
                pos2(self.ui.cursor().max.x, self.ui.cursor().min.y),
            );
            let color = self
                .ui
                .style()
                .visuals
                .selection
                .bg_fill
                .linear_multiply(0.6);
            let radius = self.ui.visuals().widgets.active.corner_radius;
            self.ui.painter().set(
                self.ui_data.drop_marker_idx,
                Shape::rect_filled(rect, radius, color),
            );
        }
    }

    /// Add a node to the tree.
    ///
    /// If the node is a directory this method returns the openness state of the ndode.
    /// Returns `true` if the directory is open and its child are visible.
    /// Returns `false` if the directory is closed.
    ///
    /// If the node is a directory, you must call [`TreeViewBuilder::close_dir`] to close the directory.
    pub fn node(&mut self, mut node: NodeBuilder<NodeIdType>) -> bool {
        self.decrement_current_dir_child_count();

        let is_open = self.builder_state.update_and_insert_node(
            &node,
            self.parent_id().cloned(),
            self.current_branch_expanded(),
        );
        node.set_is_open(is_open);
        node.set_indent(self.indents.len());
        node.set_height(
            node.node_height
                .unwrap_or(self.ui.spacing().interact_size.y),
        );

        if self.current_branch_expanded() && !node.flatten {
            self.node_structually_visible(&mut node);
        }

        if node.is_dir {
            self.stack.push(DirectoryState {
                id: node.id.clone(),
                child_count: None,
                branch_expanded: self.current_branch_expanded() && node.is_open,
                branch_dragged: self.current_branch_dragged() || self.state.is_dragged(&node.id),
            });
        }

        if self.should_close_current_dir() {
            self.close_dir();
        }

        node.is_open
    }

    fn node_structually_visible(&mut self, node: &mut NodeBuilder<NodeIdType>) {
        let row_range = Rangef::new(
            self.ui.cursor().min.y,
            self.ui.cursor().min.y + node.node_height.unwrap(),
        )
        .expand(self.ui.spacing().item_spacing.y);
        let in_clip_rect = self.ui.clip_rect().y_range().intersects(row_range);
        if in_clip_rect {
            self.node_visible_in_clip_rect(node);
        }
        if node.is_dir {
            self.indents.push(IndentState {
                source_node: node.id.clone(),
                anchor: row_range,
                positions: Vec::new(),
            });
        }
    }

    fn node_visible_in_clip_rect(&mut self, node: &mut NodeBuilder<NodeIdType>) {
        let row_rect = Rect::from_min_max(
            self.ui.cursor().min,
            pos2(
                self.ui.cursor().max.x,
                self.ui.cursor().min.y + node.node_height.unwrap(),
            ),
        )
        .expand2(vec2(0.0, self.ui.spacing().item_spacing.y * 0.5));
        let cursor_above_row = self
            .ui_data
            .interaction
            .hover_pos()
            .is_some_and(|pos| rect_contains_visually(&row_rect, &pos));
        let pressed_on_this_row = self
            .ui
            .input(|i| i.pointer.press_origin())
            .is_some_and(|pos| rect_contains_visually(&row_rect, &pos));

        // React to secondary clicks
        // Context menus in egui only show up when the secondary mouse button is pressed.
        // Since we are handling inputs after the tree has already been build we only know
        // that we should show a context menu one frame after the click has happened and the
        // context menu would never show up.
        // To fix this we handle the secondary click here and return the even in the result.
        let is_mouse_above_row = self
            .ui_data
            .interaction
            .hover_pos()
            .is_some_and(|pos| row_rect.contains(pos));
        if is_mouse_above_row && self.ui_data.interaction.secondary_clicked() {
            self.ui_data.seconday_click = Some(node.id.clone());
        }

        // Handle drag start
        if self.ui_data.interaction.drag_started() && pressed_on_this_row {
            if self.state.is_selected(&node.id) {
                self.ui_data.new_dragged = Some(Dragged::Selection);
            } else {
                self.ui_data.new_dragged = Some(Dragged::One(node.id.clone()));
            }
        }

        // Show the context menu.
        let was_right_clicked = self
            .ui_data
            .seconday_click
            .as_ref()
            .is_some_and(|id| id == &node.id)
            || self.state.is_secondary_selected(&node.id);
        let was_only_target = !self.state.is_selected(&node.id)
            || self.state.is_selected(&node.id) && self.state.selected_count() == 1;
        if was_right_clicked && was_only_target {
            self.ui_data.context_menu_was_open = node.show_context_menu(&self.ui_data.interaction);
        }

        // Draw background
        if self.state.is_selected(&node.id) {
            let (shape_idx, rect) = self
                .selection_background
                .get_or_insert_with(|| (self.ui.painter().add(Shape::Noop), Rect::NOTHING));
            *rect = Rect::from_min_max(rect.min.min(row_rect.min), rect.max.max(row_rect.max));
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

        // Draw context menu marker
        if self.state.is_secondary_selected(&node.id) && self.ui_data.context_menu_was_open {
            self.ui.painter().rect_stroke(
                row_rect,
                self.ui.visuals().widgets.active.corner_radius,
                self.ui.visuals().widgets.inactive.fg_stroke,
                egui::StrokeKind::Inside,
            );
        }

        // Draw selection cursor
        if self.state.is_selection_cursor(&node.id) {
            self.ui.painter().rect_stroke(
                row_rect,
                self.ui.visuals().widgets.active.corner_radius,
                self.ui.visuals().widgets.inactive.fg_stroke,
                egui::StrokeKind::Inside,
            );
        }

        // Handle drop position
        if !self.ui_data.interaction.drag_started()
            && (self.ui_data.interaction.dragged() || self.ui_data.interaction.drag_stopped())
            && cursor_above_row
        {
            if !self.current_branch_dragged() && !self.state.is_dragged(&node.id) {
                self.ui_data.drop_target = self.get_drop_position(row_rect, node);
            }
        }

        // Draw drop marker
        if cursor_above_row {
            if let Some((_, position)) = self.ui_data.drop_target.as_ref() {
                let color = self
                    .ui
                    .style()
                    .visuals
                    .selection
                    .bg_fill
                    .linear_multiply(0.6);
                let radius = self.ui.visuals().widgets.active.corner_radius;
                pub const DROP_LINE_HEIGHT: f32 = 3.0;
                if !matches!(position, DirPosition::Last) {
                    let y = match position {
                        DirPosition::First => row_rect.max.y,
                        DirPosition::After(_) => row_rect.max.y,
                        DirPosition::Before(_) => row_rect.min.y,
                        DirPosition::Last => unreachable!(),
                    };
                    let rect = Rect::from_x_y_ranges(
                        row_rect.x_range(),
                        Rangef::point(y).expand(DROP_LINE_HEIGHT * 0.5),
                    );

                    self.ui.painter().set(
                        self.ui_data.drop_marker_idx,
                        Shape::rect_filled(rect, radius, color),
                    );
                }
            }
        }

        let drag_overlay_rect = self.ui.available_rect_before_wrap();

        // Draw node
        let (row, closer, icon, label) = self
            .ui
            .scope(|ui| {
                // Set the fg stroke colors here so that the ui added by the user
                // has the correct colors when selected or focused.
                let fg_stroke = if self.state.is_selected(&node.id) && self.ui_data.has_focus {
                    ui.visuals().selection.stroke
                } else if self.state.is_selected(&node.id) {
                    ui.visuals().widgets.inactive.fg_stroke
                } else {
                    ui.visuals().widgets.noninteractive.fg_stroke
                };
                ui.visuals_mut().widgets.noninteractive.fg_stroke = fg_stroke;
                ui.visuals_mut().widgets.inactive.fg_stroke = fg_stroke;

                node.show_node(ui, &self.ui_data.interaction, self.settings)
            })
            .inner;

        // Draw node dragged
        if self.ui_data.interaction.dragged() && self.state.is_dragged(&node.id) {
            node.show_node_dragged(
                self.ui,
                &self.ui_data.interaction,
                self.settings,
                self.ui_data.drag_layer,
                drag_overlay_rect,
            );
        }

        // Save rectangles for later
        self.ui_data.row_rectangles.insert(
            node.id.clone(),
            RowRectangles {
                row_rect: row,
                closer_rect: closer,
            },
        );
        // Save position for indent hint
        if let Some(indent) = self.indents.last_mut() {
            indent
                .positions
                .push(closer.or(icon).unwrap_or(label).left_center());
        }
    }

    fn get_drop_position(
        &self,
        row: Rect,
        node: &NodeBuilder<NodeIdType>,
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
