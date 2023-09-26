use crate::{appurl::AppUrl, perform_request, view_error, view_log, SETTINGS};
use seed::{
    prelude::{js_sys::Promise, *},
    *,
};
use typhon_types::*;

pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Job,
    info: Option<responses::JobInfo>,
    log_begin: Option<String>,
    log_end: Option<String>,
    log: Vec<String>,
}

impl Model {
    pub fn app_url(&self) -> AppUrl {
        Vec::<String>::from(self.handle.clone()).into()
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Cancel,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    FetchLogBegin,
    FetchLogEnd,
    GetInfo(responses::JobInfo),
    GetLogBegin(String),
    GetLogEnd(String),
    Noop,
    LogLine(String),
}

#[wasm_bindgen(inline_js = "export async function read_line_by_line(reader) {
    let next = async () => {
        let o = await reader.read();
        return o.done ? null : {line: new TextDecoder().decode(o.value), next};
    };
    return next();
 }
")]
extern "C" {
    fn read_line_by_line(reader: js_sys::Object) -> Promise;
}

use futures_core::stream::Stream;
pub fn fetch_logs_as_stream(drv: String) -> impl Stream<Item = String> {
    use crate::*;
    use async_stream::stream;
    stream! {
        let settings = SETTINGS.get().unwrap();
        let token = get_token();
        let req = Request::new(format!("{}/drv-log", settings.api_server.url(false)))
            .method(Method::Post)
            .json(&drv)
            .expect("Failed to serialize request");
        let req = match token {
            None => req,
            Some(token) => req.header(Header::custom("token", token)),
        };
        let res = req.fetch().await.unwrap();
        let readable_stream: web_sys::ReadableStream = res.raw_response().body().unwrap();
        let reader: js_sys::Object = readable_stream.get_reader();
        let promise = read_line_by_line(reader);
        let mut maybe_promise = Some(promise);
        while let Some(promise) = maybe_promise {
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            let it = future.await.unwrap();
            if it.is_null() {
                maybe_promise = None;
            } else {
                let o = js_sys::Object::from(it);
                let line = js_sys::Reflect::get(&o, &"line".into()).unwrap();
                let line: String = js_sys::JsString::from(line).into();
                let next = js_sys::Function::from(js_sys::Reflect::get(&o, &"next".into()).unwrap());
                let promise =
                    js_sys::Reflect::apply(&next, &js_sys::Object::new(), &js_sys::Array::new())
                    .unwrap();
                yield line;
                maybe_promise = Some(promise.into());
            }
        }
    }
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Job) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        handle: handle.clone(),
        info: None,
        log_begin: None,
        log_end: None,
        log: vec![],
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Cancel => {
            let handle = model.handle.clone();
            let req = requests::Request::Job(handle, requests::Job::Cancel);
            perform_request!(
                orders,
                req,
                responses::Response::Ok => Msg::Noop,
                Msg::Error,
            );
        }
        Msg::Error(err) => {
            model.error = Some(err);
        }
        Msg::ErrorIgnored => {
            model.error = None;
        }
        Msg::Event(_) => {
            orders.send_msg(Msg::FetchInfo);
        }
        Msg::FetchInfo => {
            let handle = model.handle.clone();
            let req = requests::Request::Job(handle, requests::Job::Info);
            perform_request!(
                orders,
                req,
                responses::Response::JobInfo(info) => Msg::GetInfo(info),
                Msg::Error,
            );
        }
        Msg::FetchLogBegin => {
            let handle = model.handle.clone();
            let req = requests::Request::Job(handle, requests::Job::LogBegin);
            perform_request!(
                orders,
                req,
                responses::Response::Log(log) => Msg::GetLogBegin(log),
                Msg::Error,
            );
        }
        Msg::FetchLogEnd => {
            let handle = model.handle.clone();
            let req = requests::Request::Job(handle, requests::Job::LogEnd);
            perform_request!(
                orders,
                req,
                responses::Response::Log(log) => Msg::GetLogEnd(log),
                Msg::Error,
            );
        }
        Msg::GetInfo(info) => {
            if info.status == "waiting" || info.status == "end" || info.status == "success" {
                orders.send_msg(Msg::FetchLogBegin);
            }
            if info.status == "success" {
                orders.send_msg(Msg::FetchLogEnd);
            }
            let drv = info.build_infos.drv.clone();
            orders
                .proxy(Msg::LogLine)
                .stream(fetch_logs_as_stream(drv.into()));
            model.info = Some(info);
        }
        Msg::GetLogBegin(log) => {
            model.log_begin = Some(log);
        }
        Msg::GetLogEnd(log) => {
            model.log_end = Some(log);
        }
        Msg::Noop => (),
        Msg::LogLine(line) => model.log.push(line),
    }
}

fn view_job(model: &Model) -> Node<Msg> {
    div![
        h2![
            "Job",
            " ",
            a![
                &model.handle.evaluation.jobset.project.project,
                attrs! {
                    At::Href => crate::Urls::project(&model.handle.evaluation.jobset.project),
                },
            ],
            ":",
            a![
                &model.handle.evaluation.jobset.jobset,
                attrs! {
                    At::Href => crate::Urls::jobset(&model.handle.evaluation.jobset),
                },
            ],
            ":",
            a![
                &model.handle.evaluation.evaluation,
                attrs! {
                    At::Href => crate::Urls::evaluation(&model.handle.evaluation)
                },
            ],
            ":",
            &model.handle.job,
        ],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                p![
                    "Build: ",
                    a![
                        format!("{}", info.build_handle),
                        attrs! {
                            At::Href => crate::Urls::build(&info.build_handle)
                        },
                    ]
                ],
                p![format!("Status: {}", info.status)],
                if info.dist {
                    let api_url = SETTINGS.get().unwrap().api_server.url(false);
                    let job = &model.handle.job;
                    let evaluation = &model.handle.evaluation.evaluation;
                    let jobset = &model.handle.evaluation.jobset.jobset;
                    let project = &model.handle.evaluation.jobset.project.project;
                    a![
                        "Dist",
                        attrs! {
                            At::Href => format!("{}/projects/{}/jobsets/{}/evaluations/{}/jobs/{}/dist/index.html", api_url, project, jobset, evaluation, job),
                        },
                    ]
                } else {
                    empty![]
                }
            ],
        },
        code![
            &model
                .log
                .join("\n")
                .split("\n")
                .map(|line| div![line])
                .collect::<Vec<_>>(),
            style![St::Background => "#EEFFFFFF"]
        ],
        match &model.log_begin {
            None => empty![],
            Some(log) => div![h3!["Log (begin)"], view_log(log.clone()),],
        },
        match &model.log_end {
            None => empty![],
            Some(log) => div![h3!["Log (end)"], view_log(log.clone()),],
        },
    ]
}

fn view_admin() -> Node<Msg> {
    div![
        h3!["Administration"],
        p![button!["Cancel", ev(Ev::Click, |_| Msg::Cancel),]],
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_job(model),
            if admin { view_admin() } else { empty![] },
        ])
}
