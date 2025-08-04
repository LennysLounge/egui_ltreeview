#[path = "data.rs"]
mod data;

use egui::ThemePreference;
use egui_ltreeview::TreeView;


#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            TreeView::new(ui.make_persistent_id("Names tree view")).show(ui, |builder| {
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
                builder.close_dir();
                builder.close_dir();
            });
        });
    }
}

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    
    let web_options = eframe::WebOptions::default();
    
    // Get the canvas element from DOM
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    
    wasm_bindgen_futures::spawn_local(async move {
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    cc.egui_ctx
                        .options_mut(|options| options.theme_preference = ThemePreference::Dark);
                    Ok(Box::<MyApp>::default())
                }),
            )
            .await
            .expect("failed to start eframe");
    });
    
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([300.0, 500.0]),
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