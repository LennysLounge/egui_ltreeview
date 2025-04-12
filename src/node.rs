use egui::{
    emath, epaint, remap, vec2, CursorIcon, Id, InnerResponse, Label, LayerId, Rect, Response,
    Shape, Stroke, Ui, UiBuilder, Vec2, WidgetText,
};

use crate::{NodeId, RowLayout, TreeViewSettings};

/// A builder to build a node.
pub struct NodeBuilder<'add_ui, NodeIdType> {
    pub(crate) id: NodeIdType,
    pub(crate) is_dir: bool,
    pub(crate) flatten: bool,
    pub(crate) is_open: bool,
    pub(crate) default_open: bool,
    pub(crate) drop_allowed: bool,
    pub(crate) activatable: bool,
    indent: usize,
    #[allow(clippy::type_complexity)]
    icon: Option<Box<dyn FnMut(&mut Ui) + 'add_ui>>,
    #[allow(clippy::type_complexity)]
    closer: Option<Box<dyn FnMut(&mut Ui, CloserState) + 'add_ui>>,
    #[allow(clippy::type_complexity)]
    label: Option<Box<dyn FnMut(&mut Ui) + 'add_ui>>,
    #[allow(clippy::type_complexity)]
    context_menu: Option<Box<dyn FnMut(&mut Ui) + 'add_ui>>,
}
impl<'add_ui, NodeIdType: NodeId> NodeBuilder<'add_ui, NodeIdType> {
    /// Create a new node builder from a leaf prototype.
    pub fn leaf(id: NodeIdType) -> Self {
        Self {
            id,
            is_dir: false,
            flatten: false,
            drop_allowed: false,
            activatable: true,
            icon: None,
            closer: None,
            label: None,
            context_menu: None,
            is_open: false,
            default_open: true,
            indent: 0,
        }
    }

    /// Create a new node builder from a directory prorotype.
    pub fn dir(id: NodeIdType) -> Self {
        Self {
            id,
            is_dir: true,
            flatten: false,
            drop_allowed: true,
            activatable: false,
            icon: None,
            closer: None,
            label: None,
            context_menu: None,
            is_open: false,
            default_open: true,
            indent: 0,
        }
    }

    /// Whether or not the directory should be flattened into the parent directiron.
    ///
    /// A directory that is flattened is not visible in the tree and cannot be navigated to.
    /// Its children appear like the children of the grand parent directory.
    ///
    /// This node will still appear in [`Action::SetSelected`](crate::Action) if it is part of a relevant
    /// multi selection process.
    /// This node will still be the target of any [`drag and drop action`](crate::Action) as if it was visible.
    pub fn flatten(mut self, flatten: bool) -> Self {
        self.flatten = flatten;
        self
    }

    /// Whether or not a directory should be open by default or closed.
    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    /// Whether or not dropping onto this node is allowed.
    pub fn drop_allowed(mut self, drop_allowed: bool) -> Self {
        self.drop_allowed = drop_allowed;
        self
    }

    /// Whether or not this node can be activated.
    pub fn activatable(mut self, activatable: bool) -> Self {
        self.activatable = activatable;
        self
    }

    /// Add a icon to the node.
    pub fn icon(
        mut self,
        add_icon: impl FnMut(&mut Ui) + 'add_ui,
    ) -> NodeBuilder<'add_ui, NodeIdType> {
        self.icon = Some(Box::new(add_icon));
        self
    }

    /// Add a custom closer to the directory node.
    /// Leaf nodes do not show a closer.
    pub fn closer(
        mut self,
        add_closer: impl FnMut(&mut Ui, CloserState) + 'add_ui,
    ) -> NodeBuilder<'add_ui, NodeIdType> {
        self.closer = Some(Box::new(add_closer));
        self
    }

    /// Add a label to this node.
    pub fn label_ui(
        mut self,
        add_label: impl FnMut(&mut Ui) + 'add_ui,
    ) -> NodeBuilder<'add_ui, NodeIdType> {
        self.label = Some(Box::new(add_label));
        self
    }

    /// Add a label to this node from a `WidgetText`.
    pub fn label(self, text: impl Into<WidgetText> + 'add_ui) -> Self {
        let widget_text = text.into();
        self.label_ui(move |ui| {
            ui.add(Label::new(widget_text.clone()).selectable(false));
        })
    }

    /// Add a context menu to this node.
    ///
    /// A context menu in egui gets its size the first time it becomes visible.
    /// Since all nodes in the tree view share the same context menu you must set
    /// the size of the context menu manually for each node if you want to have differently
    /// sized context menus.
    pub fn context_menu(
        mut self,
        add_context_menu: impl FnMut(&mut Ui) + 'add_ui,
    ) -> NodeBuilder<'add_ui, NodeIdType> {
        self.context_menu = Some(Box::new(add_context_menu));
        self
    }

    pub(crate) fn set_is_open(&mut self, open: bool) {
        self.is_open = open;
    }

    pub(crate) fn set_indent(&mut self, indent: usize) {
        self.indent = indent;
    }

    pub(crate) fn show_node(
        &mut self,
        ui: &mut Ui,
        interaction: &Response,
        settings: &TreeViewSettings,
    ) -> (Rect, Option<Rect>, Option<Rect>, Rect) {
        let (reserve_closer, draw_closer, reserve_icon, draw_icon) = match settings.row_layout {
            RowLayout::Compact => (self.is_dir, self.is_dir, false, false),
            RowLayout::CompactAlignedLabels => (
                self.is_dir,
                self.is_dir,
                !self.is_dir,
                !self.is_dir && self.icon.is_some(),
            ),
            RowLayout::AlignedIcons => {
                (true, self.is_dir, self.icon.is_some(), self.icon.is_some())
            }
            RowLayout::AlignedIconsAndLabels => (true, self.is_dir, true, self.icon.is_some()),
        };

        let InnerResponse {
            inner: (closer, icon, label),
            response: row_response,
        } = ui.horizontal(|ui| {
            // The layouting in the row has to be pretty tight so we tunr of the item spacing here.
            let original_item_spacing = ui.spacing().item_spacing;
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            ui.add_space(original_item_spacing.x);

            // Add a little space so the closer/icon/label doesnt touch the left side
            // and add the indentation space.
            ui.add_space(ui.spacing().item_spacing.x);
            ui.add_space(
                self.indent as f32 * settings.override_indent.unwrap_or(ui.spacing().indent),
            );

            // Draw the closer
            let closer = draw_closer.then(|| {
                let (small_rect, big_rect) = ui
                    .spacing()
                    .icon_rectangles(ui.available_rect_before_wrap());

                let res = ui.allocate_new_ui(UiBuilder::new().max_rect(big_rect), |ui| {
                    let is_hovered = interaction
                        .hover_pos()
                        .is_some_and(|pos| ui.max_rect().contains(pos));
                    if is_hovered {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if let Some(add_closer) = self.closer.as_mut() {
                        (add_closer)(
                            ui,
                            CloserState {
                                is_open: self.is_open,
                                is_hovered,
                            },
                        );
                    } else {
                        let icon_id = Id::new(self.id).with("tree view closer icon");
                        let openness = ui.ctx().animate_bool(icon_id, self.is_open);
                        paint_default_icon(ui, openness, &small_rect, is_hovered);
                    }
                    ui.allocate_space(ui.available_size_before_wrap());
                });
                res.response.rect
            });
            if closer.is_none() && reserve_closer {
                ui.add_space(ui.spacing().icon_width);
            }

            // Draw icon
            let icon = draw_icon
                .then(|| {
                    self.icon.as_mut().map(|add_icon| {
                        let (_, big_rect) = ui
                            .spacing()
                            .icon_rectangles(ui.available_rect_before_wrap());
                        ui.allocate_new_ui(UiBuilder::new().max_rect(big_rect), |ui| {
                            ui.set_min_size(big_rect.size());
                            add_icon(ui);
                        })
                        .response
                        .rect
                    })
                })
                .flatten();
            if icon.is_none() && reserve_icon {
                ui.add_space(ui.spacing().icon_width);
            }

            ui.add_space(2.0);
            // Draw label
            let label = ui
                .scope(|ui| {
                    ui.spacing_mut().item_spacing = original_item_spacing;
                    if let Some(add_label) = self.label.as_mut() {
                        add_label(ui);
                    }
                })
                .response
                .rect;

            ui.add_space(original_item_spacing.x);

            (closer, icon, label)
        });

        let mut row = row_response
            .rect
            .expand2(vec2(0.0, ui.spacing().item_spacing.y * 0.5));
        row.set_width(ui.available_width());

        (row, closer, icon, label)
    }

    /// Draw the content as a drag overlay if it is beeing dragged.
    pub(crate) fn show_node_dragged(
        &mut self,
        ui: &mut Ui,
        interaction: &Response,
        settings: &TreeViewSettings,
        drag_layer: LayerId,
        target_rect: Rect,
    ) {
        ui.new_child(UiBuilder::new().max_rect(target_rect).layout(*ui.layout()))
            .scope_builder(UiBuilder::new().layer_id(drag_layer), |ui| {
                let background_position = ui.painter().add(Shape::Noop);

                let (row, _, _, _) = self.show_node(ui, interaction, settings);

                ui.painter().set(
                    background_position,
                    epaint::RectShape::new(
                        row,
                        ui.visuals().widgets.active.corner_radius,
                        ui.visuals().selection.bg_fill.linear_multiply(0.4),
                        Stroke::NONE,
                        egui::StrokeKind::Inside,
                    ),
                );
                row
            });
    }

    pub(crate) fn show_context_menu(&mut self, response: &Response) -> bool {
        if let Some(context_menu) = self.context_menu.as_mut() {
            let mut was_open = false;
            response.context_menu(|ui| {
                context_menu(ui);
                was_open = true;
            });
            was_open
        } else {
            false
        }
    }
}

/// Paint the arrow icon that indicated if the region is open or not
pub(crate) fn paint_default_icon(ui: &mut Ui, openness: f32, rect: &Rect, is_hovered: bool) {
    let visuals = if is_hovered {
        ui.visuals().widgets.hovered
    } else {
        ui.visuals().widgets.inactive
    };

    // Draw a pointy triangle arrow:
    let rect = Rect::from_center_size(rect.center(), vec2(rect.width(), rect.height()) * 0.75);
    let rect = rect.expand(visuals.expansion);
    let mut points = vec![rect.left_top(), rect.right_top(), rect.center_bottom()];
    use std::f32::consts::TAU;
    let rotation = emath::Rot2::from_angle(remap(openness, 0.0..=1.0, -TAU / 4.0..=0.0));
    for p in &mut points {
        *p = rect.center() + rotation * (*p - rect.center());
    }

    ui.painter().add(Shape::convex_polygon(
        points,
        visuals.fg_stroke.color,
        Stroke::NONE,
    ));
}

/// State of the closer when it is drawn.
pub struct CloserState {
    /// Wether the current directory this closer represents is currently open or closed.
    pub is_open: bool,
    /// Wether the pointer is hovering over the closer.
    pub is_hovered: bool,
}
