mod home;
mod login;
mod resources;
mod server_fn;
mod streams;

pub mod app;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount_to_body(app::App);
}
