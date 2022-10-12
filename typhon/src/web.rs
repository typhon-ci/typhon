//use crate::*;
//
//use askama::Template;
//use rocket::form::Form;
//use rocket::response::Redirect;
//use rocket::*;
//
//struct Tmp<T>(T); // a wrapper type to implement rocket::response::Responder
//
//impl<'r, T: Template> response::Responder<'r, 'static> for Tmp<T> {
//    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
//        use rocket::response::content::RawHtml;
//        let Tmp(template) = self;
//        let rsp = template
//            .render()
//            .map_err(|_| rocket::http::Status::InternalServerError)?;
//        RawHtml(rsp).respond_to(req)
//    }
//}
//
//#[derive(FromForm)]
//struct FormLogIn {
//    password: String,
//}
//
//#[post("/login", data = "<input>")]
//async fn login(jar: &http::CookieJar<'_>, input: Form<FormLogIn>) -> Result<(), RspError> {
//    let hash = sha256::digest(input.password.clone());
//    if hash == SETTINGS.get().unwrap().hashed_password {
//        jar.add_private(http::Cookie::new("admin", ""));
//        Ok(())
//    } else {
//        Err(RspError::BadRequest("Wrong password".to_string()))
//    }
//}
//
//#[post("/logout")]
//async fn logout(jar: &http::CookieJar<'_>) -> () {
//    jar.remove_private(http::Cookie::named("admin"))
//}
//
//#[derive(Template)]
//#[template(path = "homepage.html")]
//struct HomepageTemplate {
//    admin: bool,
//    root: String,
//    projects: Vec<String>,
//}
//
//#[get("/")]
//async fn homepage(user: User) -> Result<Tmp<HomepageTemplate>, RspError> {
//    let req = Req::ListProjects;
//    let rsp = handle_request(user, req)?;
//    let r = match rsp {
//        Rsp::ListProjects(r) => r,
//        _ => unreachable!(),
//    };
//    Ok(Tmp(HomepageTemplate {
//        admin: user.is_admin(),
//        root: SETTINGS.get().unwrap().webroot.clone(),
//        projects: r,
//    }))
//}
//
//#[derive(FromForm)]
//struct FormCreateProject {
//    project_name: String,
//}
//
//#[post("/create_project", data = "<input>")]
//async fn create_project(user: User, input: Form<FormCreateProject>) -> Result<Redirect, RspError> {
//    let req = Req::CreateProject(input.project_name.clone());
//    let rsp = handle_request(user, req)?;
//    match rsp {
//        Rsp::Ok => (),
//        _ => unreachable!(),
//    };
//    Ok(Redirect::to(uri!(homepage)))
//}
//
//#[derive(Template)]
//#[template(path = "project.html")]
//struct ProjectTemplate {
//    admin: bool,
//    root: String,
//    project_name: String,
//    project_info: crate::projects::ProjectInfo,
//}
//
//#[get("/projects/<project_name>")]
//async fn project(user: User, project_name: String) -> Result<Tmp<ProjectTemplate>, RspError> {
//    let req = Req::ProjectInfo(project_name.clone());
//    let rsp = handle_request(user, req)?;
//    let project_info = match rsp {
//        Rsp::ProjectInfo(r) => r,
//        _ => unreachable!(),
//    };
//    Ok(Tmp(ProjectTemplate {
//        admin: user.is_admin(),
//        root: SETTINGS.get().unwrap().webroot.clone(),
//        project_name: project_name,
//        project_info: project_info,
//    }))
//}
//
//#[derive(FromForm)]
//struct FormProjectSetDecl {
//    flake: String,
//}
//
//#[post("/projects/<project_name>/set_decl", data = "<input>")]
//async fn project_set_decl(
//    user: User,
//    project_name: String,
//    input: Form<FormProjectSetDecl>,
//) -> Result<Redirect, RspError> {
//    let req = Req::ProjectSetDecl(project_name.clone(), input.flake.clone());
//    let rsp = handle_request(user, req)?;
//    match rsp {
//        Rsp::Ok => (),
//        _ => unreachable!(),
//    };
//    Ok(Redirect::to(uri!(project(project_name))))
//}
//
//#[derive(FromForm)]
//struct FormProjectSetPrivateKey {
//    private_key: String,
//}
//
//#[post("/projects/<project_name>/set_private_key", data = "<input>")]
//async fn project_set_private_key(
//    user: User,
//    project_name: String,
//    input: Form<FormProjectSetPrivateKey>,
//) -> Result<Redirect, RspError> {
//    let req = Req::ProjectSetPrivateKey(project_name.clone(), input.private_key.clone());
//    let rsp = handle_request(user, req)?;
//    match rsp {
//        Rsp::Ok => (),
//        _ => unreachable!(),
//    };
//    Ok(Redirect::to(uri!(project(project_name))))
//}
//
//#[post("/projects/<project_name>/refresh")]
//async fn project_refresh(user: User, project_name: String) -> Result<Redirect, RspError> {
//    let req = Req::ProjectRefresh(project_name.clone());
//    let rsp = handle_request(user, req)?;
//    match rsp {
//        Rsp::Ok => (),
//        _ => unreachable!(),
//    };
//    Ok(Redirect::to(uri!(project(project_name))))
//}
//
//#[post("/projects/<project_name>/update_jobsets")]
//async fn project_update_jobsets(user: User, project_name: String) -> Result<Redirect, RspError> {
//    let req = Req::ProjectUpdateJobsets(project_name.clone());
//    let rsp = handle_request(user, req)?;
//    match rsp {
//        Rsp::Ok => (),
//        _ => unreachable!(),
//    };
//    Ok(Redirect::to(uri!(project(project_name))))
//}
//
//#[post("/projects/<project_name>/delete")]
//async fn project_delete(user: User, project_name: String) -> Result<Redirect, RspError> {
//    let req = Req::ProjectDelete(project_name.clone());
//    let rsp = handle_request(user, req)?;
//    match rsp {
//        Rsp::Ok => (),
//        _ => unreachable!(),
//    };
//    Ok(Redirect::to(uri!(homepage)))
//}
//
//#[derive(Template)]
//#[template(path = "jobset.html")]
//struct JobsetTemplate {
//    admin: bool,
//    root: String,
//    project_name: String,
//    jobset_name: String,
//    jobset_info: crate::jobsets::JobsetInfo,
//}
//
//#[get("/projects/<project_name>/jobsets/<jobset_name>")]
//async fn jobset(
//    user: User,
//    project_name: String,
//    jobset_name: String,
//) -> Result<Tmp<JobsetTemplate>, RspError> {
//    let req = Req::JobsetInfo(project_name.clone(), jobset_name.clone());
//    let rsp = handle_request(user, req)?;
//    let jobset_info = match rsp {
//        Rsp::JobsetInfo(r) => r,
//        _ => unreachable!(),
//    };
//    Ok(Tmp(JobsetTemplate {
//        admin: user.is_admin(),
//        root: SETTINGS.get().unwrap().webroot.clone(),
//        project_name: project_name.clone(),
//        jobset_name: jobset_name.clone(),
//        jobset_info: jobset_info,
//    }))
//}
//
//#[post("/projects/<project_name>/jobsets/<jobset_name>/evaluate")]
//async fn jobset_evaluate(
//    user: User,
//    project_name: String,
//    jobset_name: String,
//) -> Result<Redirect, RspError> {
//    let req = Req::JobsetEvaluate(project_name.clone(), jobset_name.clone());
//    let rsp = handle_request(user, req)?;
//    match rsp {
//        Rsp::Ok => (),
//        _ => unreachable!(),
//    };
//    Ok(Redirect::to(uri!(jobset(project_name, jobset_name))))
//}
//
//#[derive(Template)]
//#[template(path = "evaluation.html")]
//struct EvaluationTemplate {
//    admin: bool,
//    root: String,
//    evaluation_num: i32,
//    evaluation_info: crate::evaluations::EvaluationInfo,
//    jobs: Vec<(String, String, String)>,
//}
//
//#[get("/projects/<project_name>/jobsets/<jobset_name>/evaluations/<evaluation_num>")]
//async fn evaluation(
//    user: User,
//    project_name: String,
//    jobset_name: String,
//    evaluation_num: i32,
//) -> Result<Tmp<EvaluationTemplate>, RspError> {
//    let req = Req::EvaluationInfo(project_name, jobset_name, evaluation_num);
//    let rsp = handle_request(user, req)?;
//    let evaluation_info = match rsp {
//        Rsp::EvaluationInfo(r) => r,
//        _ => unreachable!(),
//    };
//    Ok(Tmp(EvaluationTemplate {
//        admin: user.is_admin(),
//        root: SETTINGS.get().unwrap().webroot.clone(),
//        evaluation_num: evaluation_num,
//        evaluation_info: evaluation_info,
//        jobs: vec![], // TODO
//    }))
//}
//
//pub fn routes() -> Vec<Route> {
//    routes![
//    //       login,
//    //       logout,
//    //       homepage,
//    //       create_project,
//    //       project,
//    //       project_set_decl,
//    //       project_set_private_key,
//    //       project_refresh,
//    //       project_update_jobsets,
//    //       project_delete,
//    //       jobset,
//    //       jobset_evaluate,
//    //       evaluation,
//       ]
//}
//
//mod filters {
//    pub fn strftime(time: &i64) -> ::askama::Result<String> {
//        Ok(crate::time::strftime(*time))
//    }
//}
