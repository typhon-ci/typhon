use seed::prelude::*;

pub struct Model {
    log: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Chunk(String),
}

pub fn init(orders: &mut impl Orders<Msg>, drv: &String) -> Model {
    use crate::secrets;
    use crate::settings::Settings;
    use crate::streams;

    use gloo_net::http;

    let settings = Settings::load();
    let req = http::RequestBuilder::new(&format!("{}/drv-log{}", settings.api_url, drv))
        .method(http::Method::GET);
    let req = match secrets::get_token() {
        None => req,
        Some(token) => req.header(&"token", &token),
    };
    let req = req.build().unwrap();
    orders
        .proxy(|chunk: String| Msg::Chunk(chunk))
        .stream(streams::fetch_as_stream(req));

    Model { log: Vec::new() }
}

pub fn update(msg: Msg, model: &mut Model, _orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Chunk(chunk) => model.log.push(chunk),
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    crate::views::log::view(model.log.join("\n"))
}
