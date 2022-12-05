pub fn main() {
    typhon_webapp::app(serde_wasm_bindgen::to_value(&typhon_webapp::Settings::default()).unwrap());
}
