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

allow_columns_to_appear_in_same_group_by_clause!(
    schema::jobs::build_out,
    schema::evaluations::jobset_id
);

fn update_aux(conn: &mut diesel::SqliteConnection) -> Result<(), Error> {
    // collect all gcroots from the database
    let mut gcroots: HashSet<String> = HashSet::new();
    let mut res = schema::evaluations::table
        .inner_join(schema::jobs::table)
        .group_by((schema::jobs::build_out, schema::evaluations::jobset_id))
        .select((
            schema::jobs::build_out,
            schema::evaluations::jobset_id,
            diesel::dsl::max(schema::evaluations::num),
        ))
        .load::<(String, i32, Option<i64>)>(conn)?;
    for (path, _, _) in res.drain(..) {
        gcroots.insert(path);
    }
    // TODO: insert build time dependencies
    let mut res = schema::projects::table
        .select(schema::projects::actions_path)
        .load::<Option<String>>(conn)?;
    for actions in res.drain(..) {
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

pub fn update(conn: &mut diesel::SqliteConnection) -> () {
    update_aux(conn).unwrap_or_else(|e| log::error!("error when updating gcroots: {:?}", e));
}
