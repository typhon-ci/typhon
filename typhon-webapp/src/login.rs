use crate::perform_request;
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: bool,
    password: String,
}

#[derive(Clone)]
pub enum Msg {
    Enter,
    Error,
    Success(String),
    Update(String),
}

pub enum OutMsg {
    Login(String),
    Noop,
}

pub fn init(_orders: &mut impl Orders<Msg>) -> Model {
    Model {
        error: false,
        password: "".into(),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) -> OutMsg {
    match msg {
        Msg::Enter => {
            let req = requests::Request::Login(model.password.clone());
            model.password = "".into();
            perform_request!(
                orders,
                req,
                responses::Response::Login {token} => Msg::Success(token),
                |_| Msg::Error,
            );
            OutMsg::Noop
        }
        Msg::Error => {
            model.error = true;
            model.password = "".into();
            OutMsg::Noop
        }
        Msg::Success(token) => {
            model.error = false;
            OutMsg::Login(token)
        }
        Msg::Update(pw) => {
            model.password = pw;
            OutMsg::Noop
        }
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    div![
        p![if model.error { "Failed to log in" } else { "" }],
        input![
            attrs! {
                At::Value => model.password,
                At::Type => "password",
            },
            input_ev(Ev::Input, Msg::Update),
        ],
        button!["Login", ev(Ev::Click, |_| Msg::Enter),],
    ]
}
