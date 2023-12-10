use std::marker::PhantomData;

use egui::{
    collapsing_header::CollapsingState,
    epaint::{self},
    layers::ShapeIdx,
    pos2, vec2, Color32, CursorIcon, InnerResponse, LayerId, NumExt, Order, PointerButton, Pos2,
    Rangef, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};
use split_collapsing_state::SplitCollapsingState;
use uuid::Uuid;

pub mod split_collapsing_state;
pub mod v2;

pub struct TreeUi<'a> {
    pub ui: &'a mut Ui,
    pub bounds: Rangef,
    pub parent_id: Option<Uuid>,
    tree_config: &'a TreeViewBuilder,
    context: &'a mut TreeContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition {
    First,
    Last,
    After(Uuid),
    Before(Uuid),
}

#[derive(Clone)]
pub struct DropAction {
    pub dragged_node: Uuid,
    pub target_node: Uuid,
    pub position: DropPosition,
}

pub struct TreeViewResponse {
    pub response: Response,
    pub selected: Option<Uuid>,
    pub hovered: Option<DropAction>,
    pub dropped: Option<DropAction>,
}

struct TreeContext {
    line_count: i32,
    dragged_last_frame: Option<Uuid>,
    selected: Option<Uuid>,
    dragged: Option<Uuid>,
    hovered: Option<(Uuid, DropPosition)>,
    drop_disallowed: bool,
}

pub struct TreeViewBuilder {
    highlight_odd_rows: bool,
    selected: Option<Uuid>,
}
impl TreeViewBuilder {
    pub fn new() -> Self {
        Self {
            highlight_odd_rows: true,
            selected: None,
        }
    }

    pub fn highlight_odd_row(mut self, state: bool) -> Self {
        self.highlight_odd_rows = state;
        self
    }

    pub fn selected(mut self, selected: Option<Uuid>) -> Self {
        self.selected = selected;
        self
    }

    pub fn show(self, ui: &mut Ui, add_content: impl FnOnce(&mut TreeUi)) -> TreeViewResponse {
        // Load state
        let tree_id = ui.make_persistent_id("TreeView");
        ui.ctx().check_for_id_clash(
            tree_id,
            Rect::from_min_size(ui.cursor().min, Vec2::ZERO),
            "Tree view",
        );
        let (selected_last_frame, dragged_last_frame) = ui
            .data_mut(|d| d.get_persisted::<(Option<Uuid>, Option<Uuid>)>(tree_id))
            .unwrap_or((None, None));

        let mut context = TreeContext {
            line_count: 0,
            selected: self.selected.or(selected_last_frame),
            dragged_last_frame: dragged_last_frame,
            dragged: None,
            hovered: None,
            drop_disallowed: false,
        };

        let bounds = ui.available_rect_before_wrap().x_range();
        let res = ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 7.0;

            ui.allocate_at_least(
                vec2(0.0, -ui.spacing().item_spacing.y / 2.0),
                Sense::hover(),
            );

            let mut tree_ui = TreeUi {
                bounds,
                ui,
                context: &mut context,
                parent_id: None,
                tree_config: &self,
            };
            add_content(&mut tree_ui);
            ui.allocate_at_least(
                vec2(ui.available_width(), -ui.spacing().item_spacing.y / 2.0),
                Sense::hover(),
            );
        });

        // Store state
        ui.data_mut(|d| {
            d.insert_persisted::<(Option<Uuid>, Option<Uuid>)>(
                tree_id,
                (context.selected, context.dragged),
            )
        });

        let drop_action = if let (Some(dragged_node), Some((target_node, position)), false) =
            (context.dragged, context.hovered, context.drop_disallowed)
        {
            Some(DropAction {
                dragged_node,
                target_node,
                position,
            })
        } else {
            None
        };

        TreeViewResponse {
            response: res.response,
            selected: context.selected,
            hovered: drop_action.clone(),
            dropped: if ui.ctx().input(|i| i.pointer.any_released()) {
                drop_action
            } else {
                None
            },
        }
    }

    pub fn dir(id: Uuid) -> Node<DirectoryMarker> {
        Node {
            id,
            is_drop_target: true,
            is_open: false,
            phantom: PhantomData,
            is_draggable: true,
            is_selectable: true,
            is_default_open: false,
        }
    }

    pub fn leaf(id: Uuid) -> Node<LeafMarker> {
        Node {
            id,
            is_drop_target: false,
            is_open: false,
            phantom: PhantomData,
            is_draggable: true,
            is_selectable: true,
            is_default_open: false,
        }
    }
}

pub struct DirectoryMarker;
pub struct DirectoryHeadlessMarker;
pub struct LeafMarker;

pub struct Node<T> {
    id: Uuid,
    is_drop_target: bool,
    is_open: bool,
    phantom: PhantomData<T>,
    is_draggable: bool,
    is_selectable: bool,
    is_default_open: bool,
}

impl Node<DirectoryMarker> {
    pub fn headless(self) -> Node<DirectoryHeadlessMarker> {
        Node {
            id: self.id,
            is_drop_target: self.is_drop_target,
            is_open: self.is_open,
            is_draggable: self.is_draggable,
            is_selectable: self.is_selectable,
            phantom: PhantomData,
            is_default_open: false,
        }
    }

    pub fn default_open(mut self, state: bool) -> Self {
        self.is_default_open = state;
        self
    }

    pub fn show<T1, T2>(
        &mut self,
        tree_ui: &mut TreeUi,
        mut add_header: impl FnMut(&mut Ui) -> T1,
        mut add_body: impl FnMut(&mut TreeUi) -> T2,
    ) -> (InnerResponse<T1>, Option<InnerResponse<T2>>) {
        let collapsing_id = tree_ui.ui.id().with("Directory header").with(self.id);
        self.is_open = CollapsingState::load_with_default_open(
            tree_ui.ui.ctx(),
            collapsing_id,
            self.is_default_open,
        )
        .is_open();

        let InnerResponse {
            inner: state,
            response: header,
        } = self.row(tree_ui, |ui| {
            SplitCollapsingState::show_header(ui, collapsing_id, self.is_default_open, |ui| {
                add_header(ui)
            })
        });

        if header.double_clicked_by(PointerButton::Primary) {
            if let Some(mut state) = CollapsingState::load(tree_ui.ui.ctx(), collapsing_id) {
                state.toggle(tree_ui.ui);
                state.store(tree_ui.ui.ctx());
            }
        }

        let hovered_before = tree_ui.context.hovered.clone();
        let body = state.show_body(tree_ui.ui, |ui| {
            let mut tree_ui = TreeUi {
                ui,
                bounds: tree_ui.bounds.clone(),
                context: tree_ui.context,
                parent_id: Some(self.id),
                tree_config: tree_ui.tree_config,
            };
            add_body(&mut tree_ui)
        });

        // It is not allowed to drop a parent node onto one of its child nodes
        let drop_is_child_node = tree_ui.context.hovered != hovered_before;
        let parent_is_dragged = tree_ui
            .context
            .dragged
            .as_ref()
            .is_some_and(|id| id == &self.id);
        if drop_is_child_node && parent_is_dragged {
            tree_ui.context.drop_disallowed = true;
            tree_ui.ui.ctx().set_cursor_icon(CursorIcon::NoDrop);
        }

        (
            InnerResponse::new(state.header_response.inner, header),
            body,
        )
    }
}

impl Node<DirectoryHeadlessMarker> {
    pub fn show<T>(
        &mut self,
        tree_ui: &mut TreeUi,
        mut add_body: impl FnMut(&mut TreeUi) -> T,
    ) -> InnerResponse<T> {
        tree_ui.ui.scope(|ui| {
            let mut tree_ui = TreeUi {
                ui,
                bounds: tree_ui.bounds.clone(),
                context: tree_ui.context,
                parent_id: Some(self.id),
                tree_config: tree_ui.tree_config,
            };
            add_body(&mut tree_ui)
        })
    }
}

impl Node<LeafMarker> {
    pub fn show<T>(
        &self,
        tree_ui: &mut TreeUi,
        mut add_header: impl FnMut(&mut Ui) -> T,
    ) -> InnerResponse<T> {
        self.row(tree_ui, |ui| ui.horizontal(|ui| add_header(ui)).inner)
    }
}

impl<Marker> Node<Marker> {
    pub fn is_draggable(mut self, state: bool) -> Self {
        self.is_draggable = state;
        self
    }
    pub fn is_drop_target(mut self, state: bool) -> Self {
        self.is_drop_target = state;
        self
    }
    pub fn is_selectable(mut self, state: bool) -> Self {
        self.is_selectable = state;
        self
    }

    fn row<T>(
        &self,
        tree_ui: &mut TreeUi,
        mut add_content: impl FnMut(&mut Ui) -> T,
    ) -> InnerResponse<T> {
        tree_ui.context.line_count += 1;
        let is_selected = tree_ui
            .context
            .selected
            .is_some_and(|sel_id| sel_id == self.id);
        let is_even = tree_ui.context.line_count % 2 == 0;

        let row_background = tree_ui.ui.painter().add(Shape::Noop);
        let hover_background = tree_ui.ui.painter().add(Shape::Noop);

        let (interaction, row) = self.row_interaction(tree_ui, |ui| add_content(ui));

        if self.is_selectable {
            if interaction.clicked() || interaction.dragged() {
                tree_ui.context.selected = Some(self.id);
            }
        }

        if self.is_draggable {
            self.draw_drag_overlay(tree_ui, &interaction, &row, |ui| {
                add_content(ui);
            });
        }

        self.drop_targets(tree_ui, &row, hover_background);

        tree_ui.ui.painter().set(
            row_background,
            epaint::RectShape::new(
                row.response.rect,
                tree_ui.ui.visuals().widgets.active.rounding,
                if is_selected {
                    tree_ui.ui.style().visuals.selection.bg_fill
                } else if is_even && tree_ui.tree_config.highlight_odd_rows {
                    Color32::from_rgba_premultiplied(10, 10, 10, 0)
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::NONE,
            ),
        );

        InnerResponse::new(row.inner.inner, interaction)
    }

    /// Adds a row with the width of the bounds that can be clicked or dragged.
    fn row_interaction<T>(
        &self,
        tree_ui: &mut TreeUi,
        add_content: impl FnOnce(&mut Ui) -> T,
    ) -> (Response, InnerResponse<InnerResponse<T>>) {
        // Interact with the background first. If we tryed to interact with the background
        // after the element has been drawn we would take over all of the interaction for
        // the given area and the element would never be allowed to interact.
        // Do this this right we need to remember the size of the background area from
        // last frame.
        let interact_id = tree_ui.ui.next_auto_id().with("row background interaction");
        let interact_rect = tree_ui
            .ui
            .data_mut(|d| d.get_persisted::<Rect>(interact_id))
            .unwrap_or(Rect::NOTHING);
        // The `interact` will add some space to the rect. To get exact interaction we
        // need to take that increase away.
        let interact_rect = interact_rect.expand2(
            (0.5 * tree_ui.ui.spacing().item_spacing - Vec2::splat(0.1))
                .at_least(Vec2::splat(0.0))
                .at_most(Vec2::splat(5.0))
                * -1.0,
        );
        let interact_res = tree_ui
            .ui
            .interact(interact_rect, interact_id, Sense::click_and_drag());

        let res = draw_content_at_full_size(tree_ui, add_content);

        tree_ui
            .ui
            .data_mut(|d| d.insert_persisted(interact_id, res.response.rect));

        (interact_res, res)
    }

    /// Draw the content as a drag overlay if it is beeing dragged.
    fn draw_drag_overlay<T, U>(
        &self,
        tree_ui: &mut TreeUi,
        interaction: &Response,
        row: &InnerResponse<InnerResponse<T>>,
        add_content: impl FnOnce(&mut Ui) -> U,
    ) {
        let TreeUi {
            ui,
            bounds,
            context,
            tree_config,
            ..
        }: &mut TreeUi<'_> = tree_ui;

        let drag_source_id = ui.make_persistent_id("Drag source");

        let drag_offset = if interaction.drag_started_by(PointerButton::Primary) {
            ui.ctx()
                .pointer_latest_pos()
                .map(|pointer_pos| row.response.rect.min - pointer_pos)
                .unwrap_or(Vec2::ZERO)
        } else {
            ui.data_mut(|d| d.get_persisted::<Vec2>(drag_source_id))
                .unwrap_or(Vec2::ZERO)
        };

        if interaction.dragged_by(PointerButton::Primary)
            || interaction.drag_released_by(PointerButton::Primary)
        {
            context.dragged = Some(self.id);
            ui.ctx().set_cursor_icon(CursorIcon::Alias);

            // Paint the content again to a new layer for the drag overlay.
            let layer_id = LayerId::new(Order::Tooltip, drag_source_id);
            let background_rect = ui
                .child_ui(ui.available_rect_before_wrap(), *ui.layout())
                .with_layer_id(layer_id, |ui| {
                    let background = ui.painter().add(Shape::Noop);

                    let mut tree_ui = TreeUi {
                        ui,
                        bounds: bounds.clone(),
                        context: context,
                        parent_id: None,
                        tree_config,
                    };
                    let res = draw_content_at_full_size(&mut tree_ui, add_content);

                    ui.painter().set(
                        background,
                        epaint::RectShape::new(
                            res.response.rect,
                            ui.visuals().widgets.active.rounding,
                            ui.visuals().selection.bg_fill.linear_multiply(0.5),
                            Stroke::NONE,
                        ),
                    );
                    res
                })
                .inner
                .response;

            // Move layer to the drag position
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - background_rect.rect.min + drag_offset;
                ui.ctx().translate_layer(layer_id, delta);
            }
        }

        ui.data_mut(|d| d.insert_persisted::<Vec2>(drag_source_id, drag_offset));
    }

    fn drop_targets<T>(
        &self,
        tree_ui: &mut TreeUi,
        row: &InnerResponse<InnerResponse<T>>,
        background_pos: ShapeIdx,
    ) {
        pub const DROP_LINE_HEIGHT: f32 = 3.0;
        pub const DROP_LINE_HOVER_HEIGHT: f32 = 5.0;
        let TreeUi {
            ui,
            context,
            parent_id,
            ..
        } = tree_ui;

        // If there is nothing dragged we dont have to worry about dropping anything either.
        if context.dragged_last_frame.is_none() {
            return;
        }

        // We dont want to allow dropping on the thing that is beind dragged.
        if context
            .dragged_last_frame
            .is_some_and(|selected_id| self.id == selected_id)
        {
            return;
        }

        let rect = row.response.rect;

        let drop_id = ui.make_persistent_id("Drop target");
        let res = ui.interact(rect, drop_id, Sense::hover());

        let Some(Pos2 { y, .. }) = res.hover_pos() else {
            return;
        };

        // The `interact` adds a bit of space around the rect to make interaction easier.
        // This causes the row above and below to also be hovered when they shouldnt be.
        // Check to make sure we are really only hovering on our rect.
        if y < row.response.rect.top() || y >= row.response.rect.bottom() {
            return;
        }

        let h0 = rect.min.y;
        let h1 = rect.min.y + DROP_LINE_HOVER_HEIGHT;
        let h2 = (rect.min.y + rect.max.y) / 2.0;
        let h3 = rect.max.y - DROP_LINE_HOVER_HEIGHT;
        let h4 = rect.max.y;

        let drop_position = match y {
            y if y >= h0 && y < h1 => parent_id
                .map(|id| (id, DropPosition::Before(self.id)))
                .or_else(|| self.is_drop_target.then_some((self.id, DropPosition::Last))),
            y if y >= h1 && y < h2 => self
                .is_drop_target
                .then_some((self.id, DropPosition::Last))
                .or_else(|| parent_id.map(|id| (id, DropPosition::Before(self.id)))),
            y if y >= h2 && y < h3 => self
                .is_drop_target
                .then_some((self.id, DropPosition::Last))
                .or_else(|| self.is_open.then_some((self.id, DropPosition::First)))
                .or_else(|| parent_id.map(|id| (id, DropPosition::After(self.id)))),
            y if y >= h3 && y < h4 => self
                .is_open
                .then_some((self.id, DropPosition::First))
                .or_else(|| parent_id.map(|id| (id, DropPosition::After(self.id))))
                .or_else(|| self.is_drop_target.then_some((self.id, DropPosition::Last))),
            _ => unreachable!(),
        };

        if let Some((parent_id, drop_position)) = drop_position {
            let line_above =
                rect.min.y - DROP_LINE_HEIGHT / 2.0..=rect.min.y + DROP_LINE_HEIGHT / 2.0;
            let line_below =
                rect.max.y - DROP_LINE_HEIGHT / 2.0..=rect.max.y + DROP_LINE_HEIGHT / 2.0;
            let line_background = h0..=h4;

            let drop_marker = match &drop_position {
                DropPosition::First => line_below,
                DropPosition::Last => line_background,
                DropPosition::After(_) => line_below,
                DropPosition::Before(_) => line_above,
            };

            tree_ui.ui.painter().set(
                background_pos,
                epaint::RectShape::new(
                    Rect::from_x_y_ranges(row.inner.response.rect.x_range(), drop_marker),
                    tree_ui.ui.visuals().widgets.active.rounding,
                    tree_ui.ui.style().visuals.selection.bg_fill,
                    Stroke::NONE,
                ),
            );

            context.hovered = Some((parent_id, drop_position));
        }
    }
}

/// Draws the content and extends their rectangles to the full width of the
/// Tree. The first (inner) `InnerResponse` expands the rectangle to the
/// right side of the tree. The second (outer) `InnerResponse` expands
/// the rect to the left side of the tree.
fn draw_content_at_full_size<T>(
    tree_ui: &mut TreeUi,
    add_content: impl FnOnce(&mut Ui) -> T,
) -> InnerResponse<InnerResponse<T>> {
    let TreeUi { ui, bounds, .. } = tree_ui;

    // Show the element.
    let scope = ui.scope(|ui| {
        let res = ui.scope(|ui| add_content(ui));

        let background_to_right = Rect::from_min_max(
            res.response.rect.min,
            pos2(bounds.max, res.response.rect.max.y),
        )
        .expand2(vec2(0.0, ui.spacing().item_spacing.y / 2.0));
        InnerResponse::new(res.inner, res.response.with_new_rect(background_to_right))
    });

    let background_full_width =
        Rect::from_x_y_ranges(bounds.clone(), scope.response.rect.y_range())
            .expand2(vec2(0.0, ui.spacing().item_spacing.y / 2.0));

    InnerResponse::new(
        scope.inner,
        scope.response.with_new_rect(background_full_width),
    )
}
