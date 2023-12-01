use crate::*;

use typhon_types::*;

use diesel::prelude::*;

pub fn search(
    limit: u8,
    offset: u32,
    req: &requests::search::Request,
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
                    total: query().count().get_result::<i64>(conn)? as u64,
                })
            }};
        }
    use {requests::search::Request, responses::search::Results};
    Ok(match req {
        Request::Projects => run!(
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
        Request::Jobsets(s) => run!(
            schema::jobsets::table
                .inner_join(schema::projects::table)
                .select((schema::projects::name, schema::jobsets::name)),
            filters(s): [
                s.project_name.map(|x| schema::projects::name.eq(x)),
            ],
            handles::jobset,
            Results::Jobsets
        ),
        Request::Evaluations(s) => run!(
            schema::evaluations::table
                .inner_join(schema::projects::table)
                .inner_join(
                    schema::tasks::table.on(schema::tasks::id.eq(schema::evaluations::task_id)),
                )
                .select((schema::projects::name, schema::evaluations::num))
                .order(schema::evaluations::time_created.desc()),
            filters(s): [
                s.project_name.map(|x| schema::projects::name.eq(x)),
                s.jobset_name.map(|x| schema::evaluations::jobset_name.eq(x)),
                s.status.map(|x| schema::tasks::status.eq(i32::from(x))),
            ],
            |(project, num): (_, i64)| handles::evaluation((project, num as u64)),
            Results::Evaluations
        ),
        Request::Builds(s) => run!(
            schema::builds::table
                .inner_join(schema::tasks::table)
                .select((schema::builds::drv, schema::builds::num))
                .order(schema::builds::time_created.desc()),
            filters(s): [
                s.drv.map(|x| schema::builds::drv.eq(x)),
                s.status.map(|x| schema::tasks::status.eq(i32::from(x))),
            ],
            |(drv, num): (_, i64)| handles::build((drv, num as u64)),
            Results::Builds
        ),
        Request::Actions(s) => run!(
            schema::actions::table
                .inner_join(schema::projects::table)
                .inner_join(schema::tasks::table)
                .select((schema::projects::name, schema::actions::num))
                .order(schema::actions::time_created.desc()),
            filters(s): [
                s.project_name.map(|x| schema::projects::name.eq(x)),
                s.status.map(|x| schema::tasks::status.eq(i32::from(x))),
            ],
            |(project, action): (_, i64)| handles::action((project, action as u64)),
            Results::Actions
        ),
        Request::Runs(s) => run!(
            schema::runs::table
                .inner_join(
                    schema::jobs::table
                        .inner_join(schema::evaluations::table.inner_join(schema::projects::table)),
                ).select(
                    (schema::projects::name, schema::evaluations::num, schema::jobs::system, schema::jobs::name, schema::runs::num)
                ).order(schema::runs::time_created.desc()),
            filters(s): [
                s.project_name.map(|x| schema::projects::name.eq(x)),
                s.jobset_name.map(|x| schema::evaluations::jobset_name.eq(x)),
                s.evaluation_num.map(|x| schema::evaluations::num.eq(x as i64)),
                s.job_name.map(|x| schema::jobs::name.eq(x)),
                s.job_system.map(|x| schema::jobs::system.eq(x)),
            ],
            |(project, eval, job_system, job_name, run): (_, i64, _, _, i64)| {
                handles::run((
                    project, eval as u64, job_system, job_name, run as u64
                ))
            },
            Results::Runs
        ),
    })
}
