use crate::actions;
use crate::builds;
use crate::error::Error;
use crate::handles;
use crate::log_event;
use crate::models;
use crate::responses;
use crate::schema;
use crate::tasks;
use crate::Conn;
use crate::POOL;
use crate::RUNS;

use futures_core::Stream;
use std::path::Path;
use tokio::sync::mpsc;

mod run {
    use super::*;
    #[ext_trait::extension(pub trait RunExt)]
    impl handles::Run {
        fn info(&self, conn: &mut Conn) -> Result<responses::RunInfo, Error> {
            todo!()
        }

        fn cancel(&self, conn: &mut Conn) -> Result<(), Error> {
            todo!()
        }
    }
}

/// A context is a set of inputs required to perform some
/// operation. This set of inputs can be fetched from the database
/// from a handle (of type `Self::Handle`).
trait Context: Sized {
    type Handle;
    /// Fetches the context from the database
    fn new(handle: Self::Handle, conn: &mut Conn) -> Result<Self, Error>;

    type Output: Sized;
    /// Performs the operation
    fn call(self, conn: &mut Conn) -> Result<Self::Output, Error>;
    fn call_from_handle(handle: Self::Handle, conn: &mut Conn) -> Result<Self::Output, Error> {
        let this = Self::new(handle, conn)?;
        this.call(conn)
    }
}

mod job {
    use super::*;

    /// Spawns a new run of a given job (a build and two actions)
    /// Takes models since it is called internally.
    struct NewRun {
        project: models::Project,
        evaluation: models::Evaluation,
        job: models::Job,
    }

    impl Context for NewRun {
        type Handle = handles::Job;
        type Output = handles::Run;
        fn new(handle: Self::Handle, conn: &mut Conn) -> Result<Self, Error> {
            todo!()
        }
        fn call(self, conn: &mut Conn) -> Result<Self::Output, Error> {
            todo!()
        }
    }

    #[ext_trait::extension(pub trait JobExt)]
    impl handles::Job {
        fn info(&self, conn: &mut Conn) -> Result<responses::JobInfo, Error> {
            todo!()
        }
    }
}

pub type LogSink = mpsc::UnboundedSender<String>;
mod task {
    use super::*;

    pub fn log(task: i32, conn: &mut Conn) -> Result<impl Stream<Item = String>, Error> {
        todo!();
        Ok(async_stream::stream! {
            yield "hello".to_string();
        })
    }

    // pub fn create() {

    // }

    /// Cancel a task which is currently running in the task manager
    fn cancel(task: i32, conn: &mut Conn) -> Result<(), Error> {
        todo!()
    }
}
mod action {
    use super::*;

    // pub fn refresh(project: models::Project, conn: &mut Conn) -> Result<(), Error> {
    //     todo!()
    // }

    /// `action_dir` is a directory containing:
    /// 1. a number of executables (aka the actions);
    /// 2. a `secret` file.
    /// Among the executables, one should be named `action_name`.
    /// `input` and the secrets constitute the payload given to the
    /// executable `action_name` as stdin.
    ///
    /// In case of success, returns the JSON stdout of the command.
    pub fn spawn(
        input: serde_json::Value,
        action_dir: &Path,
        action_name: &str,
        logs: LogSink,
    ) -> Result<serde_json::Value, Error> {
        todo!()
    }

    #[ext_trait::extension(pub trait JobsetExt)]
    impl handles::Project {
        fn info(&self, conn: &mut Conn) -> Result<responses::ActionInfo, Error> {
            todo!()
        }
    }
}

// mod project {
//     use super::*;

//     pub fn refresh(project: models::Project, conn: &mut Conn) -> Result<(), Error> {
//         todo!()
//     }

//     #[ext_trait::extension(pub trait JobsetExt)]
//     impl handles::Project {
//         fn info(&self, conn: &mut Conn) -> Result<responses::ProjectInfo, Error> {
//             todo!()
//         }
//     }
// }

mod jobset {
    use super::*;

    struct Evaluate {
        project: models::Project,
        jobset: models::Jobset,
    }

    impl Context for Evaluate {
        type Handle = handles::Jobset;
        type Output = handles::Evaluation;
        fn new(handle: Self::Handle, conn: &mut Conn) -> Result<Self, Error> {
            todo!()
        }
        fn call(self, conn: &mut Conn) -> Result<Self::Output, Error> {
            todo!()
        }
    }

    #[ext_trait::extension(pub trait JobsetExt)]
    impl handles::Jobset {
        fn info(&self, conn: &mut Conn) -> Result<responses::JobsetInfo, Error> {
            todo!()
        }
        fn delete(&self, conn: &mut Conn) -> Result<(), Error> {
            todo!()
        }
    }
}
mod evaluation {
    use super::*;

    #[ext_trait::extension(pub trait EvaluationExt)]
    impl handles::Evaluation {
        fn info(&self, conn: &mut Conn) -> Result<responses::EvaluationInfo, Error> {
            todo!()
        }
        /// Cancel the nix process that evaluates a jobset
        fn cancel(&self, conn: &mut Conn) -> Result<(), Error> {
            todo!()
        }
    }
}
