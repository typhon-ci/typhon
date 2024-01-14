#[cfg(feature = "hydrate")]
use leptos::set_interval;
use leptos::{create_signal, Signal};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct CurrentTime(pub Signal<time::OffsetDateTime>);

#[cfg(feature = "hydrate")]
pub fn now_signal() -> CurrentTime {
    fn imperative_now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp((js_sys::Date::now() / 1000.0) as i64).unwrap()
    }
    let (now, set_now) = create_signal(imperative_now());
    const FIVE_SECONDS: core::time::Duration = core::time::Duration::new(5, 0);
    set_interval(move || set_now(imperative_now()), FIVE_SECONDS);
    CurrentTime(now.into())
}

#[cfg(feature = "ssr")]
pub fn now_signal() -> CurrentTime {
    fn imperative_now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
    let (now, _) = create_signal(imperative_now());
    CurrentTime(now.into())
}

pub struct FlakeUri {
    pub r#ref: String,
    pub web_url: String,
}

impl FlakeUri {
    pub fn parse(uri: String) -> Option<Self> {
        match &uri.clone().split(":").collect::<Vec<_>>()[..] {
            ["github", rest] if let [owner, repo, commit] = &rest.split("/").collect::<Vec<_>>()[..] => {
                let web_url = format!("https://github.com/{owner}/{repo}/tree/{commit}");
                let r#ref = commit[..8].to_string();
                Some(Self {r#ref, web_url})
            },
            _ => None
        }
    }
}
