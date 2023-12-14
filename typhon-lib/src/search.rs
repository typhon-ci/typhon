use crate::*;

use typhon_types::*;

use diesel::prelude::*;
use uuid::Uuid;

use std::str::FromStr;

pub fn search(
    limit: u8,
    offset: u32,
    kind: &requests::search::Kind,
    conn: &mut Conn,
) -> Result<responses::Response, Error> {
    macro_rules! run {
            ($query:expr, filters$(($ctx:ident))?: [$($filter: expr),*$(,)?], $reshape: expr, $into_results: expr) => {{
                let query = || {
                    #[allow(unused_mut)]
                    let mut query = $query.into_boxed();
                    $(let $ctx = $ctx.clone();)?
                    $(if let Some(f) = $filter {query = query.filter(f);})*
                    query
                };
                let page = query().limit(limit.into()).offset(offset.into());
                let data = page.load(conn)?.into_iter().map($reshape).collect();
                responses::Response::Search(responses::search::Info {
                    results: $into_results(data),
                    total: query().count().get_result::<i64>(conn)? as u32,
                })
            }};
        }
    use {requests::search::Kind, responses::search::Results};
    Ok(match kind {
        Kind::Projects => run!(
            schema::projects::table.select({
                use schema::projects::*;
                (name, description, homepage, title)
            }),
            filters: [],
            |(name, description, homepage, title)|
            (
                handles::project(name),
                responses::ProjectMetadata {description, homepage, title}
            ),
            Results::Projects
        ),
        Kind::Jobsets(s) => run!(
            schema::jobsets::table
                .inner_join(schema::projects::table)
                .select((schema::projects::name, schema::jobsets::name)),
            filters(s): [
                s.project_name.map(|x| schema::projects::name.eq(x)),
            ],
            handles::jobset,
            Results::Jobsets
        ),
        Kind::Evaluations(s) => run!(
            schema::evaluations::table
                .inner_join(schema::projects::table)
                .inner_join(
                    schema::tasks::table.on(schema::tasks::id.eq(schema::evaluations::task_id)),
                )
                .select(schema::evaluations::uuid)
                .order(schema::evaluations::time_created.desc()),
            filters(s): [
                s.project_name.map(|x| schema::projects::name.eq(x)),
                s.jobset_name.map(|x| schema::evaluations::jobset_name.eq(x)),
                s.status.map(|x| schema::tasks::status.eq(i32::from(x))),
            ],
            |uuid: String| handles::evaluation(Uuid::from_str(&uuid).unwrap()),
            Results::Evaluations
        ),
        Kind::Builds(s) => run!(
            schema::builds::table
                .inner_join(schema::tasks::table)
                .select(schema::builds::uuid)
                .order(schema::builds::time_created.desc()),
            filters(s): [
                s.drv.map(|x| schema::builds::drv.eq(x)),
                s.status.map(|x| schema::tasks::status.eq(i32::from(x))),
            ],
            |uuid: String| handles::build(Uuid::from_str(&uuid).unwrap()),
            Results::Builds
        ),
        Kind::Actions(s) => run!(
            schema::actions::table
                .inner_join(schema::projects::table)
                .inner_join(schema::tasks::table)
                .select(schema::actions::uuid)
                .order(schema::actions::time_created.desc()),
            filters(s): [
                s.project_name.map(|x| schema::projects::name.eq(x)),
                s.status.map(|x| schema::tasks::status.eq(i32::from(x))),
            ],
            |uuid: String| handles::action(Uuid::from_str(&uuid).unwrap()),
            Results::Actions
        ),
        Kind::Runs(s) => run!(
            schema::runs::table
                .inner_join(
                    schema::jobs::table
                        .inner_join(schema::evaluations::table.inner_join(schema::projects::table)),
                ).select(
                    (schema::evaluations::uuid, schema::jobs::system, schema::jobs::name, schema::runs::num)
                ).order(schema::runs::time_created.desc()),
            filters(s): [
                s.project_name.map(|x| schema::projects::name.eq(x)),
                s.jobset_name.map(|x| schema::evaluations::jobset_name.eq(x)),
                s.evaluation_uuid.map(|x| schema::evaluations::uuid.eq(x.to_string())),
                s.job_name.map(|x| schema::jobs::name.eq(x)),
                s.job_system.map(|x| schema::jobs::system.eq(x)),
            ],
            |(eval, job_system, job_name, run): (String, _, _, i32)| {
                handles::run((
                    Uuid::from_str(&eval).unwrap(), job_system, job_name, run as u32
                ))
            },
            Results::Runs
        ),
    })
}
