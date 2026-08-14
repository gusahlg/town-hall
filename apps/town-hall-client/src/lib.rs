//! Native and WebAssembly launchers for the shared Town Hall eframe app.

#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1050.0, 680.0])
            .with_title("Town Hall Simulation"),
        ..Default::default()
    };
    eframe::run_native(
        "Town Hall Simulation",
        options,
        Box::new(|cc| Ok(Box::new(town_hall_gui::native_app(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start_web() -> Result<(), JsValue> {
    use wasm_bindgen::JsCast as _;

    console_error_panic_hook::set_once();
    let canvas = eframe::web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("town-hall-canvas"))
        .ok_or_else(|| JsValue::from_str("missing #town-hall-canvas"))?
        .dyn_into::<eframe::web_sys::HtmlCanvasElement>()?;
    let web_options = eframe::WebOptions::default();
    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(town_hall_gui::web_app(cc)))),
        )
        .await
}
