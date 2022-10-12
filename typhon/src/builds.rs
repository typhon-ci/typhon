use crate::error::Error;
use crate::models::*;
use crate::nix;
use crate::schema::builds::dsl::*;
use crate::{connection, BUILDS};
use crate::{handles, responses};
use diesel::prelude::*;

impl Build {
    pub fn cancel(&self) -> Result<(), Error> {
        let r = BUILDS.get().unwrap().cancel(self.build_id);
        if r {
            Ok(())
        } else {
            Err(Error::BuildNotRunning(handles::Build {
                build_hash: self.build_hash.clone(),
            }))
        }
    }

    pub fn get(hash: &String) -> Result<Self, Error> {
        let conn = &mut *connection();
        Ok(builds
            .filter(build_hash.eq(hash))
            .first::<Build>(conn)
            .map_err(|_| {
                Error::BuildNotFound(handles::Build {
                    build_hash: hash.to_string(),
                })
            })?)
    }

    pub fn info(&self) -> Result<responses::BuildInfo, Error> {
        Ok(responses::BuildInfo {
            drv: self.build_drv.clone(),
            status: self.build_status.clone(),
        })
    }

    pub fn run(self) -> () {
        let id = self.build_id;
        let drv = self.build_drv.clone();
        let task = async move {
            nix::build(drv)?;
            Ok::<(), Error>(())
        };
        let f = move |r| {
            let status = match r {
                Some(Ok(())) => "success",
                Some(Err(_)) => "error", // TODO: log error
                None => "canceled",
            };
            let conn = &mut *connection();
            let _ = diesel::update(builds.find(id))
                .set(build_status.eq(status))
                .execute(conn);
        };
        BUILDS.get().unwrap().run(id, task, f);
    }
}
