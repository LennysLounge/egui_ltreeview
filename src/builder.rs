use std::collections::HashMap;

use egui::{pos2, vec2, LayerId, Order, Rangef, Rect, Response, Ui, WidgetText};

use crate::{
    builder_state::BuilderState, node::NodeBuilder, node_states::NodeStates, IndentHintStyle,
    NodeId, TreeViewSettings, TreeViewState,
};

// #[derive(Clone)]
// pub struct DirectoryState<NodeIdType> {
//     /// Id of the directory node.
//     id: NodeIdType,
//     /// If directory is expanded
//     is_open: bool,
// }
// pub struct IndentState<NodeIdType> {
//     /// Id of the node that created this indent
//     source_node: NodeIdType,
//     /// Anchor for the indent hint at the source directory
//     anchor: Pos2,
//     /// Positions of child nodes for the indent hint.
//     positions: Vec<Pos2>,
// }

pub(crate) struct TreeViewBuilderResult<NodeIdType> {
    pub(crate) new_node_states: NodeStates<NodeIdType>,
    pub(crate) row_rectangles: HashMap<NodeIdType, RowRectangles>,
    pub(crate) seconday_click: Option<NodeIdType>,
    pub(crate) context_menu_was_open: bool,
    pub(crate) interaction: Response,
    pub(crate) drag_layer: LayerId,
}

pub(crate) struct RowRectangles {
    pub(crate) row_rect: Rect,
    pub(crate) closer_rect: Option<Rect>,
}

/// The builder used to construct the tree.
///
/// Use this to add directories or leaves to the tree.
pub struct TreeViewBuilder<'ui, NodeIdType> {
    ui: &'ui mut Ui,
    state: &'ui TreeViewState<NodeIdType>,
    settings: &'ui TreeViewSettings,
    tree_has_focus: bool,
    result: TreeViewBuilderResult<NodeIdType>,
    builder_state: BuilderState<NodeIdType>,
}

impl<'ui, NodeIdType: NodeId> TreeViewBuilder<'ui, NodeIdType> {
    pub(crate) fn new(
        ui: &'ui mut Ui,
        interaction: Response,
        state: &'ui mut TreeViewState<NodeIdType>,
        settings: &'ui TreeViewSettings,
        tree_has_focus: bool,
    ) -> Self {
        Self {
            result: TreeViewBuilderResult {
                new_node_states: NodeStates::new(),
                row_rectangles: HashMap::new(),
                seconday_click: None,
                interaction,
                context_menu_was_open: false,
                drag_layer: LayerId::new(
                    Order::Tooltip,
                    ui.make_persistent_id("ltreeviw drag layer"),
                ),
            },
            state,
            settings,
            tree_has_focus,
            ui,
            builder_state: BuilderState::new(),
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
    /// Must call [`TreeViewBuilder::close_dir`] to close the directory.
    ///
    /// To customize the node that is added to the tree consider using [`TreeViewBuilder::node`]
    pub fn dir(&mut self, id: NodeIdType, label: impl Into<WidgetText>) {
        let widget_text = label.into();
        self.node(NodeBuilder::dir(id).label_ui(|ui| {
            ui.add(egui::Label::new(widget_text.clone()).selectable(false));
        }));
    }

    /// Close the current directory.
    pub fn close_dir(&mut self) {
        let Some((anchor, positions, level)) = self.builder_state.close_dir() else {
            return;
        };
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
    /// If the node is a directory, you must call [`TreeViewBuilder::close_dir`] to close the directory.
    pub fn node(&mut self, mut node: NodeBuilder<NodeIdType>) {
        let open = self
            .state
            .node_state_of(&node.id)
            .map(|node_state| node_state.open)
            .unwrap_or(node.default_open);
        node.set_is_open(open);
        node.set_indent(self.builder_state.get_indent());
        self.builder_state.insert_node(&node);

        let node_response = self.node_internal(&mut node);

        self.builder_state
            .insert_node_response(&node, node_response);
    }

    pub(crate) fn get_result(mut self) -> TreeViewBuilderResult<NodeIdType> {
        let (new_node_states, row_rectangles) = self.builder_state.take();
        self.result.new_node_states = new_node_states;
        self.result.row_rectangles = row_rectangles;
        self.result
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
                .expect("Should have been filled with a default value"),
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
                let fg_stroke = if self.state.is_selected(&node.id) && self.tree_has_focus {
                    ui.visuals().selection.stroke
                } else if self.state.is_selected(&node.id) {
                    ui.visuals().widgets.inactive.fg_stroke
                } else {
                    ui.visuals().widgets.noninteractive.fg_stroke
                };
                ui.visuals_mut().widgets.noninteractive.fg_stroke = fg_stroke;
                ui.visuals_mut().widgets.inactive.fg_stroke = fg_stroke;

                node.show_node(ui, &self.result.interaction, self.settings)
            })
            .inner;

        if self.state.is_dragged(&node.id) {
            node.show_node_dragged(
                self.ui,
                &self.result.interaction,
                self.settings,
                self.result.drag_layer,
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
            .result
            .interaction
            .hover_pos()
            .is_some_and(|pos| row.contains(pos));
        if is_mouse_above_row
            && self.result.interaction.secondary_clicked()
            && !self.state.drag_valid()
        {
            self.result.seconday_click = Some(node.id);
        }

        // Show the context menu.
        let was_right_clicked = self.result.seconday_click.is_some_and(|id| id == node.id)
            || self.state.is_secondary_selected(&node.id);
        let was_only_target = !self.state.is_selected(&node.id)
            || self.state.is_selected(&node.id) && self.state.selected().len() == 1;
        if was_right_clicked && was_only_target {
            self.result.context_menu_was_open = node.show_context_menu(&self.result.interaction);
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
