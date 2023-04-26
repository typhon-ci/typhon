use typhon_webapp::{app, Settings};

async fn load_settings() -> Settings {
    use seed::prelude::*;
    let req = Request::new("settings.json").method(Method::Get);
    match req.fetch().await {
        Ok(data) if data.status().is_ok() => data.json().await.ok().unwrap_or_else(|| {
            web_sys::console::error_1(&"WARNING: Could not load [settings.json]!".into());
            Settings::default()
        }),
        _ => Settings::default(),
    }
}

async fn async_main() {
    app(serde_wasm_bindgen::to_value(&load_settings().await).unwrap());
}

pub fn main() {
    wasm_bindgen_futures::spawn_local(async_main())
}
