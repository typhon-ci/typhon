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

pub fn run_info(run: handles::Run, conn: &mut Conn) -> Result<responses::RunInfo, Error> {
    todo!()
}
