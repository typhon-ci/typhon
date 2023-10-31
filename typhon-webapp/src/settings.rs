use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub api_url: String,
}

impl Settings {
    pub fn load() -> Self {
        serde_json::from_str::<Option<Self>>(
            &leptos::document()
                .query_selector("script[id='settings']")
                .unwrap()
                .unwrap()
                .inner_html(),
        )
        .unwrap()
        .unwrap_or_default()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:8000/api".into(),
        }
    }
}
