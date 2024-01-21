use egui::{
    epaint, vec2, CursorIcon, InnerResponse, LayerId, Order, PointerButton, Pos2, Rangef, Rect,
    Response, Sense, Shape, Stroke, Ui, Vec2,
};

pub struct Row<NodeIdType> {
    pub id: NodeIdType,
    pub depth: usize,
    pub drop_on_allowed: bool,
    pub is_open: bool,
    pub is_dir: bool,
}

impl<NodeIdType> Row<NodeIdType>
where
    NodeIdType: Clone + std::hash::Hash,
{
    pub fn show(
        &self,
        ui: &mut Ui,
        add_label: &mut dyn FnMut(&mut Ui),
        add_icon: &mut Option<&mut dyn FnMut(&mut Ui) -> Response>,
    ) -> RowResponse {
        // Load row data
        let row_id = ui.id().with(self.id.clone()).with("row");
        let row_rect = crate::load(ui, row_id).unwrap_or(Rect::NOTHING);

        // Interact with the row
        let interaction = crate::interact(ui, row_rect, row_id, Sense::click_and_drag());

        let was_dragged = self.drag(ui, &interaction, add_label, add_icon);
        let drop_target = self.drop(ui, &interaction);

        let (row_response, icon_response) = self.draw_row(ui, add_label, add_icon);

        crate::store(ui, row_id, row_response.rect);

        RowResponse {
            interaction,
            visual: row_response,
            icon: icon_response,
            was_dragged,
            drop_quarter: drop_target,
        }
    }
    /// Draw the content as a drag overlay if it is beeing dragged.
    fn drag(
        &self,
        ui: &mut Ui,
        interaction: &Response,
        add_label: &mut dyn FnMut(&mut Ui),
        add_icon: &mut Option<&mut dyn FnMut(&mut Ui) -> Response>,
    ) -> bool {
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
            crate::store(ui, drag_source_id, drag_offset);
            drag_offset
        } else {
            crate::load(ui, drag_source_id).unwrap_or(Vec2::ZERO)
        };

        // Paint the content to a new layer for the drag overlay.
        let layer_id = LayerId::new(Order::Tooltip, drag_source_id);

        let background_rect = ui
            .child_ui(ui.available_rect_before_wrap(), *ui.layout())
            .with_layer_id(layer_id, |ui| {
                let background_position = ui.painter().add(Shape::Noop);

                let (row, _) = self.draw_row(ui, add_label, add_icon);

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
            let Some(Pos2 { y, .. }) = crate::interact(
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

    fn draw_row(
        &self,
        ui: &mut Ui,
        add_label: &mut dyn FnMut(&mut Ui),
        add_icon: &mut Option<&mut dyn FnMut(&mut Ui) -> Response>,
    ) -> (Response, Option<Response>) {
        let InnerResponse {
            inner: icon_response,
            response: row_response,
        } = ui.horizontal(|ui| {
            ui.add_space(ui.spacing().indent * self.depth as f32);

            let icon_pos = ui.cursor().min;
            if add_icon.is_some() {
                ui.add_space(ui.spacing().icon_width);
            };
            (add_label)(ui);
            ui.add_space(ui.available_width());

            add_icon.as_mut().map(|add_icon| {
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

pub enum DropQuarter {
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

pub struct RowResponse {
    pub interaction: Response,
    pub visual: Response,
    pub icon: Option<Response>,
    pub was_dragged: bool,
    pub drop_quarter: Option<DropQuarter>,
}
