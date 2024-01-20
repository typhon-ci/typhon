use crate::prelude::*;

use data::TaskStatusKind;
use responses::TaskStatus;

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct EvalStatus {
    status: TaskStatus,
    map: HashMap<TaskStatusKind, u8>,
}

fn status_of_evaluation(info: &responses::EvaluationInfo) -> EvalStatus {
    let statuses: Vec<_> = info
        .jobs
        .clone()
        .into_values()
        .map(TaskStatus::from)
        .collect();
    let mut map: HashMap<TaskStatusKind, u8> = HashMap::new();
    for kind in &statuses {
        *map.entry(kind.into()).or_insert(0) += 1
    }
    let status = statuses
        .into_iter()
        .reduce(|x, y| TaskStatus::union(&x, &y))
        .unwrap_or(info.status);
    EvalStatus { status, map }
}

#[component]
pub fn FlakeURI(#[prop(into)] uri: String) -> impl IntoView {
    let style = style! {
        div.wrapper {
            display: inline-flex;
            align-items: center;
        }
        .commit {
            display: inline-block;
        }
        .commit {
            font-size: var(--font-size-big);
        }
        code {
            font-family: var(--font-family-monospace), monospace;
        }
        div :deep(.tag svg) {
            margin-right: 2px;
        }
    };
    match &uri.clone().split(":").collect::<Vec<_>>()[..] {
        ["github", rest]
            if let [owner, repo, commit] = &rest.split("/").collect::<Vec<_>>()[..] =>
        {
            let text = format!("{}/{}", owner, repo);
            view! { class=style,
                <div class="wrapper">
                    <span class="commit">Commit <code>{commit[..8].to_string()}</code></span>
                    <Tag href=format!("https://github.com/{owner}/{repo}")>
                        <Icon icon=Icon::from(BiGithub)/>
                        {text}
                    </Tag>
                </div>
            }
            .into_view()
        }
        _ => uri.into_view(),
    }
}

#[component]
pub fn Evaluation(handle: handles::Evaluation) -> impl IntoView {
    let (error, info) = resource!(
        Signal::derive(move || requests::Request::Evaluation(
            handle.clone(),
            requests::Evaluation::Info,
        )),
        |responses::Response::EvaluationInfo(info)| info
    );
    view! {
        <Trans error>
            {move || {
                info()
                    .map(|info| {
                        let status_infos = status_of_evaluation(&info);
                        let created = info.time_created;
                        let duration = Signal::derive({
                            let status = status_infos.status.clone();
                            move || {
                                let (start, end) = status.times();
                                start.zip(end).map(|(start, end)| end - start)
                            }
                        });
                        let href = Root::Evaluation(routes::EvaluationPage {
                            handle: info.handle.clone(),
                            tab: routes::EvaluationTab::Info,
                        });
                        view! {
                            <div class="status">

                                {
                                    let status_kind: TaskStatusKind = status_infos
                                        .status
                                        .clone()
                                        .into();
                                    view! {
                                        <Status status=Signal::derive(move || status_kind.clone())/>
                                    }
                                }

                            </div>
                            <div class="titles">
                                <A href class="first">
                                    <FlakeURI uri=info.url/>
                                </A>
                                <div class="second">
                                    Evaluation <UuidLabel uuid=info.handle.uuid/>
                                </div>
                            </div>
                            <div class="jobs-summary">
                                {move || format!("{:#?}", status_infos.map)}
                            </div>
                            <div class="informations">
                                <RelativeTime datetime=created/>
                                <div>
                                    <Icon icon=Icon::from(BiTimerRegular)/>
                                    <Duration duration/>
                                </div>
                            </div>
                            <button>
                                <Icon icon=Icon::from(BiTrashRegular)/>
                            </button>
                        }
                    })
            }}

        </Trans>
    }
}

#[component]
pub fn Evaluations(
    count: Signal<u32>,
    evaluations: Signal<Vec<handles::Evaluation>>,
) -> impl IntoView {
    let style = style! {
        .jobset-contents {
            --radius: 8px;
        }
        .header, .rows :deep(> .row) {
            margin: 0px 20px;
            display: flex;
            align-items: center;
            border: 1px solid var(--color-border-default);
            padding: 12px;
        }
        .rows :deep(> .row) {
            border-top: 0px;
        }
        .header {
            border-radius: var(--radius) var(--radius) 0 0;
            background: var(--color-bg-light);
        }
        .header :deep(.title) {
            flex: 1;
        }
        .header :deep(.buttons) {
            display: none;
        }
        .rows :deep(> .row:last-child) {
            border-radius: 0 0 var(--radius) var(--radius);
        }
        .rows :deep(> .row > .titles) {
            flex: 1;
        }
        .rows :deep(> .row > div) {
            padding-right: 8px;
        }
        .rows :deep(> .row > div:last-child) {
            padding-right: 0px;
        }
        .rows :deep(> .row > .informations > *) {
            display: flex;
            align-items: center;
            font-size: var(--font-size-small);
            color: var(--color-fg-muted);
            padding-bottom: 2px;
            padding-top: 2px;
        }
        .rows :deep(> .row > .informations svg) {
            margin-right: 4px;
            font-size: var(--font-size-normal);
        }
    };
    view! { class=style,
        <div class="jobset-contents">
            <div class="header">
                <div class="title">{count} evaluations</div>
                <div class="buttons">
                    <button>Event <Icon icon=Icon::from(BiChevronDownRegular)/></button>
                    <button>Status <Icon icon=Icon::from(BiChevronDownRegular)/></button>
                    <button>Branch <Icon icon=Icon::from(BiChevronDownRegular)/></button>
                    <button>Actor <Icon icon=Icon::from(BiChevronDownRegular)/></button>
                </div>
            </div>
            <div class="rows">
                <For
                    each=evaluations
                    key=|handle| handle.uuid.clone()
                    children=move |handle| {
                        view! {
                            <div class="row">
                                <Evaluation handle/>
                            </div>
                        }
                    }
                />

            </div>
        </div>
    }
}
