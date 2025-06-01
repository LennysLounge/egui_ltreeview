use egui::{pos2, vec2, Pos2, Rangef, Rect, Ui, WidgetText};

use crate::{
    builder_state::BuilderState, node::NodeBuilder, IndentHintStyle, NodeId, PartialTreeViewState,
    RowRectangles, TreeViewSettings, TreeViewState, UiData,
};

/// The builder used to construct the tree.
///
/// Use this to add directories or leaves to the tree.
pub struct TreeViewBuilder<'ui, NodeIdType> {
    ui: &'ui mut Ui,
    state: PartialTreeViewState<'ui, NodeIdType>,
    settings: &'ui TreeViewSettings,
    ui_data: &'ui mut UiData<NodeIdType>,
    builder_state: BuilderState<'ui, NodeIdType>,
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
        }
    }

    /// Get the current parent id if any.
    pub fn parent_id(&self) -> Option<NodeIdType> {
        self.builder_state.parent_id()
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
        self.builder_state.set_child_count(child_count);
    }

    /// Close the current directory.
    pub fn close_dir(&mut self) {
        loop {
            if let Some((anchor, positions, level)) = self.builder_state.close_dir() {
                self.draw_indent_hint(anchor, positions, level);
            }
            if !self.builder_state.should_close_current_dir() {
                break;
            }
        }
    }

    fn draw_indent_hint(&mut self, anchor: f32, positions: Vec<Pos2>, level: usize) {
        let top = pos2(
            self.ui.cursor().min.x
                + self.ui.spacing().item_spacing.x
                + self.ui.spacing().icon_width * 0.5
                + level as f32
                    * self
                        .settings
                        .override_indent
                        .unwrap_or(self.ui.spacing().indent),
            anchor + self.ui.spacing().icon_width * 0.5 + 2.0,
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

    /// Add a node to the tree.
    ///
    /// If the node is a directory this method returns the openness state of the ndode.
    /// Returns `true` if the directory is open and its child are visible.
    /// Returns `false` if the directory is closed.
    ///
    /// If the node is a directory, you must call [`TreeViewBuilder::close_dir`] to close the directory.
    pub fn node(&mut self, mut node: NodeBuilder<NodeIdType>) -> bool {
        node = self.builder_state.update_and_insert_node(node);

        let node_response = self.node_internal(&mut node);

        if let Some(node_rects) = node_response.as_ref().and_then(|nr| nr.rects.as_ref()) {
            self.ui_data.row_rectangles.insert(
                node.id.clone(),
                RowRectangles {
                    row_rect: node_rects.row,
                    closer_rect: node_rects.closer,
                },
            );
        }

        self.builder_state
            .insert_node_response(&node, node_response);

        if self.builder_state.should_close_current_dir() {
            self.close_dir();
        }

        node.is_open
    }

    fn node_internal(&mut self, node: &mut NodeBuilder<NodeIdType>) -> Option<NodeResponse> {
        if !self.builder_state.parent_dir_is_open() {
            return None;
        }
        if node.flatten {
            return None;
        }

        let node_height = *node.node_height.get_or_insert(
            self.settings
                .default_node_height
                .unwrap_or(self.ui.spacing().interact_size.y),
        );
        let row_range = Rangef::new(self.ui.cursor().min.y, self.ui.cursor().min.y + node_height)
            .expand(self.ui.spacing().item_spacing.y);
        let is_visible = self.ui.clip_rect().y_range().intersects(row_range);
        if !is_visible {
            self.ui
                .add_space(node_height + self.ui.spacing().item_spacing.y);
            return Some(NodeResponse {
                range: row_range,
                rects: None,
            });
        }

        let drag_overlay_rect = self.ui.available_rect_before_wrap();

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

        if self.state.is_dragged(&node.id) {
            node.show_node_dragged(
                self.ui,
                &self.ui_data.interaction,
                self.settings,
                self.ui_data.drag_layer,
                drag_overlay_rect,
            );
        }

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
            .is_some_and(|pos| row.contains(pos));
        if is_mouse_above_row
            && self.ui_data.interaction.secondary_clicked()
            && !self.state.drag_valid()
        {
            self.ui_data.seconday_click = Some(node.id.clone());
        }

        // Show the context menu.
        let was_right_clicked = self.ui_data.seconday_click.as_ref().is_some_and(|id| id == &node.id)
            || self.state.is_secondary_selected(&node.id);
        let was_only_target = !self.state.is_selected(&node.id)
            || self.state.is_selected(&node.id) && self.state.selected_count() == 1;
        if was_right_clicked && was_only_target {
            self.ui_data.context_menu_was_open = node.show_context_menu(&self.ui_data.interaction);
        }

        Some(NodeResponse {
            range: row_range,
            rects: Some(NodeRectangles {
                row,
                closer,
                icon,
                label,
            }),
        })
    }
}

pub(crate) struct NodeResponse {
    pub range: Rangef,
    pub rects: Option<NodeRectangles>,
}
pub(crate) struct NodeRectangles {
    pub row: Rect,
    pub closer: Option<Rect>,
    pub icon: Option<Rect>,
    pub label: Rect,
}
