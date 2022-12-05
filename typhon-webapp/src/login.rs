use seed::{prelude::*, *};

#[derive(Clone)]
pub struct Model {
    password: String,
}

#[derive(Clone)]
pub enum Msg {
    Enter,
    Update(String),
}

pub enum OutMsg {
    Noop,
    Login(String),
}

pub fn init(_orders: &mut impl Orders<Msg>) -> Model {
    Model {
        password: "".into(),
    }
}

pub fn update(msg: Msg, model: &mut Model, _: &mut impl Orders<Msg>) -> OutMsg {
    match msg {
        Msg::Enter => {
            let pw = model.password.clone();
            model.password = "".into();
            OutMsg::Login(pw)
        }
        Msg::Update(pw) => {
            model.password = pw;
            OutMsg::Noop
        }
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    div![
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
