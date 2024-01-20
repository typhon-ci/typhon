use crate::prelude::*;

use data::TaskStatusKind;
use responses::TaskStatus;

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Describe the summary of the statuses for an evaluation
pub struct EvalStatus {
    /// Status of the Nix evaluation
    pub eval: TaskStatus,
    /// Aggregate status for all the jobs
    pub jobs: Option<TaskStatus>,
    /// Map from `Succeeded`/`Pending`/... to the count of jobs in the
    /// corresponding status
    pub map: HashMap<TaskStatusKind, u32>,
}

#[component]
pub fn StatusMap(map: HashMap<TaskStatusKind, u32>, compact: bool) -> impl IntoView {
    let style = style! {
        .statuses {
            display: inline-flex;
        }
        .statuses.compact {
            --status-font-size: var(--font-size-normal);
        }
        .statuses:not(.compact) {
            gap: 12px;
        }
        .statuses:not(.compact) :deep(> .status) {
            gap: 4px;
        }
        .statuses :deep(> .status) {
            display: flex;
            align-items: center;
        }
        .statuses.compact :deep(> .status) {
            flex-direction: column;
            width: 20px;
        }
        .statuses:not(.compact) :deep(> .status[data-n="0"] svg) {
            opacity: 0.6;
        }
        .statuses.compact :deep(> .status[data-n="0"] svg) {
            opacity: 0.3;
        }
        .statuses.compact :deep(> .status[data-n="0"]) {
            opacity: 0.2;
        }
        .statuses :deep(> .status[data-n="0"]) {
            filter: saturate(0%);
        }
        .statuses.compact :deep(> .status > .count) {
            display: block;
        }
        .statuses :deep(> .status > .count) {
            padding-bottom: 2px;
            color: color-mix(in lch, var(--color-task-status) 70%, black);
        }
    };
    (!map.is_empty()).then(|| {
        view! { class=style,
            <div class="statuses" class:compact=compact>

                {
                    use strum::IntoEnumIterator;
                    TaskStatusKind::iter()
                        .map(|k| {
                            let k = k.clone();
                            let n = map.get(&k).copied().unwrap_or(0);
                            let n = format!("{}", n);
                            view! {
                                <div class="status" data-n=n.clone()>
                                    <span class="count" data-status=format!("{:?}", &k)>
                                        {n}
                                    </span>
                                    <Status status=move || k/>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                }

            </div>
        }
    })
}

impl EvalStatus {
    pub fn hybrid_status(&self) -> HybridStatusKind {
        use crate::components::status::HybridStatusKind;
        match TaskStatusKind::from(&self.eval) {
            TaskStatusKind::Pending => HybridStatusKind::EvalPending,
            TaskStatusKind::Success => HybridStatusKind::EvalSucceeded {
                build: self.jobs.unwrap_or_default().into(),
            },
            TaskStatusKind::Error | TaskStatusKind::Canceled => HybridStatusKind::EvalStopped,
        }
    }
    pub fn summary(&self) -> TaskStatus {
        if let Some(jobs) = self.jobs {
            jobs
        } else {
            self.eval
        }
    }
    pub fn new(info: &responses::EvaluationInfo) -> Self {
        let job_statuses: Vec<_> = info
            .jobs
            .clone()
            .into_values()
            .map(TaskStatus::from)
            .collect();
        let mut map: HashMap<TaskStatusKind, u32> = HashMap::new();
        for kind in &job_statuses {
            *map.entry(kind.into()).or_insert(0) += 1
        }
        let jobs = job_statuses
            .into_iter()
            .reduce(|x, y| TaskStatus::union(&x, &y));
        let eval = info.status.clone();
        Self { jobs, eval, map }
    }
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
    let style = style! {
        .row {
            gap: 8px;
        }
        .titles {
            flex: 1;
        }
        .informations :deep(> *) {
            display: flex;
            align-items: center;
            font-size: var(--font-size-small);
            color: var(--color-fg-muted);
            padding-bottom: 2px;
            padding-top: 2px;
        }
        .informations :deep(svg) {
            margin-right: 4px;
            font-size: var(--font-size-normal);
        }
    };
    view! {
        <Trans error>
            {move || {
                info()
                    .map(|info| {
                        let status_infos = EvalStatus::new(&info);
                        let created = info.time_created;
                        let duration = Signal::derive({
                            let status = status_infos.summary().clone();
                            move || {
                                let (start, end) = status.times();
                                let end = end
                                    .unwrap_or_else(|| use_context::<crate::utils::CurrentTime>()
                                        .unwrap()
                                        .0());
                                start.map(|start| end - start)
                            }
                        });
                        let href = Root::Evaluation(routes::EvaluationPage {
                            handle: info.handle.clone(),
                            tab: routes::EvaluationTab::Info,
                        });
                        view! { class=style,
                            <div class="row">
                                <div class="status">

                                    {
                                        let status_kind: HybridStatusKind = status_infos
                                            .hybrid_status();
                                        view! {
                                            <HybridStatus status=Signal::derive(move || {
                                                status_kind.clone()
                                            })/>
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
                                    <StatusMap map=status_infos.map compact=true/>
                                </div>
                                <div class="informations">
                                    <RelativeTime datetime=created/>
                                    <div>
                                        <Icon icon=Icon::from(BiTimerRegular)/>
                                        <Duration duration/>
                                    </div>
                                </div>
                            </div>
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
                        view! { <Evaluation handle/> }
                    }
                />

            </div>
        </div>
    }
}
