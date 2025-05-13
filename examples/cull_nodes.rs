#[path = "data.rs"]
mod data;

use egui::{vec2, Color32, Rect, ThemePreference, UiBuilder};
use egui_ltreeview::TreeView;

fn main() -> Result<(), eframe::Error> {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Egui_ltreeview simple example",
        options,
        Box::new(|cc| {
            cc.egui_ctx
                .options_mut(|options| options.theme_preference = ThemePreference::Dark);
            Ok(Box::<MyApp>::default())
        }),
    )
} 

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_zoom_factor(3.0);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(25.0);
            // ui.horizontal(|ui| {
            //     let r = ui.label("xxjH");
            //     ui.painter().rect_stroke(
            //         r.rect,
            //         0.0,
            //         (1.0, Color32::WHITE),
            //         egui::StrokeKind::Inside,
            //     );
            //     let size = vec2(
            //         10.0,
            //         ui.fonts(|f| f.row_height(&FontSelection::Default.resolve(ui.style()))),
            //     );
            //     ui.painter().rect_stroke(
            //         Rect::from_min_size(pos2(r.rect.max.x + 2.0, r.rect.min.y), size),
            //         0.0,
            //         (1.0, Color32::WHITE),
            //         StrokeKind::Inside,
            //     );
            // });

            let rect = Rect::from_min_size(ui.cursor().min, vec2(200.0, 200.0));

            ui.painter()
                .rect_stroke(rect, 0, (1.0, Color32::WHITE), egui::StrokeKind::Middle);

            let mut new_ui = ui.new_child(UiBuilder::new().max_rect(rect));
            new_ui.add_space(-25.0);
            TreeView::new(ui.make_persistent_id("Names tree view")).show(&mut new_ui, |builder| {
                builder.dir(0, "Root");
                builder.dir(1, "Foo");
                builder.leaf(2, "Ava");
                builder.dir(3, "Bar");
                builder.leaf(4, "Benjamin");
                builder.leaf(5, "Charlotte");
                builder.close_dir();
                builder.close_dir();
                builder.leaf(6, "Daniel");
                builder.leaf(7, "Emma");
                builder.dir(8, "Baz");
                builder.leaf(9, "Finn");
                builder.leaf(10, "Grayson");
                builder.leaf(11, "Harry");
                builder.leaf(12, "Iris");
                builder.close_dir();
                builder.close_dir();
            });
        });
    }
}
