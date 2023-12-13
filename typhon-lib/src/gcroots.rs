use crate::nix;
use crate::schema;
use crate::Conn;

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
    schema::jobs::out,
    schema::jobs::drv,
    schema::jobsets::name,
);

fn update_aux(conn: &mut Conn) -> Result<(), Error> {
    // collect all gcroots from the database
    let mut gcroots: HashSet<String> = HashSet::new();
    let mut res_1 = schema::evaluations::table
        .inner_join(schema::jobs::table)
        .inner_join(
            schema::jobsets::table.on(schema::evaluations::jobset_name.eq(schema::jobsets::name)),
        )
        .group_by((schema::jobs::out, schema::jobs::drv, schema::jobsets::name))
        .select((
            schema::jobs::out,
            schema::jobs::drv,
            schema::jobsets::name,
            diesel::dsl::max(schema::evaluations::time_created),
        ))
        .load::<(String, String, String, Option<i64>)>(conn)?;
    let mut res_2 = schema::projects::table
        .select(schema::projects::actions_path)
        .load::<Option<String>>(conn)?;

    for (path, drv, _, _) in res_1.drain(..) {
        if let Ok(deps) = nix::dependencies(&drv) {
            for dep in deps {
                gcroots.insert(dep);
            }
        } else {
            log::warn!("gcroots: missing derivation {}", drv);
        }
        gcroots.insert(drv);
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

pub fn update(conn: &mut Conn) -> () {
    update_aux(conn).unwrap_or_else(|e| log::error!("error when updating gcroots: {:?}", e));
}
