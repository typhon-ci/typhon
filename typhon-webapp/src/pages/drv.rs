use crate::widgets::drv_log;

use seed::{prelude::*, *};

struct_urls!();

pub struct Model {
    drv: String,
    log: drv_log::Model,
}

#[derive(Debug)]
pub enum Msg {
    MsgLog(drv_log::Msg),
}

pub fn init(_base_url: Url, orders: &mut impl Orders<Msg>, drv: &String) -> Model {
    Model {
        drv: drv.clone(),
        log: drv_log::init(&mut orders.proxy(Msg::MsgLog), &drv),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::MsgLog(msg) => drv_log::update(msg, &mut model.log, &mut orders.proxy(Msg::MsgLog)),
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    div![
        h2![format!("Derivation {}", model.drv)],
        h2!["Log"],
        drv_log::view(&model.log).map_msg(Msg::MsgLog),
    ]
}
