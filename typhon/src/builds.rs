use crate::error::Error;
use crate::models;
use crate::nix;
use crate::schema;
use crate::tasks;
use crate::Conn;

use typhon_types::*;

use diesel::prelude::*;

#[derive(Clone)]
pub struct Build {
    pub task: tasks::Task,
    pub build: models::Build,
}

impl Build {
    pub fn get(conn: &mut Conn, handle: &handles::Build) -> Result<Self, Error> {
        let (build, task) = schema::builds::table
            .inner_join(schema::tasks::table)
            .filter(schema::builds::drv.eq(&handle.drv))
            .filter(schema::builds::num.eq(handle.num as i64))
            .first(conn)
            .optional()?
            .ok_or(Error::BuildNotFound(handle.clone()))?;
        Ok(Self {
            task: tasks::Task { task },
            build,
        })
    }

    pub fn handle(&self) -> handles::Build {
        handles::build((self.build.drv.clone(), self.build.num as u64))
    }

    pub fn info(&self) -> responses::BuildInfo {
        responses::BuildInfo {
            drv: self.build.drv.clone(),
            status: self.task.status(),
        }
    }

    pub fn last(conn: &mut Conn, drv: &nix::DrvPath) -> Result<Option<Self>, Error> {
        Ok(schema::builds::table
            .inner_join(schema::tasks::table)
            .filter(schema::builds::drv.eq(drv.to_string()))
            .order(schema::builds::time_created.desc())
            .first(conn)
            .optional()?
            .map(|(build, task)| Self {
                task: tasks::Task { task },
                build,
            }))
    }

    pub fn log(&self, conn: &mut Conn) -> Result<Option<String>, Error> {
        self.task.log(conn)
    }

    pub fn search(
        conn: &mut Conn,
        search: &requests::BuildSearch,
    ) -> Result<responses::SearchResult<handles::Build>, Error> {
        let query = || {
            let mut query = schema::builds::table
                .inner_join(schema::tasks::table)
                .into_boxed();
            if let Some(drv) = &search.drv {
                query = query.filter(schema::builds::drv.eq(drv));
            }
            if let Some(status) = search.status {
                query = query.filter(schema::tasks::status.eq(status.to_i32()));
            }
            query.order(schema::builds::time_created.desc())
        };
        let (builds, total): (Vec<_>, i64) = conn.transaction::<_, Error, _>(|conn| {
            let total = query().count().get_result(conn)?;
            let builds = query()
                .offset(search.offset as i64)
                .limit(search.limit as i64)
                .load::<(models::Build, models::Task)>(conn)?;
            Ok((builds, total))
        })?;
        let count = builds.len() as u8;
        let total = total as u64;
        let list = builds
            .into_iter()
            .map(|(build, _)| handles::build((build.drv, build.num as u64)))
            .collect();
        Ok(responses::SearchResult { count, list, total })
    }
}
