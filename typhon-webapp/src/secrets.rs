use gloo_storage::LocalStorage;
use gloo_storage::Storage;

pub fn get_token() -> Option<String> {
    LocalStorage::get("typhon_token").ok()
}

pub fn set_token(token: &String) {
    LocalStorage::set("typhon_token", &token).unwrap()
}

pub fn reset_token() {
    LocalStorage::delete("typhon_token")
}
