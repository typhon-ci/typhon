#[cfg(feature = "hydrate")]
use gloo_storage::LocalStorage;
#[cfg(feature = "hydrate")]
use gloo_storage::Storage;

#[allow(dead_code)]
#[cfg(feature = "hydrate")]
pub fn get_token() -> Option<String> {
    LocalStorage::get("typhon_token").ok()
}

#[allow(dead_code)]
#[cfg(feature = "hydrate")]
pub fn set_token(token: &String) {
    LocalStorage::set("typhon_token", &token).unwrap()
}

#[allow(dead_code)]
#[cfg(feature = "hydrate")]
pub fn reset_token() {
    LocalStorage::delete("typhon_token")
}

#[allow(dead_code)]
#[cfg(feature = "ssr")]
pub fn get_token() -> Option<String> {
    None
}

#[allow(dead_code)]
#[cfg(feature = "ssr")]
pub fn set_token(_token: &String) {}

#[allow(dead_code)]
#[cfg(feature = "ssr")]
pub fn reset_token() {}
