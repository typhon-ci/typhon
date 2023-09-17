use colored::Colorize;
use std::{thread::sleep, time::Duration};

use async_trait::async_trait;
use typhon_types::*;

#[async_trait]
trait SendRequest {
    async fn send(&self) -> Result<responses::Response, responses::ResponseError>;
}

#[async_trait]
impl SendRequest for requests::Request {
    async fn send(&self) -> Result<responses::Response, responses::ResponseError> {
        let mut res = surf::post("http://localhost:8000/api")
            .body_json(self)
            .unwrap()
            .header("token", "password")
            .await
            .unwrap();
        res.body_json().await.unwrap()
    }
}

use requests::Request as Req;
use responses::Response as Res;

macro_rules! p {
    (let $pat:pat = $e:expr) => {
        let value = $e;
        let $pat = value
                else {
                    println!("{}", " failed!".red());
                    println!("     {} {}", "Expected something of the shape".red(), stringify!($pat).green().bold());
                    println!("     {}", format!("Got instead: \n\n{}\n", format!("{:#?}", value).bold()).red());
                    panic!()
                };
    };
}
use std::io::Write;
macro_rules! s {
    (Err($pat:pat) = $e:expr) => {
        p!(let Err($pat) = $e.send().await)
    };
    ($pat:pat = $e:expr) => {
        let request = $e;
        print!(" • {}...", request.explain());
        std::io::stdout().flush().unwrap();
        p!(let Ok($pat) = request.send().await);
        println!("{}", " done".green());
    };
}

async fn create_project(name: &String, decl: String) {
    s!(Res::Ok = Req::CreateProject {
        handle: handles::project(name.clone()),
        decl
    });
}

trait Explain {
    fn explain(&self) -> String;
}
impl Explain for Req {
    fn explain(&self) -> String {
        match self {
            Req::CreateProject { handle, decl } => format!(
                "Create the project {} with declaration {}",
                handle.project.bold(),
                decl.bold()
            ),
            Req::Project(proj, requests::Project::Refresh) => {
                format!("Refresh project {}", proj.project.bold())
            }
            Req::Project(proj, requests::Project::UpdateJobsets) => {
                format!("Update the jobsets from project {}", proj.project.bold())
            }
            Req::Jobset(jobset, requests::Jobset::Evaluate(force)) => {
                format!(
                    "{} the jobset {} from project {}",
                    if *force {
                        "Forcefully evaluate"
                    } else {
                        "Evaluate"
                    },
                    jobset.jobset.bold(),
                    jobset.project.project.bold()
                )
            }
            Req::Evaluation(evaluation, requests::Evaluation::Info) => {
                format!(
                    "Fetch the information of evaluation {}",
                    format!("{}", evaluation).bold()
                )
            }

            req => format!("Asking the API {}", format!("{:#?}", req).bold()),
        }
    }
}

#[tokio::main]
async fn main() {
    let name = "test".to_string();
    create_project(
        &name,
        std::env::var("PROJECT_DECL").unwrap_or("path:../tests/empty".into()),
    )
    .await;

    s!(Res::Ok = Req::Project(handles::project(name.clone()), requests::Project::Refresh));

    s!(Res::ProjectUpdateJobsets(jobsets) = Req::Project(
        handles::project(name.clone()),
        requests::Project::UpdateJobsets
    ));

    p!(let [jobset] = jobsets.as_slice());
    let jobset = handles::jobset((name.to_string(), jobset.to_string()));
    s!(Res::JobsetEvaluate(evaluation) = Req::Jobset(jobset, requests::Jobset::Evaluate(true)));
    let mut status = "pending".to_string();
    let mut elapsed_time: u64 = 0;
    while status == "pending" {
        println!("Query evaluation's status...");
        s!(Res::EvaluationInfo(responses::EvaluationInfo {
            status: new_status,
            ..
        }) = Req::Evaluation(evaluation.clone(), requests::Evaluation::Info));
        status = new_status;
        println!("   > status is '{}'", status.bold());
        const WAIT_TIME_MS: u64 = 200;
        const MAX_TIME_SEC: u64 = 60;
        sleep(Duration::from_millis(WAIT_TIME_MS));
        elapsed_time += WAIT_TIME_MS;
        if elapsed_time > MAX_TIME_SEC * 1000 {
            panic!(
                "API is not responding: evaluation took more than {} seconds.",
                MAX_TIME_SEC
            );
        }
    }
}
