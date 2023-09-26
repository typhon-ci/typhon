use crate::appurl::AppUrl;
use crate::perform_request;

use typhon_types::*;

use seed::{prelude::*, *};

pub struct Model {
    error: bool,
    password: String,
    previous_page_url: Option<AppUrl>,
}

impl Model {
    pub fn app_url(&self) -> AppUrl {
        AppUrl::default()
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Enter,
    Update(String),
    Error,
    LoggedIn { token: String },
}

pub enum OutMsg {
    Login(String, AppUrl),
}

pub fn init(_orders: &mut impl Orders<Msg>, previous_page_url: Option<AppUrl>) -> Model {
    Model {
        error: false,
        password: "".into(),
        previous_page_url,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) -> Option<OutMsg> {
    match msg {
        Msg::Enter => {
            let req = requests::Request::Login(model.password.clone());
            perform_request!(
                orders,
                req,
                responses::Response::Login {token} => Msg::LoggedIn {token},
                |_| Msg::Error,
            );
            None
        }
        Msg::LoggedIn { token } => Some(OutMsg::Login(
            token,
            model.previous_page_url.clone().unwrap_or("".into()),
        )),
        Msg::Error => {
            model.password = "".into();
            model.error = true;
            None
        }
        Msg::Update(password) => {
            model.password = password;
            None
        }
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    div![
        h2!["Log in Typhon"],
        aside![
            i![attrs! {At::Class => "ri-information-line"}],
            div![
                "Currently Typhon only supports one user, hence the username field being disabled."
            ]
        ],
        if model.error {
            vec![div![
                i![attrs! {At::Class => "ri-error-warning-fill"}],
                " Incorrect password.",
                attrs! {
                    At::Class => "error"
                }
            ]]
        } else {
            vec![]
        },
        div![
            div![
                label![
                    i![attrs! {At::Class => "ri-user-smile-line"}],
                    "Username",
                    attrs! {
                        At::For => "username_field"
                    }
                ],
                input![attrs! {
                    At::Value => model.password,
                    At::Type => "username",
                    At::Id => "username_field",
                    At::Disabled => true,
                    At::Value => "admin"
                }],
            ],
            div![
                label![
                    i![attrs! {At::Class => "ri-lock-password-line"}],
                    "Password",
                    attrs! {
                        At::For => "password_field"
                    }
                ],
                input![
                    attrs! {
                        At::Value => model.password,
                        At::Type => "password",
                        At::Id => "password_field",
                    },
                    input_ev(Ev::Input, Msg::Update),
                    keyboard_ev(Ev::KeyUp, |e| {
                        if e.key() == "Enter" {
                            Some(Msg::Enter)
                        } else {
                            None
                        }
                    }),
                ],
            ],
            button!["Login", ev(Ev::Click, |_| Msg::Enter),],
            attrs! {
                At::Class => "login-form"
            }
        ],
        attrs! {
            At::Class => "login-page"
        }
    ]
}
