use gloo_storage::LocalStorage;
use gloo_storage::Storage;

#[allow(dead_code)]
pub fn get_token() -> Option<String> {
    LocalStorage::get("typhon_token").ok()
}

#[allow(dead_code)]
pub fn set_token(token: &String) {
    LocalStorage::set("typhon_token", &token).unwrap()
}

#[allow(dead_code)]
pub fn reset_token() {
    LocalStorage::delete("typhon_token")
}
