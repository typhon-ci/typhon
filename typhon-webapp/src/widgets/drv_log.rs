use seed::{prelude::*, *};

pub struct Model {
    drv: Option<String>,
    log: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Chunk(String),
    Load(String),
}

pub fn init(_orders: &mut impl Orders<Msg>) -> Model {
    Model {
        drv: None,
        log: Vec::new(),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    use crate::get_token;
    use crate::streams;
    use crate::Settings;

    use gloo_net::http;

    match msg {
        Msg::Chunk(chunk) => model.log.push(chunk),
        Msg::Load(drv) => {
            model.drv = Some(drv.clone());
            model.log = Vec::new();

            let settings = Settings::load();
            let req = http::RequestBuilder::new(&format!("{}/drv-log{}", settings.api_url, drv))
                .method(http::Method::GET);
            let req = match get_token() {
                None => req,
                Some(token) => req.header(&"token", &token),
            };
            let req = req.build().unwrap();

            orders
                .proxy(|chunk: String| Msg::Chunk(chunk))
                .stream(streams::fetch_as_stream(req));
        }
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    code![
        &model
            .log
            .join("\n")
            .split("\n")
            .map(|line| div![line.replace(" ", " ")])
            .collect::<Vec<_>>(),
        style![St::Background => "#EEFFFFFF"]
    ]
}
