mod row;

use egui::{
    self,
    epaint::{self, RectShape},
    layers::ShapeIdx,
    pos2,
    util::id_type_map::SerializableAny,
    vec2, Id, Layout, Pos2, Rangef, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};
use row::{DropQuarter, Row};

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

pub struct TreeView {
    id: Id,
    settings: TreeViewSettings,
}
impl TreeView {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            settings: TreeViewSettings::default(),
        }
    }

    /// Override the indent value from the current ui style with this value.
    ///
    /// If `None`, the value of the current ui style is used.
    /// Defaults to `None`.
    pub fn override_indent(mut self, indent: Option<f32>) -> Self {
        self.settings.override_indent = indent;
        self
    }

    /// Set the style of the vline to show the indentation level.
    pub fn vline_style(mut self, style: VLineStyle) -> Self {
        self.settings.vline_style = style;
        self
    }

    /// Set the row layout for this tree.
    pub fn row_layout(mut self, layout: RowLayout) -> Self {
        self.settings.row_layout = layout;
        self
    }

    /// Start displaying the tree view.
    ///
    /// Construct the tree view using the [`TreeViewBuilder`] by addind
    /// directories or leaves to the tree.
    pub fn show<NodeIdType>(
        self,
        ui: &mut Ui,
        mut build_tree_view: impl FnMut(TreeViewBuilder<'_, NodeIdType>),
    ) -> TreeViewResponse<NodeIdType>
    where
        NodeIdType:
            Clone + Copy + Send + Sync + std::hash::Hash + PartialEq + 'static + std::fmt::Debug,
    {
        let mut state = load(ui, self.id).unwrap_or(TreeViewState::default());
        let mut drop = None;
        let background_idx = ui.painter().add(Shape::Noop);
        let drop_marker_idx = ui.painter().add(Shape::Noop);

        ui.painter().rect_stroke(
            state.rect,
            egui::Rounding::ZERO,
            egui::Stroke::new(1.0, egui::Color32::BLACK),
        );

        let interaction = Interaction {
            response: interact(ui, state.rect, self.id, Sense::click_and_drag()),
            hover_pos: ui.ctx().pointer_hover_pos().unwrap_or_default(),
        };

        let res = ui.allocate_ui_with_layout(
            ui.available_size_before_wrap(),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.add_space(ui.spacing().item_spacing.y * 0.5);
                build_tree_view(TreeViewBuilder {
                    ui,
                    selected: &mut state.selected,
                    dragged: &mut state.dragged,
                    drop: &mut drop,
                    interaction: interaction.clone(),
                    stack: Vec::new(),
                    background_idx,
                    drop_marker_idx,
                    settings: self.settings,
                });
                // Add negative space because the place will add the item spacing on top of this.
                ui.add_space(-ui.spacing().item_spacing.y * 0.5);
                ui.min_rect()
            },
        );

        let drag_drop_action = state
            .dragged
            .zip(drop)
            .map(|(drag_id, (drop_id, position))| DragDropAction {
                drag_id,
                drop_id,
                position,
            });
        let dropped = ui.ctx().input(|i| i.pointer.any_released()) && drag_drop_action.is_some();
        let selected_node = state.selected;

        if interaction.response.drag_released() {
            state.dragged = None;
        }

        ui.label(format!("dragged: {:?}", state.dragged));

        state.rect = res.response.rect;
        store(ui, self.id, state);

        TreeViewResponse {
            response: res.response,
            dropped,
            drag_drop_action,
            drop_marker_idx,
            selected_node,
        }
    }
}

#[derive(Clone)]
struct TreeViewState<NodeIdType> {
    // Id of the node that was selected last frame.
    selected: Option<NodeIdType>,
    // True if something was dragged last frame.
    dragged: Option<NodeIdType>,
    // The rectangle the tree view occupied last frame.
    rect: Rect,
}
impl<NodeIdType> Default for TreeViewState<NodeIdType> {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            dragged: Default::default(),
            rect: Rect::NOTHING,
        }
    }
}

#[derive(Default)]
struct TreeViewSettings {
    override_indent: Option<f32>,
    vline_style: VLineStyle,
    row_layout: RowLayout,
}

/// Style of the vertical line to show the indentation level.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum VLineStyle {
    /// No vline is shown.
    None,
    /// A single vertical line is show for the full hight of the directory.
    VLine,
    /// A vline is show with horizontal hooks to the child nodes of the directory.
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

pub struct TreeViewResponse<NodeIdType> {
    pub response: Response,
    /// If a row was dragged in the tree this will contain information about
    /// who was dragged to who and at what position.
    pub drag_drop_action: Option<DragDropAction<NodeIdType>>,
    /// `true` if a drag and drop was performed
    pub dropped: bool,
    /// Id of the selected node.
    pub selected_node: Option<NodeIdType>,
    drop_marker_idx: ShapeIdx,
}
impl<NodeIdType> TreeViewResponse<NodeIdType> {
    /// Remove the drop marker from the tree view.
    ///
    /// Use this to remove the drop marker if a proposed drag and drop action
    /// is disallowed.
    pub fn remove_drop_marker(&self, ui: &mut Ui) {
        ui.painter().set(self.drop_marker_idx, Shape::Noop);
    }
}

#[derive(Clone)]
struct DirectoryState<NodeIdType> {
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

#[derive(Clone)]
struct Interaction {
    response: Response,
    hover_pos: Pos2,
}
impl Interaction {
    fn mouse_over(&self, rect: &Rect) -> bool {
        rect.contains(self.hover_pos)
    }

    fn clicked(&self, rect: &Rect) -> bool {
        self.response.clicked() && self.mouse_over(rect)
    }

    fn double_clicked(&self, rect: &Rect) -> bool {
        self.response.double_clicked() && self.mouse_over(rect)
    }

    fn drag_started(&self, rect: &Rect) -> bool {
        self.response.drag_started_by(egui::PointerButton::Primary) && self.mouse_over(rect)
    }

    fn drop_quarter(&self, rect: &Rect) -> Option<DropQuarter> {
        DropQuarter::new(rect.y_range(), self.hover_pos.y)
    }
}

/// The builder used to construct the tree view.
///
/// Use this to add directories or leaves to the tree.
pub struct TreeViewBuilder<'a, NodeIdType> {
    ui: &'a mut Ui,
    selected: &'a mut Option<NodeIdType>,
    dragged: &'a mut Option<NodeIdType>,
    drop: &'a mut Option<(NodeIdType, DropPosition<NodeIdType>)>,
    interaction: Interaction,
    stack: Vec<DirectoryState<NodeIdType>>,
    background_idx: ShapeIdx,
    drop_marker_idx: ShapeIdx,
    settings: TreeViewSettings,
}

impl<'a, NodeIdType> TreeViewBuilder<'a, NodeIdType>
where
    NodeIdType: Clone + Copy + Send + Sync + std::hash::Hash + PartialEq + 'static,
{
    pub fn leaf(&mut self, id: &NodeIdType, add_content: impl FnMut(&mut Ui)) -> Option<Response> {
        if !self.parent_dir_is_open() {
            return None;
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

        let _row_response = self.row(&row_config, add_content, None);
        None
    }

    pub fn dir(&mut self, id: &NodeIdType, add_content: impl FnMut(&mut Ui)) -> Option<Response> {
        if !self.parent_dir_is_open() {
            self.stack.push(DirectoryState {
                is_open: false,
                id: *id,
                drop_forbidden: true,
                row_rect: Rect::NOTHING,
                icon_rect: Rect::NOTHING,
                child_node_positions: Vec::new(),
            });
            return None;
        }

        let dir_id = self.ui.id().with(id).with("dir");
        let mut open = load(self.ui, dir_id).unwrap_or(true);

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

        if self.interaction.double_clicked(&row_response.rect) {
            open = !open;
        }

        if closer.clicked() {
            open = !open;
            *self.selected = Some(*id);
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
        None
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
        let (row_response, closer_response, label_rect) =
            row_config.draw_row(self.ui, &self.settings, &mut add_label, &mut add_icon);

        if self.interaction.clicked(&row_response.rect) {
            *self.selected = Some(row_config.id);
        }
        if self.is_selected(&row_config.id) {
            self.ui.painter().set(
                self.background_idx,
                epaint::RectShape::new(
                    row_response.rect,
                    self.ui.visuals().widgets.active.rounding,
                    self.ui.visuals().selection.bg_fill,
                    Stroke::NONE,
                ),
            );
        }
        if self.interaction.drag_started(&row_response.rect) {
            *self.dragged = Some(row_config.id);
        }
        if self.is_dragged(&row_config.id) {
            row_config.draw_row_dragged(
                self.ui,
                &self.settings,
                &self.interaction,
                &row_response,
                &mut add_label,
                &mut add_icon,
            );
        }
        if let Some(drop_quarter) = self.interaction.drop_quarter(&row_response.rect) {
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
        if self.dragged.is_none() {
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
        self.dragged.as_ref().is_some_and(|drag_id| drag_id == id)
    }

    fn push_child_node_position(&mut self, pos: Pos2) {
        if let Some(parent_dir) = self.stack.last_mut() {
            parent_dir.child_node_positions.push(pos);
        }
    }
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
