use crate::perform_request;
use crate::{appurl::AppUrl, perform_request_aux};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: bool,
    password: String,
    previous_page: Option<Box<crate::Page>>,
}

impl From<Model> for AppUrl {
    fn from(_: Model) -> AppUrl {
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

pub enum Redirect {
    ToPage(crate::Page),
    ToUrl(String),
}

pub enum OutMsg {
    Login(String, Redirect),
}

pub fn init(_orders: &mut impl Orders<Msg>, previous_page: Option<crate::Page>) -> Model {
    Model {
        error: false,
        password: "".into(),
        previous_page: previous_page.map(Box::new),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) -> Option<OutMsg> {
    match msg {
        Msg::Enter => {
            // let redirect = OutMsg::Login(
            //     model.password.clone(),
            //     match &model.previous_page {
            //         None => Redirect::ToUrl("".into()),
            //         Some(page) => Redirect::ToPage(*page.clone()),
            //     },
            // );
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
            match &model.previous_page {
                None => Redirect::ToUrl("".into()),
                Some(page) => Redirect::ToPage(*page.clone()),
            },
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
                " Currently Typhon supports only one user, whence the username field being disabled."
            ]
        ],
        if model.error {
            vec![
                div![
                    i![attrs! {At::Class => "ri-error-warning-fill"}],
                    " Incorrect password.",
                    attrs! {
                        At::Class => "error"
                    }
                ]
            ]
        } else { vec![] },
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
