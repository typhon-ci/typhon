use seed::{prelude::*, *};

#[derive(Clone)]
enum State {
    Read,
    Edit(String),
    Sync(String),
}

pub struct Model {
    pub text: String,
    state: State,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Update(String),
    Send,
    Edit,
    Synchronized,
}

#[derive(Clone)]
pub enum OutMsg {
    NewValue(String),
}

pub fn value_synchronized() -> Msg {
    Msg::Synchronized
}

pub fn init(text: String) -> Model {
    Model {
        state: State::Read,
        text,
    }
}

pub fn update(msg: Msg, model: &mut Model) -> Option<OutMsg> {
    match (msg, &mut *model) {
        (
            Msg::Update(text),
            Model {
                state: State::Edit(_),
                ..
            },
        ) => {
            model.state = State::Edit(text);
            None
        }
        (
            Msg::Send,
            Model {
                state: State::Edit(text),
                ..
            },
        ) => {
            let text = text.clone();
            model.state = State::Sync(text.clone());
            Some(OutMsg::NewValue(text))
        }
        (Msg::Edit, _) => {
            model.state = State::Edit(model.text.clone());
            None
        }
        (
            Msg::Synchronized,
            Model {
                state: State::Sync(text),
                ..
            },
        ) => {
            model.text = text.clone();
            model.state = State::Read;
            None
        }
        _ => None,
    }
}

pub fn view(model: &Model, wrap: Box<dyn FnOnce(String) -> Node<Msg>>) -> Node<Msg> {
    match &model.state {
        State::Read => div![
            wrap(model.text.clone()),
            i![C!["ri-pencil-line"], ev(Ev::Click, |_| Msg::Edit)],
            C!["editable-text", "read"],
            if model.text.trim().is_empty() {
                vec![C!["empty"]]
            } else {
                vec![]
            }
        ],
        State::Edit(text) => div![
            input![
                attrs! {At::Value => text},
                input_ev(Ev::Input, Msg::Update),
                keyboard_ev(Ev::KeyUp, |e| {
                    if e.key() == "Enter" {
                        Some(Msg::Send)
                    } else {
                        None
                    }
                })
            ],
            i![C!["ri-check-line"], ev(Ev::Click, |_| Msg::Send)],
            C!["editable-text", "edit"]
        ],
        State::Sync(text) => div![
            input![attrs! {At::Value => text, At::Disabled => true}],
            C!["editable-text", "sync"]
        ],
    }
}
