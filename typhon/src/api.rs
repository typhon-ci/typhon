use crate::listeners::Session;
use crate::requests::*;
use crate::SETTINGS;
use crate::{handle_request, handles, Response, ResponseError, User};
use actix_cors::Cors;
use actix_web::{
    body::EitherBody, guard, http::StatusCode, web, Error, HttpRequest, HttpResponse, Responder,
};
use actix_web_actors::ws;

struct ResponseWrapper(crate::Response);
#[derive(Debug)]
struct ResponseErrorWrapper(crate::ResponseError);

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
            ListProjects(payload) => web::Json(payload).respond_to(req),
            ProjectInfo(payload) => web::Json(payload).respond_to(req),
            ProjectUpdateJobsets(payload) => web::Json(payload).respond_to(req),
            JobsetInfo(payload) => web::Json(payload).respond_to(req),
            JobsetEvaluate(payload) => web::Json(payload).respond_to(req),
            EvaluationInfo(payload) => web::Json(payload).respond_to(req),
            JobInfo(payload) => web::Json(payload).respond_to(req),
            BuildInfo(payload) => web::Json(payload).respond_to(req),
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
    async fn $name (user: User, $($i : $t),*) -> Result<ResponseWrapper, ResponseErrorWrapper> {
        handle_request(user, $e).await.map(ResponseWrapper).map_err(ResponseErrorWrapper)
    } r!( $($rest)* );
    };
    (  ) => {}
}

r!(
    create_project(path: web::Path<String>, body: web::Json<String>) => {
        let handle = handles::project(path.into_inner());
        let decl = body.into_inner();
        Request::CreateProject { handle, decl }
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

    project_set_decl(path: web::Path<String>, body: web::Json<String>) =>
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

    evaluation_cancel(path: web::Path<(String,String,i32)>) =>
        Request::Evaluation(
            handles::evaluation(path.into_inner()),
            Evaluation::Cancel,
        );

    evaluation_info(path: web::Path<(String,String,i32)>) =>
        Request::Evaluation(
            handles::evaluation(path.into_inner()),
            Evaluation::Info,
        );

    evaluation_log(path: web::Path<(String, String,i32)>) =>
        Request::Evaluation(
            handles::evaluation(path.into_inner()),
            Evaluation::Log,
        );

    job_cancel(path: web::Path<(String,String,i32,String)>) =>
        Request::Job(
            handles::job(path.into_inner()),
            Job::Cancel,
        );

    job_info(path: web::Path<(String,String,i32,String)>) =>
        Request::Job(
            handles::job(path.into_inner()),
            Job::Info,
        );

    job_log_begin(path: web::Path<(String,String,i32,String)>) =>
        Request::Job(
            handles::job(path.into_inner()),
            Job::LogBegin,
        );

    job_log_end(path: web::Path<(String,String,i32,String)>) =>
        Request::Job(
            handles::job(path.into_inner()),
            Job::LogEnd,
        );

    build_cancel(path: web::Path<String>) =>
        Request::Build(
            handles::build(path.into_inner()),
            Build::Cancel,
        );

    build_info(path: web::Path<String>) =>
        Request::Build(
            handles::build(path.into_inner()),
            Build::Info,
        );

    build_nix_log(path: web::Path<String>) =>
        Request::Build(
            handles::build(path.into_inner()),
            Build::NixLog,
        );

    login(body: web::Json<String>) =>
        Request::Login(body.into_inner());
);

async fn raw_request(
    user: User,
    body: web::Json<Request>,
) -> web::Json<Result<Response, ResponseError>> {
    web::Json(handle_request(user, body.into_inner()).await)
}

async fn events(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(Session::new(), &req, stream)
}

pub fn config(cfg: &mut web::ServiceConfig) {
    let cors = Cors::permissive(); // TODO: configure
    cfg.service(
        web::scope(&format!("{}/api", SETTINGS.get().unwrap().webroot))
            .route("", web::post().to(raw_request))
            .route("/events", web::get().to(events))
            .route("/projects", web::get().to(list_projects))
            .service(
                web::scope("/projects/{project}")
                    .route("", web::get().to(project_info))
                    .route("/create", web::post().to(create_project))
                    .route("/delete", web::post().to(project_delete))
                    .route("/refresh", web::post().to(project_refresh))
                    .route("/update_jobsets", web::post().to(project_update_jobsets))
                    .route("/set_decl", web::post().to(project_set_decl))
                    .route("/set_private_key", web::post().to(project_set_private_key))
                    .service(
                        web::scope("/jobsets/{jobset}")
                            .route("", web::get().to(jobset_info))
                            .route("/evaluate", web::post().to(jobset_evaluate))
                            .service(
                                web::scope("/evaluations/{evaluation}")
                                    .route("", web::get().to(evaluation_info))
                                    .route("/cancel", web::post().to(evaluation_cancel))
                                    .route("/log", web::get().to(evaluation_log))
                                    .service(
                                        web::scope("/jobs/{job}")
                                            .route("", web::get().to(job_info))
                                            .route("/cancel", web::post().to(job_cancel))
                                            .route("/logs/begin", web::get().to(job_log_begin))
                                            .route("/logs/end", web::get().to(job_log_end)),
                                    ),
                            ),
                    ),
            )
            .service(
                web::scope("/builds/{build}")
                    .route("", web::get().to(build_info))
                    .route("/cancel", web::post().to(build_cancel))
                    .route("/nixlog", web::get().to(build_nix_log)),
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
