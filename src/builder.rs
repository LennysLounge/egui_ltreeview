use std::collections::HashMap;

use egui::{
    epaint::{self},
    layers::ShapeIdx,
    pos2, vec2, Pos2, Rect, Response, Shape, Stroke, Ui, WidgetText,
};

use crate::{
    node::NodeBuilder, IndentHintStyle, NodeState, TreeViewId, TreeViewSettings, TreeViewState,
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
    pub(crate) background_idx: HashMap<NodeIdType, ShapeIdx>,
    pub(crate) background_idx_backup: ShapeIdx,
    pub(crate) secondary_selection_idx: ShapeIdx,
    pub(crate) new_node_states: Vec<NodeState<NodeIdType>>,
    pub(crate) row_rectangles: HashMap<NodeIdType, RowRectangles>,
    pub(crate) drop_marker_idx: ShapeIdx,
}

pub(crate) struct RowRectangles {
    pub(crate) row_rect: Rect,
    pub(crate) closer_rect: Option<Rect>,
}

/// The builder used to construct the tree view.
///
/// Use this to add directories or leaves to the tree.
pub struct TreeViewBuilder<'ui, NodeIdType> {
    ui: &'ui mut Ui,
    state: &'ui TreeViewState<NodeIdType>,
    settings: &'ui TreeViewSettings,
    interaction: &'ui Response,
    stack: Vec<DirectoryState<NodeIdType>>,
    tree_has_focus: bool,
    result: TreeViewBuilderResult<NodeIdType>,
}

impl<'ui, NodeIdType: TreeViewId> TreeViewBuilder<'ui, NodeIdType> {
    pub(crate) fn new(
        ui: &'ui mut Ui,
        interaction: &'ui Response,
        state: &'ui mut TreeViewState<NodeIdType>,
        settings: &'ui TreeViewSettings,
        tree_has_focus: bool,
    ) -> Self {
        let mut background_indices = HashMap::new();
        state.node_states.iter().for_each(|ns| {
            background_indices.insert(ns.id, ui.painter().add(Shape::Noop));
        });

        Self {
            result: TreeViewBuilderResult {
                background_idx: background_indices,
                background_idx_backup: ui.painter().add(Shape::Noop),
                secondary_selection_idx: ui.painter().add(Shape::Noop),
                new_node_states: Vec::new(),
                drop_marker_idx: ui.painter().add(Shape::Noop),
                row_rectangles: HashMap::new(),
            },
            ui,
            interaction,
            state,
            stack: Vec::new(),
            settings,
            tree_has_focus,
        }
    }

    /// Get the current parent id if any.
    pub fn parent_id(&self) -> Option<NodeIdType> {
        self.parent_dir().map(|state| state.id)
    }

    /// Add a leaf to the tree.
    pub fn leaf(&mut self, id: NodeIdType, label: impl Into<WidgetText>) {
        let widget_text = label.into();
        self.node(NodeBuilder::leaf(id).label(|ui| {
            ui.add(egui::Label::new(widget_text.clone()).selectable(false));
        }));
    }

    /// Add a directory to the tree.
    /// Must call [Self::close_dir] to close the directory.
    pub fn dir(&mut self, id: NodeIdType, label: impl Into<WidgetText>) {
        let widget_text = label.into();
        self.node(NodeBuilder::dir(id).label(|ui| {
            ui.add(egui::Label::new(widget_text.clone()).selectable(false));
        }));
    }

    pub(crate) fn get_result(self) -> TreeViewBuilderResult<NodeIdType> {
        self.result
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

    fn node_internal(&mut self, node: &mut NodeBuilder<NodeIdType>) -> (Rect, Option<Rect>) {
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

                node.show_node(ui, self.interaction, self.settings)
            })
            .inner;

        if self.state.is_selected(&node.id) {
            self.ui.painter().set(
                *self
                    .result
                    .background_idx
                    .get(&node.id)
                    .unwrap_or(&self.result.background_idx_backup),
                epaint::RectShape::new(
                    row,
                    self.ui.visuals().widgets.active.corner_radius,
                    if self.tree_has_focus {
                        self.ui.visuals().selection.bg_fill
                    } else {
                        self.ui
                            .visuals()
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

        if self.state.is_dragged(&node.id) {
            node.show_node_dragged(self.ui, self.interaction, self.state, self.settings);
        }

        if self.state.is_secondary_selected(&node.id) {
            let context_menu_visible = node.show_context_menu(self.interaction);

            if !self.state.is_selected(&node.id) && context_menu_visible {
                self.ui.painter().set(
                    self.result.secondary_selection_idx,
                    epaint::RectShape::new(
                        row,
                        self.ui.visuals().widgets.active.corner_radius,
                        egui::Color32::TRANSPARENT,
                        self.ui.visuals().widgets.inactive.fg_stroke,
                        egui::StrokeKind::Inside,
                    ),
                );
            }
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
