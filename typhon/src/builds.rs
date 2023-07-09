use crate::connection;
use crate::error::Error;
use crate::models::*;
use crate::nix;
use crate::schema::builds::dsl::*;
use crate::BUILDS;
use crate::{handles, responses};
use crate::{log_event, Event};
use diesel::prelude::*;

impl From<Build> for responses::BuildInfo {
    fn from(build: Build) -> responses::BuildInfo {
        responses::BuildInfo {
            drv: build.build_drv.clone(),
            out: build.build_out.clone(),
            status: build.build_status.clone(),
        }
    }
}

impl Build {
    pub async fn cancel(&self) -> Result<(), Error> {
        let r = BUILDS.cancel(self.build_id).await;
        if r {
            Ok(())
        } else {
            Err(Error::BuildNotRunning(self.handle()))
        }
    }

    pub async fn get(build_handle: &handles::Build) -> Result<Self, Error> {
        let build_hash_ = &build_handle.build_hash;
        let mut conn = connection().await;
        Ok(builds
            .filter(build_hash.eq(build_hash_))
            .first::<Build>(&mut *conn)
            .map_err(|_| {
                Error::BuildNotFound(handles::Build {
                    build_hash: build_hash_.to_string(),
                })
            })?)
    }

    pub fn handle(&self) -> handles::Build {
        handles::Build {
            build_hash: self.build_hash.clone(),
        }
    }

    pub fn info(&self) -> Result<responses::BuildInfo, Error> {
        Ok(self.clone().into())
    }

    pub async fn nixlog(&self) -> Result<String, Error> {
        let log = nix::log(self.build_drv.clone()).await?;
        Ok(log)
    }

    pub async fn run(self) -> () {
        let handle = self.handle();
        let id = self.build_id;
        let drv = self.build_drv.clone();
        let task = async move {
            nix::build(&nix::DrvPath::new(&drv)).await?;
            Ok::<(), Error>(())
        };
        let f = move |r| async move {
            let status = match r {
                Some(Ok(())) => "success",
                Some(Err(_)) => "error", // TODO: log error
                None => "canceled",
            };
            let conn = &mut *connection().await;
            let _ = diesel::update(builds.find(id))
                .set(build_status.eq(status))
                .execute(conn);
            log_event(Event::BuildFinished(handle));
        };
        BUILDS.run(id, task, f).await;
    }
}
