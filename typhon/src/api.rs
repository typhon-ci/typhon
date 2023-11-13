use crate::actions::webhooks;
use crate::error;
use crate::requests::*;
use crate::{handle_request, handles, Msg, Response, ResponseError, User};
use crate::{live_log_action, live_log_build};
use crate::{EVENT_LOGGER, SETTINGS};

use actix_cors::Cors;
use actix_files::NamedFile;
use actix_web::{
    body::EitherBody, guard, http::StatusCode, web, HttpRequest, HttpResponse, Responder,
};
use tokio::sync::mpsc;

use std::collections::HashMap;

struct ResponseWrapper(crate::Response);
#[derive(Debug)]
struct ResponseErrorWrapper(crate::ResponseError);

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
        use crate::Response::*;
        match self.0 {
            Ok => web::Json(true).respond_to(req),
            ListEvaluations(payload) => web::Json(payload).respond_to(req),
            ListBuilds(payload) => web::Json(payload).respond_to(req),
            ListActions(payload) => web::Json(payload).respond_to(req),
            ListRuns(payload) => web::Json(payload).respond_to(req),
            ListProjects(payload) => web::Json(payload).respond_to(req),
            ProjectInfo(payload) => web::Json(payload).respond_to(req),
            JobsetInfo(payload) => web::Json(payload).respond_to(req),
            JobsetEvaluate(payload) => web::Json(payload).respond_to(req),
            EvaluationInfo(payload) => web::Json(payload).respond_to(req),
            JobInfo(payload) => web::Json(payload).respond_to(req),
            BuildInfo(payload) => web::Json(payload).respond_to(req),
            ActionInfo(payload) => web::Json(payload).respond_to(req),
            RunInfo(payload) => web::Json(payload).respond_to(req),
            Log(payload) => web::Json(payload).respond_to(req),
            Login { token } => web::Json(token).respond_to(req),
        }
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
    async fn $name (sender: web::Data<mpsc::Sender<Msg>>, user: User, $($i : $t),*) -> Result<ResponseWrapper, ResponseErrorWrapper> {
        handle_request((**sender).clone(), user, $e).await.map(ResponseWrapper).map_err(ResponseErrorWrapper)
    } r!( $($rest)* );
    };
    (  ) => {}
}

r!(
    list_evaluations(body: web::Json<EvaluationSearch>) =>
        Request::ListEvaluations(body.into_inner());

    create_project(path: web::Path<String>, body: web::Json<ProjectDecl>) => {
        let name = path.into_inner();
        let decl = body.into_inner();
        Request::CreateProject { name, decl }
    };

    list_projects() => Request::ListProjects;

    project_delete(path: web::Path<String>) =>
        Request::Project(
            handles::project(path.into_inner()),
            Project::Delete,
        );

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

    project_set_private_key(path: web::Path<String>, body: web::Json<String>) =>
        Request::Project(
            handles::project(path.into_inner()),
            Project::SetPrivateKey(body.into_inner()),
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

    evaluation_cancel(path: web::Path<(String,u64)>) =>
        Request::Evaluation(
            handles::evaluation(path.into_inner()),
            Evaluation::Cancel,
        );

    evaluation_info(path: web::Path<(String,u64)>) =>
        Request::Evaluation(
            handles::evaluation(path.into_inner()),
            Evaluation::Info,
        );

    evaluation_log(path: web::Path<(String,u64)>) =>
        Request::Evaluation(
            handles::evaluation(path.into_inner()),
            Evaluation::Log,
        );

    job_info(path: web::Path<(String,u64,String,String)>) =>
        Request::Job(
            handles::job(path.into_inner()),
            Job::Info,
        );

    run_cancel(path: web::Path<(String,u64,String,String,u64)>) =>
        Request::Run(
            handles::run(path.into_inner()),
            Run::Cancel,
        );

    run_info(path: web::Path<(String,u64,String,String,u64)>) =>
        Request::Run(
            handles::run(path.into_inner()),
            Run::Info,
        );

    build_info(path: web::Path<(String,u64)>) =>
        Request::Build(
            handles::build(path.into_inner()),
            Build::Info,
        );

    build_log(path: web::Path<(String,u64)>) =>
        Request::Build(
            handles::build(path.into_inner()),
            Build::Log,
        );

    action_info(path: web::Path<(String,u64)>) =>
        Request::Action(
            handles::action(path.into_inner()),
            Action::Info,
        );

    action_log(path: web::Path<(String,u64)>) =>
        Request::Action(
            handles::action(path.into_inner()),
            Action::Log,
        );

    login(body: web::Json<String>) =>
        Request::Login(body.into_inner());
);

async fn dist(
    sender: web::Data<mpsc::Sender<Msg>>,
    user: User,
    path: web::Path<(String, u64, String, String, String)>,
) -> Result<impl Responder, ResponseErrorWrapper> {
    let (project, evaluation, system, job, path) = path.into_inner();
    let handle = handles::job((project, evaluation, system, job));
    let req = Request::Job(handle, Job::Info);
    let rsp = handle_request((**sender).clone(), user, req)
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

async fn build_live_log(
    path: web::Path<(String, u64)>,
) -> Result<Option<HttpResponse>, ResponseErrorWrapper> {
    let handle = handles::build(path.into_inner());
    let maybe_stream = web::block(move || live_log_build(handle)).await??;
    Ok(maybe_stream.map(streaming_response))
}

async fn action_live_log(
    path: web::Path<(String, u64)>,
) -> Result<Option<HttpResponse>, ResponseErrorWrapper> {
    let handle = handles::action(path.into_inner());
    let maybe_stream = web::block(move || live_log_action(handle)).await??;
    Ok(maybe_stream.map(streaming_response))
}

async fn raw_request(
    sender: web::Data<mpsc::Sender<Msg>>,
    user: User,
    body: web::Json<Request>,
) -> web::Json<Result<Response, ResponseError>> {
    web::Json(handle_request((**sender).clone(), user, body.into_inner()).await)
}

async fn events() -> Option<HttpResponse> {
    use futures::StreamExt;
    EVENT_LOGGER.listen_async().await.map(|stream| {
        HttpResponse::Ok().streaming(stream.map(|x: typhon_types::Event| {
            Ok::<_, actix_web::Error>(actix_web::web::Bytes::from(format!(
                "{}\n",
                serde_json::to_string(&x).unwrap()
            )))
        }))
    })
}

async fn webhook(
    sender: web::Data<mpsc::Sender<Msg>>,
    path: web::Path<String>,
    req: HttpRequest,
    body: String,
) -> Result<HttpResponse, ResponseErrorWrapper> {
    let input = webhooks::Input {
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
    let requests = crate::webhook(handle, input)?;
    for req in requests {
        handle_request((**sender).clone(), User::Admin, req)
            .await
            .map_err(ResponseErrorWrapper)?;
    }
    Ok(HttpResponse::Ok().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    let cors = Cors::permissive(); // TODO: configure
    cfg.service(
        web::scope(&format!("{}/api", SETTINGS.webroot))
            .route("", web::post().to(raw_request))
            .route("/events", web::get().to(events))
            .route("/evaluations", web::post().to(list_evaluations))
            .route("/projects", web::get().to(list_projects))
            .service(
                web::scope("/builds/{drv}/{num}")
                    .route("", web::get().to(build_info))
                    .route("/log", web::get().to(build_log))
                    .route("/live_log", web::get().to(build_live_log)),
            )
            .service(
                web::scope("/projects/{project}")
                    .route("", web::get().to(project_info))
                    .route("/create", web::post().to(create_project))
                    .route("/delete", web::post().to(project_delete))
                    .route("/refresh", web::post().to(project_refresh))
                    .route("/update_jobsets", web::post().to(project_update_jobsets))
                    .route("/set_decl", web::post().to(project_set_decl))
                    .route("/set_private_key", web::post().to(project_set_private_key))
                    .route("/webhook", web::post().to(webhook))
                    .service(
                        web::scope("/jobsets/{jobset}")
                            .route("", web::get().to(jobset_info))
                            .route("/evaluate", web::post().to(jobset_evaluate)),
                    )
                    .service(
                        web::scope("/evaluations/{evaluation}")
                            .route("", web::get().to(evaluation_info))
                            .route("/cancel", web::post().to(evaluation_cancel))
                            .route("/log", web::get().to(evaluation_log))
                            .service(
                                web::scope("/jobs/{system}/{job}")
                                    .route("", web::get().to(job_info))
                                    .route("/dist/{path:.*}", web::get().to(dist))
                                    .service(
                                        web::scope("/runs/{run}")
                                            .route("", web::get().to(run_info))
                                            .route("/cancel", web::post().to(run_cancel)),
                                    ),
                            ),
                    )
                    .service(
                        web::scope("/actions/{action}")
                            .route("", web::get().to(action_info))
                            .route("/log", web::get().to(action_log))
                            .route("/live_log", web::get().to(action_live_log)),
                    ),
            )
            .route("/login", web::post().to(login))
            .route(
                "{anything:.*}",
                web::route()
                    .guard(guard::Options())
                    .to(|| HttpResponse::Ok()),
            )
            .wrap(cors),
    );
}
