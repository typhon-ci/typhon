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
pub fn setup_tracing_web() {
    use tracing_subscriber::fmt::time::UtcTime;
    use tracing_subscriber::prelude::*;
    use tracing_web::MakeWebConsoleWriter;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false) // Only partially supported across browsers
        .with_timer(UtcTime::rfc_3339())
        .with_writer(MakeWebConsoleWriter::new()); // write events to the console

    tracing_subscriber::registry().with(fmt_layer).init()
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    setup_tracing_web();
    leptos::mount_to_body(App);
}
