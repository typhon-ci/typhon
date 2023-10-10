use crate::connection;
use crate::nix;
use crate::schema;

use diesel::prelude::*;

use std::collections::HashSet;
use std::fs::{remove_dir_all, rename, DirBuilder};
use std::os::unix::fs::symlink;
use std::path::Path;

#[derive(Debug)]
enum Error {
    DbError(diesel::result::Error),
    IoError(std::io::Error),
    NixError(nix::Error),
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

impl From<nix::Error> for Error {
    fn from(e: nix::Error) -> Error {
        Error::NixError(e)
    }
}

allow_columns_to_appear_in_same_group_by_clause!(
    schema::jobs::build_out,
    schema::evaluations::jobset_id
);

async fn update_aux() -> Result<(), Error> {
    // collect all gcroots from the database
    let mut conn = connection().await;
    let mut gcroots: HashSet<String> = HashSet::new();
    let mut res_1 = schema::evaluations::table
        .inner_join(schema::jobs::table)
        .group_by((schema::jobs::build_out, schema::evaluations::jobset_id))
        .select((
            schema::jobs::build_out,
            schema::evaluations::jobset_id,
            diesel::dsl::max(schema::evaluations::num),
        ))
        .load::<(String, i32, Option<i64>)>(&mut *conn)?;
    let mut res_2 = schema::projects::table
        .select(schema::projects::actions_path)
        .load::<Option<String>>(&mut *conn)?;
    drop(conn);
    // TODO: insert build time dependencies
    for (path, _, _) in res_1.drain(..) {
        gcroots.insert(path);
    }
    for actions in res_2.drain(..) {
        if let Some(path) = actions {
            gcroots.insert(path);
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

pub async fn update() -> () {
    update_aux()
        .await
        .unwrap_or_else(|e| log::error!("error when updating gcroots: {:?}", e));
}
