use seed::{prelude::*, *};

#[derive(Clone)]
pub struct Model {
    password: String,
}

#[derive(Clone)]
pub enum Msg {
    Update(String),
    Enter,
}

pub fn init(_orders: &mut impl Orders<Msg>) -> Model {
    Model {
        password: "".into(),
    }
}

pub fn update(msg: Msg, model: &mut Model, _: &mut impl Orders<Msg>) {
    match msg {
        Msg::Update(pw) => {
            model.password = pw;
        }
        Msg::Enter => {
            crate::set_password(&model.password); // TODO
            model.password = "".into();
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
