#![feature(iter_next_chunk)]
#![feature(iter_intersperse)]
#![feature(if_let_guard)]

pub mod app;
pub use app::App;

mod components;
mod handle_request;
mod pages;
mod prelude;
mod routes;
mod status;
mod streams;
mod utils;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
