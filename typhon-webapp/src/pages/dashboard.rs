use crate::prelude::*;
use crate::routes::DashboardTab;

use data::TaskStatusKind;

#[component]
pub fn PendingBuilds(page: u32) -> impl IntoView {
    view! {
        <h1>"Pending builds:"</h1>
        "TODO"
    }
}

#[component]
pub fn PendingEvaluations(page: u32) -> impl IntoView {
    let limit = Signal::derive(move || 10 as u8);
    let offset = Signal::derive(move || (page - 1) * (limit() as u32));
    let (error, evaluations) = search!(
        offset,
        limit,
        Signal::derive({
            move || {
                requests::search::Kind::Evaluations(requests::search::Evaluation {
                    jobset_name: None,
                    project_name: None,
                    status: Some(TaskStatusKind::Pending),
                })
            }
        }),
        |total, responses::search::Results::Evaluations(evals)| (total, evals)
    );
    let evaluations = Signal::derive(move || evaluations().unwrap_or((0, Vec::new())));
    let count = Signal::derive(move || evaluations().0);
    let evaluations = Signal::derive(move || evaluations().1);
    view! {
        <h1>"Pending evaluations:"</h1>
        <Trans error>
            <Evaluations count evaluations/>
        </Trans>
    }
}

#[component]
pub fn PendingActions(page: u32) -> impl IntoView {
    view! {
        <h1>"Pending actions:"</h1>
        "TODO"
    }
}

#[component]
pub fn Dashboard(tab: DashboardTab, page: u32) -> impl IntoView {
    view! {
        <A href=String::from(Root::Dashboard {
            tab: DashboardTab::Builds,
            page: 1,
        })>Builds</A>
        <A href=String::from(Root::Dashboard {
            tab: DashboardTab::Evaluations,
            page: 1,
        })>Evaluations</A>
        <A href=String::from(Root::Dashboard {
            tab: DashboardTab::Actions,
            page: 1,
        })>Actions</A>
        {match tab {
            DashboardTab::Builds => view! { <PendingBuilds page/> }.into_view(),
            DashboardTab::Evaluations => view! { <PendingEvaluations page/> }.into_view(),
            DashboardTab::Actions => view! { <PendingActions page/> }.into_view(),
        }}
    }
}
