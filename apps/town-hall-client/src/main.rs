#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    town_hall_client::run_native()
}

// `wasm-pack` builds the library target, whose `#[wasm_bindgen(start)]`
// entry point launches eframe after the generated module is initialized.
#[cfg(target_arch = "wasm32")]
fn main() {}
