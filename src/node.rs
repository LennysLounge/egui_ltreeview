use egui::{
    emath, remap, vec2, CursorIcon, Id, Label, Layout, Rect, Response, Shape, Stroke, Ui,
    UiBuilder, Vec2, WidgetText,
};

use crate::{NodeId, RowLayout, TreeViewSettings};

/// Used to configure the appearance and behavior of a node in the tree.
///
/// Implementing this trait is not necessary most of the time. The [`NodeBuilder`]
/// implements this trait and can be used for most purposes.
pub trait NodeConfig<NodeIdType> {
    /// Returns the id of this node
    fn id(&self) -> &NodeIdType;
    /// Returns whether or not this node is a directory.
    fn is_dir(&self) -> bool;
    /// Renders the label of this node
    fn label(&mut self, ui: &mut Ui);
    /// Whether or not the directory should be flattened into the parent directiron.
    ///
    /// A directory that is flattened is not visible in the tree and cannot be navigated to.
    /// Its children appear like the children of the grand parent directory.
    ///
    /// For example, this file structure:
    /// ```text
    /// Foo
    /// ├─ Alice
    /// ├─ Bar
    /// │  ├─ Bob
    /// │  └─ Clair
    /// └─ Denis
    /// ```
    /// looks like this when the `Bar` directory is flattened:
    /// ```text
    /// Foo
    /// ├─ Alice
    /// ├─ Bob
    /// ├─ Clair
    /// └─ Denis
    /// ```
    ///
    /// This node (`Bar` in the example) will still appear in [`Action::SetSelected`](crate::Action) if it is part of a relevant
    /// multi selection process.
    /// This node will still be the target of any [`drag and drop action`](crate::Action) as if it was visible.
    ///
    /// Default value is false. Override to customize.
    fn flatten(&self) -> bool {
        false
    }
    /// Whether or not a directory should be open by default or closed.
    ///
    /// Default is true. Override to customize.
    fn default_open(&self) -> bool {
        true
    }
    /// Whether or not dropping onto this node is allowed.
    ///
    /// Default is true for directories and false otherwise. Override to customize.
    fn drop_allowed(&self) -> bool {
        self.is_dir()
    }
    /// Whether or not this node can be activated.
    ///
    /// Default is false for directories and true otherwise. Override to customize.
    fn activatable(&self) -> bool {
        !self.is_dir()
    }
    /// The height of this node. If `None` the default height of the
    /// [`TreeViewSettings`](`TreeViewSettings::default_node_height`) is used.
    ///
    /// Default is `None`. Override to customize.
    fn node_height(&self) -> Option<f32> {
        None
    }
    /// Whether or not this node has a custom icon.
    ///
    /// Default is false. Override to customize.
    fn has_custom_icon(&self) -> bool {
        false
    }
    /// If [`has_custom_icon`](`NodeConfig::has_custom_icon`) returns true, this method is used to render the custom icon.
    ///
    /// Default does nothing. Override to customize.
    #[allow(unused)]
    fn icon(&mut self, ui: &mut Ui) {}
    /// Whether or not this node has a custom closer.
    ///
    /// Default is false. Override to customize.
    fn has_custom_closer(&self) -> bool {
        false
    }
    /// If [`has_custom_closer`](`NodeConfig::has_custom_closer`) returns true, this method is used to render the custom closer.
    ///
    /// Default does nothing. Override to customize.
    #[allow(unused)]
    fn closer(&mut self, ui: &mut Ui, closer_state: CloserState) {}

    /// Whether or not this node has a context menu.
    ///
    /// Default is false. Override to customize.
    fn has_context_menu(&self) -> bool {
        false
    }
    /// If [`has_context_menu`](`NodeConfig::has_context_menu`) returns true, this method is used to render the context menu.
    ///
    /// Default does nothing. Override to customize.
    #[allow(unused)]
    fn context_menu(&mut self, ui: &mut Ui) {}
}

/// A builder to build a node.
pub struct NodeBuilder<'add_ui, NodeIdType> {
    id: NodeIdType,
    is_dir: bool,
    flatten: bool,
    default_open: bool,
    drop_allowed: bool,
    activatable: bool,
    node_height: Option<f32>,
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
            node_height: None,
            icon: None,
            closer: None,
            label: None,
            context_menu: None,
            default_open: true,
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
            node_height: None,
            icon: None,
            closer: None,
            label: None,
            context_menu: None,
            default_open: true,
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

    /// Set the height of this node.
    pub fn height(mut self, height: f32) -> Self {
        self.node_height = Some(height);
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
}
impl<NodeIdType: NodeId> NodeConfig<NodeIdType> for NodeBuilder<'_, NodeIdType> {
    fn id(&self) -> &NodeIdType {
        &self.id
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }

    fn flatten(&self) -> bool {
        self.flatten
    }

    fn default_open(&self) -> bool {
        self.default_open
    }

    fn drop_allowed(&self) -> bool {
        self.drop_allowed
    }

    fn activatable(&self) -> bool {
        self.activatable
    }

    fn node_height(&self) -> Option<f32> {
        self.node_height
    }

    fn has_custom_icon(&self) -> bool {
        self.icon.is_some()
    }

    fn icon(&mut self, ui: &mut Ui) {
        if let Some(icon) = &mut self.icon {
            (icon)(ui);
        }
    }

    fn has_custom_closer(&self) -> bool {
        self.closer.is_some()
    }

    fn closer(&mut self, ui: &mut Ui, closer_state: CloserState) {
        if let Some(closer) = &mut self.closer {
            (closer)(ui, closer_state);
        }
    }

    fn label(&mut self, ui: &mut Ui) {
        if let Some(label) = &mut self.label {
            (label)(ui);
        }
    }

    fn has_context_menu(&self) -> bool {
        self.context_menu.is_some()
    }

    fn context_menu(&mut self, ui: &mut Ui) {
        if let Some(context_menu) = &mut self.context_menu {
            (context_menu)(ui);
        }
    }
}

pub(crate) struct Node<'config, NodeIdType> {
    pub id: NodeIdType,
    pub is_dir: bool,
    pub is_open: bool,
    pub drop_allowed: bool,
    pub activatable: bool,
    pub node_height: f32,
    pub indent: usize,
    config: &'config mut dyn NodeConfig<NodeIdType>,
}
impl<'config, NodeIdType: NodeId> Node<'config, NodeIdType> {
    pub fn from_config(
        is_open: bool,
        default_node_height: f32,
        indent: usize,
        config: &'config mut dyn NodeConfig<NodeIdType>,
    ) -> Self {
        Self {
            id: config.id().clone(),
            is_dir: config.is_dir(),
            is_open,
            drop_allowed: config.drop_allowed(),
            activatable: config.activatable(),
            node_height: config.node_height().unwrap_or(default_node_height),
            indent,
            config,
        }
    }

    pub fn show_node(
        &mut self,
        ui: &mut Ui,
        interaction: &Response,
        settings: &TreeViewSettings,
        row_rect: Rect,
        selected: bool,
        has_focus: bool,
    ) -> (Option<Rect>, Option<Rect>, Rect) {
        let mut ui = ui.new_child(
            UiBuilder::new()
                .max_rect(row_rect)
                .layout(Layout::left_to_right(egui::Align::Center)),
        );

        // Set the fg stroke colors here so that the ui added by the user
        // has the correct colors when selected or focused.
        let fg_stroke = if selected && has_focus {
            ui.visuals().selection.stroke
        } else if selected {
            ui.visuals().widgets.inactive.fg_stroke
        } else {
            ui.visuals().widgets.noninteractive.fg_stroke
        };
        ui.visuals_mut().widgets.noninteractive.fg_stroke = fg_stroke;
        ui.visuals_mut().widgets.inactive.fg_stroke = fg_stroke;

        // The layouting in the row has to be pretty tight so we tunr of the item spacing here.
        let original_item_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = Vec2::ZERO;

        let (reserve_closer, draw_closer, reserve_icon, draw_icon) = match settings.row_layout {
            RowLayout::Compact => (self.is_dir, self.is_dir, false, false),
            RowLayout::CompactAlignedLabels => (
                self.is_dir,
                self.is_dir,
                !self.is_dir,
                !self.is_dir && self.config.has_custom_icon(),
            ),
            RowLayout::AlignedIcons => (
                true,
                self.is_dir,
                self.config.has_custom_icon(),
                self.config.has_custom_icon(),
            ),
            RowLayout::AlignedIconsAndLabels => {
                (true, self.is_dir, true, self.config.has_custom_icon())
            }
        };

        ui.set_height(self.node_height);
        ui.add_space(original_item_spacing.x);

        // Add a little space so the closer/icon/label doesnt touch the left side
        // and add the indentation space.
        ui.add_space(ui.spacing().item_spacing.x);
        ui.add_space(self.indent as f32 * settings.override_indent.unwrap_or(ui.spacing().indent));

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
                if self.config.has_custom_closer() {
                    self.config.closer(
                        ui,
                        CloserState {
                            is_open: self.is_open,
                            is_hovered,
                        },
                    );
                } else {
                    let icon_id = Id::new(&self.id).with("tree view closer icon");
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
        let icon = if draw_icon && self.config.has_custom_icon() {
            let (_, big_rect) = ui
                .spacing()
                .icon_rectangles(ui.available_rect_before_wrap());
            Some(
                ui.allocate_new_ui(UiBuilder::new().max_rect(big_rect), |ui| {
                    ui.set_min_size(big_rect.size());
                    self.config.icon(ui);
                })
                .response
                .rect,
            )
        } else {
            None
        };
        if icon.is_none() && reserve_icon {
            ui.add_space(ui.spacing().icon_width);
        }

        ui.add_space(2.0);
        // Draw label
        let label = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing = original_item_spacing;
                self.config.label(ui);
            })
            .response
            .rect;

        ui.add_space(original_item_spacing.x);

        (closer, icon, label)
    }

    pub(crate) fn show_context_menu(&mut self, response: &Response) -> bool {
        if self.config.has_context_menu() {
            let mut was_open = false;
            response.context_menu(|ui| {
                self.config.context_menu(ui);
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
