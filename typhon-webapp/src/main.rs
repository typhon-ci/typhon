use serde::{Deserialize, Serialize};
use typhon_webapp::{app, ApiServerSettings, Settings};

/// Finds the webroot of the webapp by grapping the [link] tag
/// corresponding to the typhon webapp's wasm.
fn find_webroot() -> Option<String> {
    let href = web_sys::window()?
        .document()?
        .query_selector("link[type='application/wasm']")
        .ok()
        .flatten()?
        .get_attribute("href")?;
    web_sys::console::log_1(&format!("href={:#?}", href).into());
    Some(format!(
        "{}/",
        href.rsplit_once("/")
            .map(|(dirname, _)| dirname)
            .unwrap_or(".")
    ))
}

async fn load_settings() -> Settings {
    let client_webroot = find_webroot().unwrap_or_else(|| {
        web_sys::console::warn_1(&"Could not detect webroot".into());
        panic!()
    });
    use seed::prelude::*;
    let settings_path = format!("{client_webroot}settings.json");
    let req = Request::new(&settings_path).method(Method::Get);
    let api_server = match req.fetch().await {
        Ok(data) if data.status().is_ok() => data
            .json()
            .await
            .map_err(|_| {
                web_sys::console::warn_1(
                    &format!("Could not load `{}`, using defaults.", settings_path).into(),
                )
            })
            .ok(),
        _ => None,
    }
    .unwrap_or_else(ApiServerSettings::default);
    Settings {
        api_server,
        client_webroot,
    }
}

async fn async_main() {
    app(serde_wasm_bindgen::to_value(&load_settings().await).unwrap());
}

pub fn main() {
    wasm_bindgen_futures::spawn_local(async_main())
}
