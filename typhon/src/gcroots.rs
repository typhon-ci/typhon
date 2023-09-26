use crate::models::*;
use crate::schema::builds::dsl::*;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::schema::jobsets::dsl::*;
use crate::schema::projects::dsl::*;
use diesel::prelude::*;

use std::collections::HashSet;
use std::fs::{remove_dir_all, rename, DirBuilder};
use std::os::unix::fs::symlink;
use std::path::Path;

#[derive(Debug)]
enum Error {
    DbError(diesel::result::Error),
    IoError(std::io::Error),
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Error {
        Error::DbError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::IoError(e)
    }
}

fn update_aux(conn: &mut SqliteConnection) -> Result<(), Error> {
    // collect all gcroots from the database
    let mut gcroots: HashSet<String> = HashSet::new();
    for project in projects.load::<Project>(conn)? {
        project
            .project_actions_path
            .map(|path| gcroots.insert(path.clone()));
    }
    for jobset in jobsets.load::<Jobset>(conn)? {
        let latest_evaluation = evaluations
            .filter(evaluation_jobset.eq(jobset.jobset_id))
            .order(evaluation_id.desc())
            .first::<Evaluation>(conn)
            .ok();
        if let Some(evaluation) = latest_evaluation {
            evaluation
                .evaluation_actions_path
                .map(|path| gcroots.insert(path.clone()));
            for job in jobs
                .filter(job_evaluation.eq(evaluation.evaluation_id))
                .load::<Job>(conn)?
            {
                let build = builds
                    .filter(build_id.eq(job.job_build))
                    .first::<Build>(conn)?;
                gcroots.insert(build.build_drv.clone());
                gcroots.insert(build.build_out.clone());
            }
        }
    }

    let gcroots_dir = Path::new("/nix/var/nix/gcroots/typhon");

    // write new gcroots on disk
    let new_path = gcroots_dir.join("new");
    if new_path.exists() {
        remove_dir_all(&new_path)?
    }
    DirBuilder::new().create(&new_path)?;
    for (i, gcroot) in gcroots.iter().enumerate() {
        symlink(Path::new(&gcroot), new_path.join(i.to_string()))?;
    }

    // replace old gcroots
    let cur_path = gcroots_dir.join("cur");
    if cur_path.exists() {
        remove_dir_all(&cur_path)?
    }
    rename(&new_path, &cur_path)?;

    Ok(())
}

pub fn update(conn: &mut SqliteConnection) -> () {
    update_aux(conn).unwrap_or_else(|e| log::error!("error when updating gcroots: {:?}", e))
}
