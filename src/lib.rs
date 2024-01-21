use egui::{
    self,
    epaint::{self, RectShape},
    layers::ShapeIdx,
    util::id_type_map::SerializableAny,
    vec2, CursorIcon, Id, InnerResponse, LayerId, Layout, Order, PointerButton, Pos2, Rangef, Rect,
    Response, Sense, Shape, Stroke, Ui, Vec2,
};

/// Contains information about a drag and drop that the
/// tree view produced.
#[derive(Debug)]
pub struct DragDropAction<NodeIdType> {
    pub drag_id: NodeIdType,
    /// Id of the dragged node.
    /// Id of the drop node where the dragged node is added to.
    pub drop_id: NodeIdType,
    /// Position of the dragged node in the drop node.
    pub position: DropPosition<NodeIdType>,
}

/// Where a dragged item should be dropped to in a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition<NodeIdType> {
    First,
    Last,
    After(NodeIdType),
    Before(NodeIdType),
}

#[derive(Clone)]
struct TreeViewBuilderState<NodeIdType> {
    // Id of the node that was selected last frame.
    selected: Option<NodeIdType>,
    // True if something was dragged last frame.
    was_dragged_last_frame: bool,
}
impl<NodeIdType> Default for TreeViewBuilderState<NodeIdType> {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            was_dragged_last_frame: Default::default(),
        }
    }
}

#[derive(Clone)]
struct DirectoryState<NodeIdType> {
    /// Id of the directory node.
    id: NodeIdType,
    /// If directory is expanded
    is_open: bool,
    /// If a directory is dragged, dropping is disallowed for any of
    /// its child nodes.
    drop_forbidden: bool,
    /// The rectangle of the row.
    row_rect: Rect,
    /// The rectangle of the icon.
    icon_rect: Rect,
}
pub struct TreeViewBuilder<'a, NodeIdType> {
    ui: &'a mut Ui,
    selected: &'a mut Option<NodeIdType>,
    drag: &'a mut Option<NodeIdType>,
    drop: &'a mut Option<(NodeIdType, DropPosition<NodeIdType>)>,
    stack: Vec<DirectoryState<NodeIdType>>,
    background_idx: ShapeIdx,
    drop_marker_idx: ShapeIdx,
    was_dragged_last_frame: bool,
}

impl<'a, NodeIdType> TreeViewBuilder<'a, NodeIdType>
where
    NodeIdType: Clone + Copy + Send + Sync + std::hash::Hash + PartialEq + 'static,
{
    pub fn new(
        ui: &mut Ui,
        base_id: Id,
        mut add_content: impl FnMut(TreeViewBuilder<'_, NodeIdType>),
    ) -> TreeViewResponse<NodeIdType> {
        let mut state = load(ui, base_id).unwrap_or(TreeViewBuilderState::default());
        let mut drag = None;
        let mut drop = None;
        let background_idx = ui.painter().add(Shape::Noop);
        let drop_marker_idx = ui.painter().add(Shape::Noop);

        let res = ui.allocate_ui_with_layout(
            ui.available_size_before_wrap(),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.add_space(ui.spacing().item_spacing.y * 0.5);
                add_content(TreeViewBuilder {
                    ui,
                    selected: &mut state.selected,
                    drag: &mut drag,
                    drop: &mut drop,
                    stack: Vec::new(),
                    background_idx,
                    drop_marker_idx,
                    was_dragged_last_frame: state.was_dragged_last_frame,
                });
                // Add negative space because the place will add the item spacing on top of this.
                ui.add_space(-ui.spacing().item_spacing.y * 0.5);
                ui.min_rect()
            },
        );

        let drag_drop_action =
            drag.zip(drop)
                .map(|(drag_id, (drop_id, position))| DragDropAction {
                    drag_id,
                    drop_id,
                    position,
                });
        let dropped = ui.ctx().input(|i| i.pointer.any_released()) && drag_drop_action.is_some();
        let selected_node = state.selected;

        state.was_dragged_last_frame = drag.is_some();
        store(ui, base_id, state);

        TreeViewResponse {
            response: res.response,
            dropped,
            drag_drop_action,
            _id: base_id,
            drop_marker_idx,
            selected_node,
        }
    }

    pub fn leaf(
        &mut self,
        id: &NodeIdType,
        mut add_content: impl FnMut(&mut Ui),
    ) -> Option<Response> {
        if !self.parent_dir_is_open() {
            return None;
        }

        let mut row_config = Row {
            id: *id,
            drop_on_allowed: false,
            is_open: false,
            add_content: &mut add_content,
            add_icon: None,
            depth: self.stack.len(),
        };
        let row_response = self.row(&mut row_config);

        Some(row_response.interaction)
    }

    pub fn dir(
        &mut self,
        id: &NodeIdType,
        mut add_content: impl FnMut(&mut Ui),
    ) -> Option<Response> {
        if !self.parent_dir_is_open() {
            self.stack.push(DirectoryState {
                is_open: false,
                id: *id,
                drop_forbidden: true,
                row_rect: Rect::NOTHING,
                icon_rect: Rect::NOTHING,
            });
            return None;
        }

        let dir_id = self.ui.id().with(id).with("dir");
        let mut open = load(self.ui, dir_id).unwrap_or(true);

        let mut add_icon = |ui: &mut Ui| {
            let icon_id = ui.make_persistent_id(id).with("icon");
            let openness = ui.ctx().animate_bool(icon_id, open);
            let icon_res = ui.allocate_rect(ui.max_rect(), Sense::click());
            egui::collapsing_header::paint_default_icon(ui, openness, &icon_res);
            icon_res
        };

        let mut node_config = Row {
            id: *id,
            drop_on_allowed: true,
            is_open: open,
            add_content: &mut add_content,
            add_icon: Some(&mut add_icon),
            depth: self.stack.len(),
        };

        let RowResponse {
            interaction,
            visual,
            icon,
            ..
        } = self.row(&mut node_config);

        if interaction.double_clicked() {
            open = !open;
        }

        let icon = icon.expect("Icon response is not available");
        if icon.clicked() {
            open = !open;
            *self.selected = Some(*id);
        }

        self.ui.data_mut(|d| d.insert_persisted(dir_id, open));

        //self.stack.push(self.current_dir.clone());
        self.stack.push(DirectoryState {
            is_open: open,
            id: *id,
            drop_forbidden: self.parent_dir_drop_forbidden() || self.is_dragged(id),
            row_rect: visual.rect,
            icon_rect: icon.rect,
        });
        Some(interaction)
    }

    pub fn close_dir(&mut self) {
        if let Some(current_dir) = self.parent_dir() {
            if let Some((drop_parent, DropPosition::Last)) = &self.drop {
                if drop_parent == &current_dir.id {
                    let mut rect = current_dir.row_rect;
                    *rect.bottom_mut() =
                        self.ui.cursor().top() - self.ui.spacing().item_spacing.y * 0.5;
                    self.ui.painter().set(
                        self.drop_marker_idx,
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
                let mut p1 = current_dir.icon_rect.center_bottom();
                p1.y += self.ui.spacing().item_spacing.y;
                let mut p2 = p1.clone();
                p2.y = self.ui.cursor().min.y - self.ui.spacing().item_spacing.y;
                self.ui
                    .painter()
                    .line_segment([p1, p2], self.ui.visuals().widgets.noninteractive.bg_stroke);
            }
        }
        self.stack.pop();
    }

    fn row(&mut self, row_config: &mut Row<NodeIdType>) -> RowResponse {
        let row_response = row_config.row(self.ui);

        if row_response.interaction.clicked() {
            *self.selected = Some(row_config.id);
        }
        if self.is_selected(&row_config.id) {
            self.ui.painter().set(
                self.background_idx,
                epaint::RectShape::new(
                    row_response.visual.rect,
                    self.ui.visuals().widgets.active.rounding,
                    self.ui.visuals().selection.bg_fill,
                    Stroke::NONE,
                ),
            );
        }
        if row_response.was_dragged {
            *self.drag = Some(row_config.id);
        }
        self.do_drop(row_config, &row_response);
        row_response
    }

    fn do_drop(&mut self, row_config: &Row<NodeIdType>, row_response: &RowResponse) {
        let Some(drop_quarter) = &row_response.drop_quarter else {
            return;
        };
        if !self.ui.ctx().memory(|m| m.is_anything_being_dragged()) {
            return;
        }
        if !self.was_dragged_last_frame {
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

        let drop_position = self.get_drop_position(&row_config, drop_quarter);
        let shape = self.drop_marker_shape(&row_response.interaction, drop_position.as_ref());

        // It is allowed to drop itself `After´ or `Before` itself.
        // This however doesn't make sense and makes executing the command more
        // difficult for the caller.
        // Instead we display the markers only.
        if self.is_dragged(&row_config.id) {
            self.ui.painter().set(self.drop_marker_idx, shape);
            return;
        }

        *self.drop = drop_position;
        self.ui.painter().set(self.drop_marker_idx, shape);
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
        self.selected
            .as_ref()
            .is_some_and(|selected_id| selected_id == id)
    }

    fn is_dragged(&self, id: &NodeIdType) -> bool {
        self.drag.as_ref().is_some_and(|drag_id| drag_id == id)
    }
}

pub struct TreeViewResponse<NodeIdType> {
    pub response: Response,
    /// If a row was dragged in the tree this will contain information about
    /// who was dragged to who and at what position.
    pub drag_drop_action: Option<DragDropAction<NodeIdType>>,
    /// `true` if a drag and drop was performed
    pub dropped: bool,
    /// Id of the selected node.
    pub selected_node: Option<NodeIdType>,
    _id: Id,
    drop_marker_idx: ShapeIdx,
}
impl<NodeIdType> TreeViewResponse<NodeIdType> {
    /// Remove the drop marker from the tree view.
    pub fn remove_drop_marker(&self, ui: &mut Ui) {
        ui.painter().set(self.drop_marker_idx, Shape::Noop);
    }
}

struct Row<'a, NodeIdType> {
    id: NodeIdType,
    depth: usize,
    drop_on_allowed: bool,
    is_open: bool,
    add_content: &'a mut dyn FnMut(&mut Ui),
    add_icon: Option<&'a mut dyn FnMut(&mut Ui) -> Response>,
}

impl<NodeIdType> Row<'_, NodeIdType>
where
    NodeIdType: Clone + std::hash::Hash,
{
    fn row(&mut self, ui: &mut Ui) -> RowResponse {
        // Load row data
        let row_id = ui.id().with(self.id.clone()).with("row");
        let row_rect = load(ui, row_id).unwrap_or(Rect::NOTHING);

        // Interact with the row
        let interaction = interact(ui, row_rect, row_id, Sense::click_and_drag());

        let was_dragged = self.drag(ui, &interaction);
        let drop_target = self.drop(ui, &interaction);

        let (row_response, icon_response) = self.draw_row(ui);

        store(ui, row_id, row_response.rect);

        RowResponse {
            interaction,
            visual: row_response,
            icon: icon_response,
            was_dragged,
            drop_quarter: drop_target,
        }
    }
    /// Draw the content as a drag overlay if it is beeing dragged.
    fn drag(&mut self, ui: &mut Ui, interaction: &Response) -> bool {
        if !interaction.dragged_by(PointerButton::Primary)
            && !interaction.drag_released_by(PointerButton::Primary)
        {
            return false;
        }

        //*self.drag = Some(self.id);
        ui.ctx().set_cursor_icon(CursorIcon::Alias);

        let drag_source_id = ui.make_persistent_id("Drag source");
        let drag_offset = if interaction.drag_started_by(PointerButton::Primary) {
            let drag_offset = ui
                .ctx()
                .pointer_latest_pos()
                .map(|pointer_pos| interaction.rect.min - pointer_pos)
                .unwrap_or(Vec2::ZERO);
            store(ui, drag_source_id, drag_offset);
            drag_offset
        } else {
            load(ui, drag_source_id).unwrap_or(Vec2::ZERO)
        };

        // Paint the content to a new layer for the drag overlay.
        let layer_id = LayerId::new(Order::Tooltip, drag_source_id);

        let background_rect = ui
            .child_ui(ui.available_rect_before_wrap(), *ui.layout())
            .with_layer_id(layer_id, |ui| {
                let background_position = ui.painter().add(Shape::Noop);

                let (row, _) = self.draw_row(ui);

                ui.painter().set(
                    background_position,
                    epaint::RectShape::new(
                        row.rect,
                        ui.visuals().widgets.active.rounding,
                        ui.visuals().selection.bg_fill.linear_multiply(0.4),
                        Stroke::NONE,
                    ),
                );
                row.rect
            })
            .inner;

        // Move layer to the drag position
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            let delta = pointer_pos - background_rect.min + drag_offset;
            ui.ctx().translate_layer(layer_id, delta);
        }

        true
    }

    fn drop(&self, ui: &mut Ui, interaction: &Response) -> Option<DropQuarter> {
        // For some reason we cannot use the provided interation response
        // because once a row is dragged all other rows dont offer any hover information.
        // To fix this we interaction with only hover again.
        let cursor_y = {
            let Some(Pos2 { y, .. }) = interact(
                ui,
                interaction.rect,
                ui.make_persistent_id("Drop target"),
                Sense::hover(),
            )
            .hover_pos() else {
                return None;
            };
            y
        };

        DropQuarter::new(interaction.rect.y_range(), cursor_y)
    }

    fn draw_row(&mut self, ui: &mut Ui) -> (Response, Option<Response>) {
        let InnerResponse {
            inner: icon_response,
            response: row_response,
        } = ui.horizontal(|ui| {
            ui.add_space(ui.spacing().indent * self.depth as f32);

            let icon_pos = ui.cursor().min;
            if self.add_icon.is_some() {
                ui.add_space(ui.spacing().icon_width);
            };
            (self.add_content)(ui);
            ui.add_space(ui.available_width());

            self.add_icon.as_mut().map(|add_icon| {
                let (small_rect, _) = ui.spacing().icon_rectangles(Rect::from_min_size(
                    icon_pos,
                    vec2(ui.spacing().icon_width, ui.min_size().y),
                ));
                ui.allocate_ui_at_rect(small_rect, |ui| add_icon(ui)).inner
            })
        });

        let background_rect = row_response
            .rect
            .expand2(vec2(0.0, ui.spacing().item_spacing.y * 0.5));

        (row_response.with_new_rect(background_rect), icon_response)
    }
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

struct RowResponse {
    interaction: Response,
    visual: Response,
    icon: Option<Response>,
    was_dragged: bool,
    drop_quarter: Option<DropQuarter>,
}

fn load<T: SerializableAny>(ui: &mut Ui, id: Id) -> Option<T> {
    ui.data_mut(|d| d.get_persisted::<T>(id))
}

fn store<T: SerializableAny>(ui: &mut Ui, id: Id, value: T) {
    ui.data_mut(|d| d.insert_persisted(id, value));
}
/// Interact with the ui without egui adding any extra space.
fn interact(ui: &mut Ui, rect: Rect, id: Id, sense: Sense) -> Response {
    let spacing_before = ui.spacing().clone();
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let res = ui.interact(rect, id, sense);
    *ui.spacing_mut() = spacing_before;
    res
}
