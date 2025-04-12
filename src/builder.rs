use std::collections::HashMap;

use egui::{pos2, vec2, LayerId, Order, Pos2, Rect, Response, Ui, WidgetText};

use crate::{
    node::NodeBuilder, IndentHintStyle, NodeId, NodeState, TreeViewSettings, TreeViewState,
};

#[derive(Clone)]
struct DirectoryState<NodeIdType> {
    /// Id of the directory node.
    id: NodeIdType,
    /// If directory is expanded
    is_open: bool,
    /// The rectangle of the icon.
    icon_rect: Rect,
    /// Positions of each child node of this directory.
    child_node_positions: Vec<Pos2>,
    /// The level of indentation.
    indent_level: usize,
    /// If this dir was flattened.
    flattened: bool,
}

pub(crate) struct TreeViewBuilderResult<NodeIdType> {
    pub(crate) new_node_states: Vec<NodeState<NodeIdType>>,
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
    stack: Vec<DirectoryState<NodeIdType>>,
    tree_has_focus: bool,
    result: TreeViewBuilderResult<NodeIdType>,
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
                new_node_states: Vec::new(),
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
            stack: Vec::new(),
            settings,
            tree_has_focus,
            ui,
        }
    }

    /// Get the current parent id if any.
    pub fn parent_id(&self) -> Option<NodeIdType> {
        self.parent_dir().map(|state| state.id)
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
        let Some(current_dir) = self.stack.pop() else {
            return;
        };

        // Draw indent hint
        if current_dir.is_open {
            let top = current_dir.icon_rect.center_bottom() + vec2(0.0, 2.0);

            let bottom = match self.settings.indent_hint_style {
                IndentHintStyle::None => top,
                IndentHintStyle::Line => pos2(
                    top.x,
                    self.ui.cursor().min.y - self.ui.spacing().item_spacing.y,
                ),
                IndentHintStyle::Hook => pos2(
                    top.x,
                    current_dir
                        .child_node_positions
                        .last()
                        .map(|pos| pos.y)
                        .unwrap_or(top.y),
                ),
            };
            self.ui.painter().line_segment(
                [top, bottom],
                self.ui.visuals().widgets.noninteractive.bg_stroke,
            );
            if matches!(self.settings.indent_hint_style, IndentHintStyle::Hook) {
                for child_pos in current_dir.child_node_positions.iter() {
                    let p1 = pos2(top.x, child_pos.y);
                    let p2 = *child_pos + vec2(-2.0, 0.0);
                    self.ui
                        .painter()
                        .line_segment([p1, p2], self.ui.visuals().widgets.noninteractive.bg_stroke);
                }
            }
        }

        // Add child markers to next dir if this one was flattened.
        if current_dir.flattened {
            if let Some(parent_dir) = self.stack.last_mut() {
                parent_dir
                    .child_node_positions
                    .extend(current_dir.child_node_positions);
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

        let (row, closer) = if self.parent_dir_is_open() && !node.flatten {
            node.set_is_open(open);
            self.node_internal(&mut node)
        } else {
            (Rect::NOTHING, Some(Rect::NOTHING))
        };

        self.result.new_node_states.push(NodeState {
            id: node.id,
            parent_id: self.parent_id(),
            open,
            visible: self.parent_dir_is_open() && !node.flatten,
            drop_allowed: node.drop_allowed,
            dir: node.is_dir,
            activatable: node.activatable,
        });
        self.result.row_rectangles.insert(
            node.id,
            RowRectangles {
                row_rect: row,
                closer_rect: closer,
            },
        );

        if node.is_dir {
            self.stack.push(DirectoryState {
                is_open: self.parent_dir_is_open() && open,
                id: node.id,
                icon_rect: closer.expect("Closer response should be availabel for dirs"),
                child_node_positions: Vec::new(),
                indent_level: if node.flatten {
                    self.get_indent_level()
                } else {
                    self.get_indent_level() + 1
                },
                flattened: node.flatten,
            });
        }
    }

    pub(crate) fn get_result(self) -> TreeViewBuilderResult<NodeIdType> {
        self.result
    }

    fn node_internal(&mut self, node: &mut NodeBuilder<NodeIdType>) -> (Rect, Option<Rect>) {
        let drag_overlay_rect = self.ui.available_rect_before_wrap();

        node.set_indent(self.get_indent_level());
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

        self.push_child_node_position(closer.or(icon).unwrap_or(label).left_center());

        (row, closer)
    }

    fn parent_dir(&self) -> Option<&DirectoryState<NodeIdType>> {
        if self.stack.is_empty() {
            None
        } else {
            self.stack.last()
        }
    }
    fn parent_dir_is_open(&self) -> bool {
        self.parent_dir().is_none_or(|dir| dir.is_open)
    }

    fn push_child_node_position(&mut self, pos: Pos2) {
        if let Some(parent_dir) = self.stack.last_mut() {
            parent_dir.child_node_positions.push(pos);
        }
    }
    fn get_indent_level(&self) -> usize {
        self.stack.last().map(|d| d.indent_level).unwrap_or(0)
    }
}
