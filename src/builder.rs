use egui::{
    epaint::{self, RectShape},
    layers::ShapeIdx,
    pos2, vec2, Pos2, Rangef, Rect, Response, Shape, Stroke, Ui,
};

use crate::{
    row::{DropQuarter, Row},
    DropPosition, TreeViewSettings, TreeViewState, VLineStyle,
};

#[derive(Clone)]
pub(crate) struct DirectoryState<NodeIdType> {
    /// Id of the directory node.
    id: NodeIdType,
    /// If directory is expanded
    is_open: bool,
    /// Wether dropping on this or any of its child nodes is allowed.
    drop_forbidden: bool,
    /// The rectangle of the row.
    row_rect: Rect,
    /// The rectangle of the icon.
    icon_rect: Rect,
    /// Positions of each child node of this directory.
    child_node_positions: Vec<Pos2>,
}

/// The builder used to construct the tree view.
///
/// Use this to add directories or leaves to the tree.
pub struct TreeViewBuilder<'a, NodeIdType> {
    ui: &'a mut Ui,
    state: &'a mut TreeViewState<NodeIdType>,
    stack: Vec<DirectoryState<NodeIdType>>,
    background_idx: ShapeIdx,
    settings: &'a TreeViewSettings,
}

impl<'a, NodeIdType> TreeViewBuilder<'a, NodeIdType>
where
    NodeIdType: Clone + Copy + Send + Sync + std::hash::Hash + PartialEq + 'static,
{
    pub(crate) fn new(
        ui: &'a mut Ui,
        state: &'a mut TreeViewState<NodeIdType>,
        settings: &'a TreeViewSettings,
    ) -> Self {
        Self {
            background_idx: ui.painter().add(Shape::Noop),
            ui,
            state,
            stack: Vec::new(),
            settings,
        }
    }

    pub fn leaf(&mut self, id: &NodeIdType, add_label: impl FnMut(&mut Ui)) {
        if !self.parent_dir_is_open() {
            return;
        }

        let row_config = Row {
            id: *id,
            drop_on_allowed: false,
            is_open: false,
            is_dir: false,
            depth: self.stack.len() as f32
                * self
                    .settings
                    .override_indent
                    .unwrap_or(self.ui.spacing().indent),
        };
        self.row(&row_config, add_label, None);
    }

    pub fn dir(&mut self, id: &NodeIdType, add_content: impl FnMut(&mut Ui)) {
        if !self.parent_dir_is_open() {
            self.stack.push(DirectoryState {
                is_open: false,
                id: *id,
                drop_forbidden: true,
                row_rect: Rect::NOTHING,
                icon_rect: Rect::NOTHING,
                child_node_positions: Vec::new(),
            });
            return;
        }

        let dir_id = self.ui.id().with(id).with("dir");
        let mut open = crate::load(self.ui, dir_id).unwrap_or(true);

        let row_config = Row {
            id: *id,
            drop_on_allowed: true,
            is_open: open,
            is_dir: true,
            depth: self.stack.len() as f32
                * self
                    .settings
                    .override_indent
                    .unwrap_or(self.ui.spacing().indent),
        };

        let (row_response, closer_response) = self.row(&row_config, add_content, None);
        let closer = closer_response.expect("Closer response should be availabel for dirs");

        let row_interaction = self.state.interact(&row_response.rect);
        if row_interaction.double_clicked {
            open = !open;
        }

        let closer_interaction = self.state.interact(&closer.rect);
        if closer_interaction.clicked {
            open = !open;
            self.state.selected = Some(*id);
        }

        self.ui.data_mut(|d| d.insert_persisted(dir_id, open));

        //self.stack.push(self.current_dir.clone());
        self.stack.push(DirectoryState {
            is_open: open,
            id: *id,
            drop_forbidden: self.parent_dir_drop_forbidden() || self.is_dragged(id),
            row_rect: row_response.rect,
            icon_rect: closer.rect,
            child_node_positions: Vec::new(),
        });
    }

    pub fn close_dir(&mut self) {
        if let Some(current_dir) = self.parent_dir() {
            if let Some((drop_parent, DropPosition::Last)) = &self.state.drop {
                if drop_parent == &current_dir.id {
                    let mut rect = current_dir.row_rect;
                    *rect.bottom_mut() =
                        self.ui.cursor().top() - self.ui.spacing().item_spacing.y * 0.5;
                    self.ui.painter().set(
                        self.state.drop_marker_idx,
                        RectShape::new(
                            rect,
                            self.ui.visuals().widgets.active.rounding,
                            self.ui.visuals().selection.bg_fill.linear_multiply(0.5),
                            Stroke::NONE,
                        ),
                    );
                }
            }
        }

        if let Some(current_dir) = self.parent_dir() {
            if current_dir.is_open {
                let top = current_dir.icon_rect.center_bottom()
                    + vec2(0.0, self.ui.spacing().item_spacing.y);

                let bottom = match self.settings.vline_style {
                    VLineStyle::None => top.clone(),
                    VLineStyle::VLine => pos2(
                        top.x,
                        self.ui.cursor().min.y - self.ui.spacing().item_spacing.y,
                    ),
                    VLineStyle::Hook => pos2(
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
                if matches!(self.settings.vline_style, VLineStyle::Hook) {
                    for child_pos in current_dir.child_node_positions.iter() {
                        let p1 = pos2(top.x, child_pos.y);
                        let p2 = *child_pos;
                        self.ui.painter().line_segment(
                            [p1, p2],
                            self.ui.visuals().widgets.noninteractive.bg_stroke,
                        );
                    }
                }
            }
        }
        self.stack.pop();
    }

    fn row(
        &mut self,
        row_config: &Row<NodeIdType>,
        mut add_label: impl FnMut(&mut Ui),
        mut add_icon: Option<&mut dyn FnMut(&mut Ui)>,
    ) -> (Response, Option<Response>) {
        let (row_response, closer_response, label_rect) = row_config.draw_row(
            self.ui,
            &self.state,
            &self.settings,
            &mut add_label,
            &mut add_icon,
        );

        let row_interaction = self.state.interact(&row_response.rect);

        if row_interaction.clicked {
            self.state.selected = Some(row_config.id);
        }
        if self.is_selected(&row_config.id) {
            self.ui.painter().set(
                self.background_idx,
                epaint::RectShape::new(
                    row_response.rect,
                    self.ui.visuals().widgets.active.rounding,
                    if self.state.has_focus {
                        self.ui.visuals().selection.bg_fill
                    } else {
                        self.ui.visuals().widgets.inactive.weak_bg_fill
                    },
                    Stroke::NONE,
                ),
            );
        }
        if row_interaction.right_clicked {
            self.state.context_menu_node = Some(row_config.id);
        }
        if row_interaction.drag_started {
            self.state.dragged = Some(row_config.id);
        }
        if self.is_dragged(&row_config.id) {
            row_config.draw_row_dragged(
                self.ui,
                &self.settings,
                &self.state,
                &row_response,
                &mut add_label,
                &mut add_icon,
            );
        }
        if let Some(drop_quarter) = self
            .state
            .response
            .hover_pos()
            .and_then(|pos| DropQuarter::new(row_response.rect.y_range(), pos.y))
        {
            self.do_drop(&row_config, &row_response, drop_quarter);
        }

        self.push_child_node_position(label_rect.left_center());

        (row_response, closer_response)
    }

    fn do_drop(
        &mut self,
        row_config: &Row<NodeIdType>,
        row_response: &Response,
        drop_quarter: DropQuarter,
    ) {
        if !self.ui.ctx().memory(|m| m.is_anything_being_dragged()) {
            return;
        }
        if self.state.dragged.is_none() {
            return;
        }
        if self.parent_dir_drop_forbidden() {
            return;
        }
        // For dirs and for nodes that allow dropping on them, it is not
        // allowed to drop itself onto itself.
        if self.is_dragged(&row_config.id) && row_config.drop_on_allowed {
            return;
        }

        let drop_position = self.get_drop_position(&row_config, &drop_quarter);
        let shape = self.drop_marker_shape(&row_response, drop_position.as_ref());

        // It is allowed to drop itself `After´ or `Before` itself.
        // This however doesn't make sense and makes executing the command more
        // difficult for the caller.
        // Instead we display the markers only.
        if self.is_dragged(&row_config.id) {
            self.ui.painter().set(self.state.drop_marker_idx, shape);
            return;
        }

        self.state.drop = drop_position;
        self.ui.painter().set(self.state.drop_marker_idx, shape);
    }

    fn get_drop_position(
        &self,
        node_config: &Row<NodeIdType>,
        drop_quater: &DropQuarter,
    ) -> Option<(NodeIdType, DropPosition<NodeIdType>)> {
        let Row {
            id,
            drop_on_allowed,
            is_open,
            ..
        } = node_config;

        match drop_quater {
            DropQuarter::Top => {
                if let Some(parent_dir) = self.parent_dir() {
                    return Some((parent_dir.id, DropPosition::Before(*id)));
                }
                if *drop_on_allowed {
                    return Some((*id, DropPosition::Last));
                }
                return None;
            }
            DropQuarter::MiddleTop => {
                if *drop_on_allowed {
                    return Some((*id, DropPosition::Last));
                }
                if let Some(parent_dir) = self.parent_dir() {
                    return Some((parent_dir.id, DropPosition::Before(*id)));
                }
                return None;
            }
            DropQuarter::MiddleBottom => {
                if *drop_on_allowed {
                    return Some((*id, DropPosition::Last));
                }
                if let Some(parent_dir) = self.parent_dir() {
                    return Some((parent_dir.id, DropPosition::After(*id)));
                }
                return None;
            }
            DropQuarter::Bottom => {
                if *drop_on_allowed && *is_open {
                    return Some((*id, DropPosition::First));
                }
                if let Some(parent_dir) = self.parent_dir() {
                    return Some((parent_dir.id, DropPosition::After(*id)));
                }
                if *drop_on_allowed {
                    return Some((*id, DropPosition::Last));
                }
                return None;
            }
        }
    }

    fn drop_marker_shape(
        &self,
        interaction: &Response,
        drop_position: Option<&(NodeIdType, DropPosition<NodeIdType>)>,
    ) -> Shape {
        pub const DROP_LINE_HEIGHT: f32 = 3.0;

        let drop_marker = match drop_position {
            Some((_, DropPosition::Before(_))) => {
                Rangef::point(interaction.rect.min.y).expand(DROP_LINE_HEIGHT * 0.5)
            }
            Some((_, DropPosition::First)) | Some((_, DropPosition::After(_))) => {
                Rangef::point(interaction.rect.max.y).expand(DROP_LINE_HEIGHT * 0.5)
            }
            Some((_, DropPosition::Last)) => interaction.rect.y_range(),
            None => return Shape::Noop,
        };

        epaint::RectShape::new(
            Rect::from_x_y_ranges(interaction.rect.x_range(), drop_marker),
            self.ui.visuals().widgets.active.rounding,
            self.ui
                .style()
                .visuals
                .selection
                .bg_fill
                .linear_multiply(0.6),
            Stroke::NONE,
        )
        .into()
    }

    fn parent_dir(&self) -> Option<&DirectoryState<NodeIdType>> {
        if self.stack.is_empty() {
            None
        } else {
            self.stack.last()
        }
    }
    fn parent_dir_is_open(&self) -> bool {
        self.parent_dir().map_or(true, |dir| dir.is_open)
    }

    fn parent_dir_drop_forbidden(&self) -> bool {
        self.parent_dir().is_some_and(|dir| dir.drop_forbidden)
    }

    fn is_selected(&self, id: &NodeIdType) -> bool {
        self.state
            .selected
            .as_ref()
            .is_some_and(|selected_id| selected_id == id)
    }

    fn is_dragged(&self, id: &NodeIdType) -> bool {
        self.state
            .dragged
            .as_ref()
            .is_some_and(|drag_id| drag_id == id)
    }

    fn push_child_node_position(&mut self, pos: Pos2) {
        if let Some(parent_dir) = self.stack.last_mut() {
            parent_dir.child_node_positions.push(pos);
        }
    }
}
