use typhon_core::error;
use typhon_core::handle_request;
use typhon_core::User;
use typhon_core::EVENT_LOGGER;
use typhon_types::handles;
use typhon_types::requests::*;
use typhon_types::responses::{Response, ResponseError};

use actix_files::NamedFile;
use actix_session::Session;
use actix_web::{
    body::EitherBody, guard, http::StatusCode, web, HttpRequest, HttpResponse, Responder,
};
use actix_web::{dev::Payload, FromRequest};
use uuid::Uuid;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

struct ResponseWrapper(Response);
#[derive(Debug)]
struct ResponseErrorWrapper(ResponseError);

impl From<actix_web::error::BlockingError> for ResponseErrorWrapper {
    fn from(_: actix_web::error::BlockingError) -> ResponseErrorWrapper {
        ResponseErrorWrapper(ResponseError::InternalError)
    }
}

impl From<error::Error> for ResponseErrorWrapper {
    fn from(e: error::Error) -> ResponseErrorWrapper {
        ResponseErrorWrapper(e.into())
    }
}

impl std::fmt::Display for ResponseErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Responder for ResponseWrapper {
    type Body = EitherBody<String>;
    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        use Response::*;
        match self.0 {
            Ok => web::Json(true).respond_to(req),
            Search(payload) => web::Json(payload).respond_to(req),
            ProjectInfo(payload) => web::Json(payload).respond_to(req),
            JobsetInfo(payload) => web::Json(payload).respond_to(req),
            JobsetEvaluate(payload) => web::Json(payload).respond_to(req),
            EvaluationInfo(payload) => web::Json(payload).respond_to(req),
            JobInfo(payload) => web::Json(payload).respond_to(req),
            BuildInfo(payload) => web::Json(payload).respond_to(req),
            ActionInfo(payload) => web::Json(payload).respond_to(req),
            RunInfo(payload) => web::Json(payload).respond_to(req),
            User(payload) => web::Json(payload).respond_to(req),
        }
    }
}

struct UserWrapper(User);

impl FromRequest for UserWrapper {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<UserWrapper, actix_web::Error>>>>;

    fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
        let maybe_user = req
            .headers()
            .get("password")
            .map(|value| value.as_bytes())
            .as_ref()
            .map(|password| User::from_password(password));
        let session = Session::from_request(req, pl);
        Box::pin(async move {
            match maybe_user {
                Some(user) => Ok(UserWrapper(user)),
                None => {
                    let user = session
                        .await?
                        .get::<User>("user")?
                        .unwrap_or(User::Anonymous);
                    Ok(UserWrapper(user))
                }
            }
        })
    }
}

impl actix_web::ResponseError for ResponseErrorWrapper {
    fn status_code(&self) -> StatusCode {
        match self.0 {
            ResponseError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ResponseError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            ResponseError::ResourceNotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}

/// A macro to generate api endpoints
macro_rules! r {
    ($name: ident($($i: ident : $t: ty),*) => $e: expr
     ;$($rest: tt)*
    ) => {
    async fn $name (user: UserWrapper, $($i : $t),*) -> Result<ResponseWrapper, ResponseErrorWrapper> {
        handle_request(user.0, $e).await.map(ResponseWrapper).map_err(ResponseErrorWrapper)
    } r!( $($rest)* );
    };
    (  ) => {}
}

r!(
    search(body: web::Json<search::Request>) =>
        Request::Search(body.into_inner());

    create_project(path: web::Path<String>, body: web::Json<ProjectDecl>) => {
        let name = path.into_inner();
        let decl = body.into_inner();
        Request::CreateProject { name, decl }
    };

    //project_delete(path: web::Path<String>) =>
    //    Request::Project(
    //        handles::project(path.into_inner()),
    //        Project::Delete,
    //    );

    project_info(path: web::Path<String>) =>
        Request::Project(
            handles::project(path.into_inner()),
            Project::Info,
        );

    project_refresh(path: web::Path<String>) =>
        Request::Project(
            handles::project(path.into_inner()),
            Project::Refresh,
        );

    project_set_decl(path: web::Path<String>, body: web::Json<ProjectDecl>) =>
        Request::Project(
            handles::project(path.into_inner()),
            Project::SetDecl(body.into_inner()),
        );

    project_update_jobsets(path: web::Path<String>) =>
        Request::Project(
            handles::project(path.into_inner()),
            Project::UpdateJobsets,
        );

    jobset_evaluate(path: web::Path<(String,String)>) =>
        Request::Jobset(
            handles::jobset(path.into_inner()),
            Jobset::Evaluate(true),
        );

    jobset_info(path: web::Path<(String,String)>) =>
        Request::Jobset(
            handles::jobset(path.into_inner()),
            Jobset::Info,
        );

    evaluation_cancel(path: web::Path<Uuid>) =>
        Request::Evaluation(
            handles::evaluation(path.into_inner()),
            Evaluation::Cancel,
        );

    evaluation_info(path: web::Path<Uuid>) =>
        Request::Evaluation(
            handles::evaluation(path.into_inner()),
            Evaluation::Info,
        );

    job_info(path: web::Path<(Uuid,String)>) =>
        Request::Job(
            handles::job(path.into_inner()),
            Job::Info,
        );

    //run_cancel(path: web::Path<(Uuid,String,u32)>) =>
    //    Request::Run(
    //        handles::run(path.into_inner()),
    //        Run::Cancel,
    //    );

    run_info(path: web::Path<(Uuid,String,u32)>) =>
        Request::Run(
            handles::run(path.into_inner()),
            Run::Info,
        );

    job_rerun(path: web::Path<(Uuid,String)>) =>
        Request::Job(
            handles::job(path.into_inner()),
            Job::Rerun,
        );

    build_info(path: web::Path<Uuid>) =>
        Request::Build(
            handles::build(path.into_inner()),
            Build::Info,
        );

    action_info(path: web::Path<Uuid>) =>
        Request::Action(
            handles::action(path.into_inner()),
            Action::Info,
        );

    login(body: web::Json<String>) =>
        Request::Login { password: body.into_inner() };
);

async fn dist(
    user: UserWrapper,
    path: web::Path<(Uuid, String, String)>,
) -> Result<impl Responder, ResponseErrorWrapper> {
    let (evaluation, job, path) = path.into_inner();
    let handle = handles::job((evaluation, job));
    let req = Request::Job(handle, Job::Info);
    let rsp = handle_request(user.0, req)
        .await
        .map_err(ResponseErrorWrapper)?;
    let info = match rsp {
        Response::JobInfo(info) => Ok(info),
        _ => Err(ResponseErrorWrapper(ResponseError::InternalError)),
    }?;
    if info.dist {
        Ok(NamedFile::open_async(format!("{}/{}", info.out, path)).await)
    } else {
        Err(ResponseErrorWrapper(ResponseError::BadRequest(
            "typhonDist is not set".into(),
        )))
    }
}

fn streaming_response(
    stream: impl futures_core::stream::Stream<Item = String> + 'static,
) -> HttpResponse {
    use futures::stream::StreamExt;
    let stream = stream.map(|x: String| {
        Ok::<_, actix_web::Error>(actix_web::web::Bytes::from(format!("{}\n", x)))
    });
    HttpResponse::Ok().streaming(stream)
}

mod log_routes {
    type Response = Result<Option<HttpResponse>, ResponseErrorWrapper>;
    use super::*;
    use handles::Log;

    async fn serve(log: Log) -> Response {
        let maybe_stream = web::block(move || typhon_core::log(log)).await??;
        Ok(maybe_stream.map(streaming_response))
    }
    pub async fn evaluation(path: web::Path<Uuid>) -> Response {
        serve(Log::Evaluation(handles::evaluation(path.into_inner()))).await
    }
    pub async fn build(path: web::Path<Uuid>) -> Response {
        serve(Log::Build(handles::build(path.into_inner()))).await
    }
    pub async fn action(path: web::Path<Uuid>) -> Response {
        serve(Log::Action(handles::action(path.into_inner()))).await
    }
    pub async fn generic(path: web::Json<Log>) -> Response {
        serve(path.into_inner()).await
    }
}

async fn raw_request(
    user: UserWrapper,
    body: web::Json<Request>,
) -> web::Json<Result<Response, ResponseError>> {
    web::Json(handle_request(user.0, body.into_inner()).await)
}

async fn events() -> Option<HttpResponse> {
    use futures::StreamExt;
    EVENT_LOGGER.listen().map(|stream| {
        HttpResponse::Ok()
            .content_type(actix_web::http::header::ContentType::plaintext())
            .streaming(stream.map(|x: typhon_types::Event| {
                Ok::<_, actix_web::Error>(actix_web::web::Bytes::from(format!(
                    "{}\n",
                    serde_json::to_string(&x).unwrap()
                )))
            }))
    })
}

async fn webhook(
    path: web::Path<String>,
    req: HttpRequest,
    body: String,
) -> Result<HttpResponse, ResponseErrorWrapper> {
    let input = typhon_core::webhooks::Input {
        headers: req
            .headers()
            .into_iter()
            .map(|(name, value)| {
                Ok((
                    name.as_str().to_string(),
                    std::str::from_utf8(value.as_bytes())
                        .map_err(|_| {
                            ResponseErrorWrapper(ResponseError::BadRequest(
                                "non-utf8 characters in request headers".to_string(),
                            ))
                        })?
                        .to_string(),
                ))
            })
            .collect::<Result<HashMap<_, _>, ResponseErrorWrapper>>()?,
        body,
    };

    let handle = handles::project(path.into_inner());
    let requests = web::block(move || typhon_core::webhook(handle, input)).await??;
    for req in requests {
        handle_request(User::Admin, req)
            .await
            .map_err(ResponseErrorWrapper)?;
    }
    Ok(HttpResponse::Ok().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("", web::post().to(raw_request))
            .route("/events", web::get().to(events))
            .route("/search", web::post().to(search))
            .route("/log", web::post().to(log_routes::generic))
            .service(
                web::scope("/builds/{build}")
                    .route("", web::get().to(build_info))
                    .route("/log", web::get().to(log_routes::build)),
            )
            .service(
                web::scope("/projects/{project}")
                    .route("", web::get().to(project_info))
                    .route("/create", web::post().to(create_project))
                    //.route("/delete", web::post().to(project_delete))
                    .route("/refresh", web::post().to(project_refresh))
                    .route("/update_jobsets", web::post().to(project_update_jobsets))
                    .route("/set_decl", web::post().to(project_set_decl))
                    .route("/webhook", web::post().to(webhook))
                    .service(
                        web::scope("/jobsets/{jobset}")
                            .route("", web::get().to(jobset_info))
                            .route("/evaluate", web::post().to(jobset_evaluate)),
                    ),
            )
            .service(
                web::scope("/evaluations/{evaluation}")
                    .route("", web::get().to(evaluation_info))
                    .route("/cancel", web::post().to(evaluation_cancel))
                    .route("/log", web::get().to(log_routes::evaluation))
                    .service(
                        web::scope("/jobs/{job}")
                            .route("", web::get().to(job_info))
                            .route("/rerun", web::get().to(job_rerun))
                            .route("/dist/{path:.*}", web::get().to(dist))
                            .service(
                                web::scope("/runs/{run}")
                                    //.route("/cancel", web::post().to(run_cancel))
                                    .route("", web::get().to(run_info)),
                            ),
                    ),
            )
            .service(
                web::scope("/actions/{action}")
                    .route("", web::get().to(action_info))
                    .route("/log", web::get().to(log_routes::action)),
            )
            .route("/login", web::post().to(login))
            .route(
                "{anything:.*}",
                web::route()
                    .guard(guard::Options())
                    .to(|| HttpResponse::Ok()),
            ),
    );
}
