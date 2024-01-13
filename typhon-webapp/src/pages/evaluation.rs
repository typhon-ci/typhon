use crate::prelude::*;
use typhon_types::data::TaskStatusKind;
use typhon_types::responses::TaskStatus;

#[derive(Debug, Clone, Copy)]
enum LogKind {
    Build(Uuid),
    Action(Uuid),
}

impl LogKind {
    #[cfg(feature = "ssr")]
    fn log_signal(self) -> ReadSignal<Option<String>> {
        create_signal(None).0
    }

    #[cfg(feature = "hydrate")]
    fn log_signal(self) -> ReadSignal<Option<String>> {
        let url = match self {
            Self::Build(uuid) => format!("/api/builds/{}/log", uuid),
            Self::Action(uuid) => format!("/api/actions/{}/log", uuid),
        };
        crate::streams::fetch_as_signal(gloo_net::http::Request::get(url.as_str()).build().unwrap())
    }
}

#[component]
fn LogTab(
    #[prop(into)] title: String,
    #[prop(into)] status: Signal<TaskStatus>,
    #[prop(into)] log_kind: LogKind,
) -> impl IntoView {
    let lines: ReadSignal<Option<String>> = log_kind.log_signal();
    let job_item_style = style! {
        details :deep(> summary > span) {
            display: inline-block;
        }
        details :deep(> summary) {
            padding: 4px;
            margin: 4px;
        }
        details :deep(> summary) {
            display: grid;
            grid-template-columns: auto auto 1fr auto;
        }
        details[open] :deep(> summary > .icon > *) {
            transform: rotate(90deg);
        }
        details :deep(> summary > .icon > *) {
            transition: transform 100ms;
        }
        details :deep(> summary > time) {
            font-family: JetBrains Mono;
        }
        details :deep(> summary > .status) {
            padding: 0 "0.5em";
        }
    };
    view! {
        <details class=job_item_style>
            <summary>
                <span class="icon">
                    <Icon icon=Icon::from(BiChevronRightRegular)/>
                </span>
                <span class="status">
                    <Status status=Signal::derive(move || status().into())/>
                </span>
                <span>{title}</span>
                <Duration duration=Signal::derive(move || match status() {
                    TaskStatus::Success(range)
                    | TaskStatus::Error(range)
                    | TaskStatus::Canceled(Some(range)) => Some(range.into()),
                    TaskStatus::Pending { start: Some(start) } => {
                        let now = use_context::<crate::utils::CurrentTime>().unwrap().0;
                        Some(now() - start)
                    }
                    _ => None,
                })/>
            </summary>

            {
                view! { <LiveLog lines/> }
            }

        </details>
    }
}

#[component]
pub fn JobSubpage(#[prop(into)] job: responses::JobInfo) -> impl IntoView {
    let style = style! {
        div.header, div.contents {
            padding: 16px;
        }
        div.header {
            border-bottom: 1px solid #32383F;
            display: grid;
            grid-template-columns: 1fr auto auto auto;
            align-items: center;
        }
        h1, h2 {
            padding: 0;
            margin: 0;
        }
        h1 {
            font-size: 110%;
            font-weight: 400;
        }
        h2 {
            font-size: 75%;
            font-weight: 300;
            padding-top: 2px;
            color: #8C959F;
        }
    };
    view! { class=style,
        <div class="header">
            <div class="name">
                <h1>
                    <span>{job.handle.name}</span>
                    <span>{format!(" ({})", job.handle.system)}</span>
                </h1>
                <h2>succeeded DDD days ago in DURATION</h2>
            </div>
            <Icon icon=Icon::from(BiRefreshRegular)/>
            <Icon icon=Icon::from(BiCogRegular)/>
        </div>
        <div class="contents">

            {
                let run = job.last_run;
                vec![
                    run
                        .begin
                        .map(|action| {
                            view! {
                                <LogTab
                                    log_kind=LogKind::Action(action.handle.uuid)
                                    status=move || action.status
                                    title="Begin job"
                                />
                            }
                        }),
                    run
                        .build
                        .map(|build| {
                            view! {
                                <LogTab
                                    log_kind=LogKind::Build(build.handle.uuid)
                                    status=move || build.status
                                    title="Build"
                                />
                            }
                        }),
                    run
                        .end
                        .map(|action| {
                            view! {
                                <LogTab
                                    log_kind=LogKind::Action(action.handle.uuid)
                                    status=move || action.status
                                    title="Begin job"
                                />
                            }
                        }),
                ]
            }

        </div>
    }
}

#[component]
fn Main(
    #[allow(unused)]
    #[prop(into)]
    info: responses::EvaluationInfo,
    #[prop(into)] tab: Signal<crate::routes::EvaluationTab>,
) -> impl IntoView {
    let active_tab = tab;
    use crate::routes::EvaluationTab as ActiveItem;
    let item_style = style! {
        .active {
            font-weight: 400;
        }
        li {
            margin: 0;
            list-style-type: none;
            padding: "0.1em";
        }
        li :deep(> a > span.label) {
            text-overflow: ellipsis;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }
        li:hover :deep(> a) {
            background: #F4F5F7;
            border-radius: 5px;
        }
        li.active :deep(> a) {
            background: #F4F5F7;
            border-radius: 5px;
        }
        li :deep(> a) {
            text-decoration: none;
            color: inherit;
            display: flex;
            align-items: center;
            padding: "0.5em";
        }
        .icon {
            margin-right: "0.4em";
            display: flex;
            align-items: center;
            color: gray;
        }
    };
    let mk_item = |tab: ActiveItem, icon, contents: View| {
        let handle = info.handle.clone();
        view! { class=item_style,
            <li class:active={
                let tab = tab.clone();
                move || active_tab() == tab.clone()
            }>

                <A href=Box::new(move || crate::routes::to_url(crate::routes::EvaluationPage {
                    handle: handle.clone(),
                    tab: tab.clone(),
                }))>
                    <span class="icon">{icon}</span>
                    <span class="label">{contents}</span>
                </A>
            </li>
        }
    };
    let items = info
        .jobs
        .clone()
        .into_iter()
        .map(|(_, info)| {
            let last_run = info.last_run.clone();
            mk_item(
                ActiveItem::Job(info.handle.clone()),
                // FIXME: why do I need to clone twice?
                view! { <Status status=move || TaskStatus::from(last_run.clone()).into()/> },
                view! {
                    <span>
                        {info.handle.name}
                        <span style="color: gray; font-size: 90%;">
                            {format!(" ({})", info.handle.system)}
                        </span>
                    </span>
                }
                .into_view(),
            )
        })
        .collect::<Vec<_>>();
    let style = style! {
        nav {
            padding: 16px;
        }
        nav :deep(section > ul) {
            padding: 0;
            margin: 0;
        }
        nav :deep(section > h1) {
            color: rgb(101, 109, 118);
            font-weight: 500;
            font-size: 80%;
            border-top: 1px solid rgba(208, 215, 222, 0.48);
            padding-top: 16px;
            margin-top: 8px;
        }
    };
    let main = view! {
        <nav class=style>
            <section>
                <ul style="padding: 0;">
                    {mk_item(
                        ActiveItem::Summary,
                        view! { <Icon icon=Icon::from(BiHomeAltRegular)/> },
                        view! { Summary }.into_view(),
                    )}

                </ul>
            </section>
            <section>
                <h1>Jobs</h1>
                <ul style="padding: 0;">{items}</ul>
            </section>
            <section>
                <h1>Details</h1>
                <ul style="padding: 0;">
                    {mk_item(
                        ActiveItem::Usage,
                        view! { <Icon icon=Icon::from(BiTimerRegular)/> },
                        view! { Usage }.into_view(),
                    )}

                </ul>
            </section>
        </nav>
        // <div>
        <div class="contents">

            {
                let jobs = info.jobs.clone();
                move || {
                    match active_tab() {
                        ActiveItem::Summary => "Summary page, todo".into_view(),
                        ActiveItem::Job(job) => {
                            let job = jobs
                                .clone()
                                .into_iter()
                                .find(|(_, info)| info.handle == job)
                                .unwrap()
                                .1;
                            view! {
                                // FIXME: why do we need to clone twice?
                                <JobSubpage job/>
                            }
                        }
                        ActiveItem::Usage => "Usage page, todo".into_view(),
                    }
                }
            }

        </div>
    };

    let global_status: Signal<TaskStatus> = Signal::derive(move || {
        info.jobs
            .iter()
            .map(|(_, info)| TaskStatus::from(info.last_run.clone()))
            .reduce(|a, b| a.union(&b))
            .unwrap()
    });
    let global_status_kind: Memo<TaskStatusKind> = create_memo(move |_| global_status().into());
    let style = style! {
        div {
            display: grid;
            grid-template-areas: raw_str("header header") raw_str("nav contents");
            grid-template-columns: 250px 1fr;
            margin-right: 16px;
        }
        div :deep(> header) {
            grid-area: header;
            padding: 16px;
        }
        div :deep(> nav) {
            grid-area: nav;
        }
        div :deep(> .contents) {
            grid-area: contents;
            background: rgb(36, 41, 47);
            border-radius: 3px;
            color: rgb(246, 248, 250);
        }
        div :deep(.summary .tag) {
            margin-left: "1em";
        }
    };
    let header_style = style! {
        header {
            display: grid;
            grid-template-areas: raw_str("s b1 b2");
            grid-template-columns: 1fr auto auto;
        }
        header :deep(> .summary > .status) {
            display: inline-block;
            padding-right: 7px;
        }
        header :deep(> .summary) {
            grid-area: s;
            font-size: var(--font-size-huge);
            display: inline-flex;
            align-items: center;
        }
        header :deep(> button) {
            padding: "0.4em";
            margin: "0.4em";
        }
        header :deep(> .rerun-jobs) {
            grid-area: b1;
        }
        header :deep(> .more) {
            grid-area: b2;
        }
    };
    view! {
        <div class=style>
            <header class=header_style>
                <div class="summary">
                    <span class="status">
                        <Status status=Signal::derive(move || global_status_kind())/>
                    </span>
                    <span>Evaluation <code>{info.handle.uuid.to_string()}</code></span>
                    {crate::utils::FlakeUri::parse(info.url)
                        .map(|flake| {
                            view! {
                                <Tag href=flake.web_url>
                                    <code>{flake.r#ref}</code>
                                </Tag>
                            }
                        })}

                </div>
                <button class="rerun-jobs">Re-run all jobs</button>
                <button class="more">
                    <Icon icon=Icon::from(BiDotsHorizontalRoundedRegular)/>
                </button>
            </header>
            {main}
        </div>
    }
}

#[component]
pub fn Evaluation(
    #[prop(into)] handle: Signal<handles::Evaluation>,
    #[prop(into)] tab: Signal<crate::routes::EvaluationTab>,
) -> impl IntoView {
    let (error, info) = resource!(
        Signal::derive(move || Request::Evaluation(handle(), requests::Evaluation::Info)),
        |Response::EvaluationInfo(info)| info
    );
    view! { <Trans error>{move || info().map(|info| view! { <Main info tab/> })}</Trans> }
}
