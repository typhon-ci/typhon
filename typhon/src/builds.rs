use crate::connection;
use crate::error::Error;
use crate::models::*;
use crate::nix;
use crate::schema::builds::dsl::*;
use crate::BUILDS;
use crate::{handles, responses};
use crate::{log_event, Event};
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

    pub fn get(conn: &mut SqliteConnection, build_handle: &handles::Build) -> Result<Self, Error> {
        let build_hash_ = &build_handle.build_hash;
        Ok(builds
            .filter(build_hash.eq(build_hash_))
            .first::<Build>(conn)
            .map_err(|_| {
                Error::BuildNotFound(handles::Build {
                    build_hash: build_hash_.to_string(),
                })
            })?)
    }

    pub fn handle(&self) -> Result<handles::Build, Error> {
        Ok(handles::Build {
            build_hash: self.build_hash.clone(),
        })
    }

    pub fn info(&self) -> Result<responses::BuildInfo, Error> {
        Ok(responses::BuildInfo {
            drv: self.build_drv.clone(),
            status: self.build_status.clone(),
        })
    }

    pub fn run(self) -> () {
        let handle = self.handle().unwrap(); // TODO
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
            let conn: &mut SqliteConnection = &mut *connection();
            let _ = diesel::update(builds.find(id))
                .set(build_status.eq(status))
                .execute(conn);
            log_event(Event::BuildFinished(handle));
        };
        BUILDS.get().unwrap().run(id, task, f);
    }
}
