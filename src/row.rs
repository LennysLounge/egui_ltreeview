use egui::{
    emath, epaint, remap, vec2, CursorIcon, Id, InnerResponse, LayerId, Order, Rangef, Rect,
    Response, Shape, Stroke, Ui,
};

use crate::{
    builder::{AddCloser, AddIcon, CloserState},
    Interaction, RowLayout, TreeViewSettings, TreeViewState,
};

pub struct Row<NodeIdType> {
    pub id: NodeIdType,
    pub depth: f32,
    pub drop_on_allowed: bool,
    pub is_open: bool,
    pub is_dir: bool,
    pub is_selected: bool,
    pub is_focused: bool,
}

impl<NodeIdType> Row<NodeIdType>
where
    NodeIdType: Clone + Copy + Send + Sync + std::hash::Hash + PartialEq + Eq + 'static,
{
    /// Draw the content as a drag overlay if it is beeing dragged.
    pub(crate) fn draw_row_dragged(
        &self,
        ui: &mut Ui,
        settings: &TreeViewSettings,
        state: &TreeViewState<NodeIdType>,
        add_label: &mut dyn FnMut(&mut Ui),
        add_icon: &mut Option<&mut AddIcon<'_>>,
        add_closer: &mut Option<&mut AddCloser<'_>>,
    ) -> bool {
        ui.ctx().set_cursor_icon(CursorIcon::Alias);

        let drag_source_id = ui.make_persistent_id("Drag source");

        // Paint the content to a new layer for the drag overlay.
        let layer_id = LayerId::new(Order::Tooltip, drag_source_id);

        let background_rect = ui
            .child_ui(ui.available_rect_before_wrap(), *ui.layout())
            .with_layer_id(layer_id, |ui| {
                let background_position = ui.painter().add(Shape::Noop);

                let (row, _, _) =
                    self.draw_row(ui, state, settings, add_label, add_icon, add_closer);

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
            //let delta = -background_rect.min.to_vec2() + pointer_pos.to_vec2() + drag_offset;
            let delta = -background_rect.min.to_vec2()
                + pointer_pos.to_vec2()
                + state.peristant.dragged.as_ref().unwrap().drag_row_offset;
            ui.ctx().translate_layer(layer_id, delta);
        }

        true
    }

    pub(crate) fn draw_row(
        &self,
        ui: &mut Ui,
        interaction: &TreeViewState<NodeIdType>,
        settings: &TreeViewSettings,
        add_label: &mut dyn FnMut(&mut Ui),
        add_icon: &mut Option<&mut AddIcon<'_>>,
        add_closer: &mut Option<&mut AddCloser<'_>>,
    ) -> (Response, Option<Response>, Rect) {
        let (reserve_closer, draw_closer, reserve_icon, draw_icon) = match settings.row_layout {
            RowLayout::Compact => (self.is_dir, self.is_dir, false, false),
            RowLayout::CompactAlignedLables => (
                self.is_dir,
                self.is_dir,
                !self.is_dir,
                !self.is_dir && add_icon.is_some(),
            ),
            RowLayout::AlignedIcons => (true, self.is_dir, add_icon.is_some(), add_icon.is_some()),
            RowLayout::AlignedIconsAndLabels => (true, self.is_dir, true, add_icon.is_some()),
        };

        let InnerResponse {
            inner: (closer_response, label_rect_min),
            response: row_response,
        } = ui.horizontal(|ui| {
            // Set the fg stroke colors here so that the ui added by the user
            // has the correct colors when selected or focused.
            let fg_stroke = if self.is_selected && self.is_focused {
                ui.visuals().selection.stroke
            } else if self.is_selected {
                ui.visuals().widgets.inactive.fg_stroke
            } else {
                ui.visuals().widgets.noninteractive.fg_stroke
            };
            ui.visuals_mut().widgets.noninteractive.fg_stroke = fg_stroke;
            ui.visuals_mut().widgets.inactive.fg_stroke = fg_stroke;

            // Add a little space so the closer/icon/label doesnt touch the left side
            // and add the indentation space.
            ui.add_space(ui.spacing().item_spacing.x);
            ui.add_space(self.depth);
            
            // The closer and the icon should be drawn vertically centered to the label.
            // To do this we first have to draw the label and then the closer and icon
            // to get the correct position.
            let closer_pos = ui.cursor().min;
            if reserve_closer {
                ui.add_space(ui.spacing().icon_width);
            }

            let icon_pos = ui.cursor().min;
            if reserve_icon {
                ui.add_space(ui.spacing().icon_width);
            };

            ui.add_space(2.0);
            let label_pos = ui.cursor().min;
            (add_label)(ui);

            let closer_response = if draw_closer {
                let (_small_rect, _big_rect) = ui.spacing().icon_rectangles(Rect::from_min_size(
                    closer_pos,
                    vec2(ui.spacing().icon_width, ui.min_size().y),
                ));

                let res = ui.allocate_ui_at_rect(_big_rect, |ui| {
                    let closer_interaction = interaction.interact(&ui.max_rect());
                    if closer_interaction.hovered {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if let Some(add_closer) = add_closer {
                        (add_closer)(
                            ui,
                            CloserState {
                                is_open: self.is_open,
                                is_hovered: closer_interaction.hovered,
                            },
                        );
                    } else {
                        let icon_id = Id::new(self.id).with("tree view closer icon");
                        let openness = ui.ctx().animate_bool(icon_id, self.is_open);
                        let closer_interaction = interaction.interact(&ui.max_rect());
                        paint_default_icon(ui, openness, &_small_rect, &closer_interaction);
                    }
                    ui.allocate_space(ui.available_size_before_wrap());
                });
                Some(res.response)
            } else {
                None
            };
            if draw_icon {
                add_icon.as_mut().map(|add_icon| {
                    let (_small_rect, _big_rect) =
                        ui.spacing().icon_rectangles(Rect::from_min_size(
                            icon_pos,
                            vec2(ui.spacing().icon_width, ui.min_size().y),
                        ));
                    ui.allocate_ui_at_rect(_big_rect, |ui| add_icon(ui))
                        .response
                });
            }
            let label_rect_min = if draw_closer {
                closer_pos
            } else if draw_icon {
                icon_pos
            } else {
                label_pos
            };
            (closer_response, label_rect_min.x)
        });

        let mut background_rect = row_response
            .rect
            .expand2(vec2(0.0, ui.spacing().item_spacing.y * 0.5));
        background_rect.set_width(ui.available_width());
        let label_rect = {
            let mut rect = background_rect;
            rect.min.x = label_rect_min;
            rect
        };

        (
            row_response.with_new_rect(background_rect),
            closer_response,
            label_rect,
        )
    }
}

/// Paint the arrow icon that indicated if the region is open or not
fn paint_default_icon(ui: &mut Ui, openness: f32, rect: &Rect, interaction: &Interaction) {
    let visuals = if interaction.hovered {
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

pub enum DropQuarter {
    Top,
    MiddleTop,
    MiddleBottom,
    Bottom,
}

impl DropQuarter {
    pub fn new(range: Rangef, cursor_pos: f32) -> Option<DropQuarter> {
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
