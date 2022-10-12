use crate::requests::*;
use crate::{handle_request, handles, ResponseError, User};
use rocket::serde::json::Json;
use rocket::{get, post, routes, Route};

struct ResponseWrapper(crate::Response);

impl<'r> rocket::response::Responder<'r, 'static> for ResponseWrapper {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        use crate::Response::*;
        match self.0 {
            Ok => Json(true).respond_to(req),
            ListProjects(payload) => Json(payload).respond_to(req),
            ProjectInfo(payload) => Json(payload).respond_to(req),
            ProjectUpdateJobsets(payload) => Json(payload).respond_to(req),
            JobsetInfo(payload) => Json(payload).respond_to(req),
            JobsetEvaluate(payload) => Json(payload).respond_to(req),
            EvaluationInfo(payload) => Json(payload).respond_to(req),
            JobInfo(payload) => Json(payload).respond_to(req),
            BuildInfo(payload) => Json(payload).respond_to(req),
            // BuildLog(payload) => payload.respond_to(req),
            BuildLog => Json("todo").respond_to(req),
        }
    }
}

/// A macro to generate api endpoints
macro_rules! r {
    ($_:tt $attr:tt $name: ident($($i: ident : $t: ty),*) => $e: expr
     ;$($rest: tt)*
    ) => {
    #$attr async fn $name (user: User, $($i : $t),*) -> Result<ResponseWrapper, ResponseError> {
        handle_request(user, $e).map(ResponseWrapper)
    } r!( $($rest)* );
    };
    (  ) => {}
}

r!(
    #[post("/create_project/<project>")]
    create_project(project: String)
        => Request::CreateProject( handles::Project { project });

    #[get("/list_projects")]
    list_projects() => Request::ListProjects;

    #[post("/projects/<project>/delete")]
    project_delete(project: String)
        => Request::Project(
            handles::Project { project },
            Project::Delete,
        );

    #[get("/projects/<project>")]
    project_info(project: String)
        => Request::Project(
            handles::Project { project },
            Project::Info,
        );

    #[post("/projects/<project>/refresh")]
    project_refresh(project: String)
        => Request::Project(
            handles::Project { project },
            Project::Refresh,
        );

    #[post("/projects/<project>/set_decl", format = "application/json", data = "<body>")]
    project_set_decl(project: String, body: Json<String>)
        => Request::Project(
            handles::Project { project },
            Project::SetDecl(body.into_inner()),
        );

    #[post("/projects/<project>/set_private_key", format = "application/json", data = "<body>")]
    project_set_private_key(project: String, body: Json<String>)
        => Request::Project(
            handles::Project { project },
            Project::SetPrivateKey(body.into_inner()),
        );

    #[post("/projects/<project>/update_jobsets")]
    project_update_jobsets(project: String)
        => Request::Project(
            handles::Project { project },
            Project::UpdateJobsets,
        );

    #[post("/projects/<project>/jobsets/<jobset>/evaluate")]
    jobset_evaluate(project: String, jobset: String)
        => Request::Jobset(
            handles::Jobset { project, jobset },
            Jobset::Evaluate,
        );

    #[get("/projects/<project>/jobsets/<jobset>")]
    jobset_info(project: String, jobset: String)
        => Request::Jobset(
            handles::Jobset { project, jobset },
            Jobset::Info,
        );

    #[post("/projects/<project>/jobsets/<jobset>/evaluations/<evaluation>/cancel")]
    evaluation_cancel(project: String, jobset: String, evaluation: i32)
        => Request::Evaluation(
            handles::Evaluation { project, jobset, evaluation },
            Evaluation::Cancel,
        );

    #[get("/projects/<project>/jobsets/<jobset>/evaluations/<evaluation>")]
    evaluation_info(project: String, jobset: String,evaluation: i32)
        => Request::Evaluation(
            handles::Evaluation { project, jobset, evaluation },
            Evaluation::Info,
        );

    #[post("/projects/<project>/jobsets/<jobset>/evaluations/<evaluation>/jobs/<job>/cancel")]
    job_cancel(project: String, jobset: String, evaluation: i32, job: String)
        => Request::Job(
            handles::Job { project, jobset, evaluation, job },
            Job::Cancel,
        );

    #[get("/projects/<project>/jobsets/<jobset>/evaluations/<evaluation>/jobs/<job>")]
    job_info(project: String, jobset: String, evaluation: i32, job: String)
        => Request::Job(
            handles::Job { project, jobset, evaluation, job },
            Job::Info,
        );

    #[post("/builds/<build_hash>/cancel")]
    build_cancel(build_hash: String)
        => Request::Build(
            handles::Build { build_hash },
            Build::Cancel,
        );

    #[get("/builds/<build_hash>")]
    build_info(build_hash: String)
        => Request::Build(
            handles::Build { build_hash },
            Build::Info,
        );

    #[get("/builds/<build_hash>/log")]
    build_log(build_hash: String)
        => Request::Build(
            handles::Build { build_hash },
            Build::Log,
        );
);

#[get("/raw-request", format = "application/json", data = "<body>")]
async fn raw_request(
    user: User,
    body: Json<crate::requests::Request>,
) -> Result<Json<crate::Response>, ResponseError> {
    handle_request(user, body.into_inner()).map(Json)
}

pub fn routes() -> Vec<Route> {
    routes![
        list_projects,
        create_project,
        project_delete,
        project_info,
        project_refresh,
        project_set_decl,
        project_set_private_key,
        project_update_jobsets,
        jobset_evaluate,
        jobset_info,
        evaluation_cancel,
        evaluation_info,
        job_cancel,
        job_info,
        build_cancel,
        build_info,
        build_log,
        raw_request,
    ]
}
