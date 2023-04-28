#[derive(Clone, Debug, Default)]
pub struct AppUrl {
    pub chunks: Vec<String>,
}
impl From<AppUrl> for seed::Url {
    fn from(url: AppUrl) -> seed::Url {
        url.chunks.iter().fold(
            seed::Url::new().set_path(crate::webroot_chunks()),
            |url, chunk| url.add_path_part(chunk),
        )
    }
}

impl From<&str> for AppUrl {
    fn from(s: &str) -> AppUrl {
        AppUrl {
            chunks: s.split("/").map(|chunk| chunk.into()).collect(),
        }
    }
}

impl From<String> for AppUrl {
    fn from(s: String) -> AppUrl {
        s.as_str().into()
    }
}

impl<T: Into<String>> From<Vec<T>> for AppUrl {
    fn from(v: Vec<T>) -> AppUrl {
        AppUrl {
            chunks: v.into_iter().map(|p| p.into()).collect(),
        }
    }
}

impl std::ops::Add for AppUrl {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            chunks: self
                .chunks
                .iter()
                .chain(other.chunks.iter())
                .cloned()
                .collect(),
        }
    }
}
