mod requests;
mod secrets;
mod streams;

pub mod app;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount_to_body(app::App);
}
