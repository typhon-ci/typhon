use crate::prelude::*;
use routes::{EvaluationTab, LogTab};
use std::collections::HashMap;
use typhon_types::data::TaskStatusKind;
use typhon_types::responses::TaskStatus;

fn fetch_log(log: handles::Log) -> ReadSignal<Option<String>> {
    #[cfg(feature = "ssr")]
    {
        let _ = log;
        create_signal(None).0
    }
    #[cfg(feature = "hydrate")]
    {
        use gloo_net::http::Request;
        crate::streams::fetch_as_signal(Request::post("/api/log").json(&log).unwrap())
    }
}

#[component]
fn LogTabHeader(
    #[prop(into)] title: String,
    #[prop(into)] status: Signal<TaskStatus>,
    href: Root,
    active: bool,
) -> impl IntoView {
    let style = style! {
        .tab-header {
            display: inline-grid;
            grid-template-columns: auto auto 1fr auto;
            color: inherit;
            text-decoration: inherit;
            font-size: 100%;
            padding: 0 5px;
            position: relative;
            z-index: 1;
            --status-font-size: var(--font-size-normal);
        }
        .tab-header.active {
            border-bottom: 1px solid white;
        }
        .tab-header :deep(> span) {
            display: inline-block;
        }
        .tab-header :deep(time) {
            color: var(--color-gray);
            font-size: 100%;
            letter-spacing: -0.3px;
            padding-left: 4px;
            font-family: JetBrains Mono, monospace;
        }
    };
    view! { class=style,
        <A
            class=format!("tab-header {style} {}", if active { " active" } else { "" })
            href=String::from(href)
        >
            <span class="status">
                <Status status=Signal::derive(move || status().into())/>
            </span>
            <span class="title">{title}</span>
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
        </A>
    }
}

#[component]
pub fn JobSubpage(
    #[prop(into)] job: responses::JobInfo,
    #[prop(into)] log_tab: LogTab,
) -> impl IntoView {
    let style = style! {
        div.header, div.contents {
            padding: 16px;
        }
        div.contents {
            padding-top: 0;
        }
        div.header {
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
        .tabs {
            position: relative;
        }
        .tabs :deep(> *) {
            margin-left: 10px;
        }
        .tabs::before {
            position: absolute;
            content: "''";
            bottom: 0;
            background: #32383F;
            height: 1px;
            width: 100%;
        }
        .active {
            padding-top: 10px;
        }
    };
    let href = {
        let eval_handle = job.handle.evaluation.clone();
        let job_handle = job.handle.clone();
        move |log_tab: LogTab| -> routes::Root {
            routes::Root::Evaluation(routes::EvaluationPage {
                handle: eval_handle.clone(),
                tab: EvaluationTab::Job {
                    handle: job_handle.clone(),
                    log_tab,
                },
            })
        }
    };

    let logs: Vec<_> = {
        let run = job.last_run;
        use handles::Log::*;
        vec![
            run.begin
                .map(|x| (Action(x.handle), x.status, "Begin", LogTab::Begin)),
            run.build
                .map(|x| (Build(x.handle), x.status, "Build", LogTab::Build)),
            run.end
                .map(|x| (Action(x.handle), x.status, "End", LogTab::End)),
        ]
        .into_iter()
        .flatten()
        .collect()
    };

    let active_log = logs
        .iter()
        .find(|(.., tab)| tab == &log_tab)
        .map(|(handle, ..)| handle);

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
            <div class="tabs">
                {logs
                    .clone()
                    .into_iter()
                    .map(|(log, status, title, tab)| {
                        view! {
                            <LogTabHeader
                                title
                                href=href(tab)
                                status=move || status
                                active=tab == log_tab
                            />
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>
            <div class="active">
                {active_log.map(|handle| view! { <LiveLog lines=fetch_log(handle.clone())/> })}
            </div>
        </div>
    }
}

fn collect_multiple<T: std::hash::Hash + Eq, V>(
    it: impl Iterator<Item = (T, V)>,
) -> HashMap<T, Vec<V>> {
    let mut hashmap: HashMap<T, Vec<V>> = HashMap::new();
    for (key, value) in it {
        hashmap.entry(key).or_insert(vec![]).push(value);
    }
    hashmap
}

#[component]
fn Main(
    #[allow(unused)]
    #[prop(into)]
    info: responses::EvaluationInfo,
    #[prop(into)] tab: Signal<crate::routes::EvaluationTab>,
) -> impl IntoView {
    let active_tab = tab;
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
    let mk_item = |tab: EvaluationTab, icon, contents: View| {
        let handle = info.handle.clone();
        view! { class=item_style,
            <li class:active={
                let tab = tab.clone();
                move || active_tab().drop_log_tab() == tab.drop_log_tab()
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
    let items = collect_multiple(
        info.jobs
            .clone()
            .into_iter()
            .map(|(responses::JobSystemName { system, name }, info)| (system, (name, info))),
    );

    let job_items = items
        .iter()
        .map(move |(system, jobs)| {
            let system = system.clone();
            view! {
                <section>
                    <h1>{system}</h1>
                    <ul style="padding: 0;">
                        {jobs
                            .into_iter()
                            .map(|(name, info)| {
                                let last_run = info.last_run.clone();
                                mk_item(
                                    EvaluationTab::Job {
                                        handle: info.handle.clone(),
                                        log_tab: LogTab::default(),
                                    },
                                    view! {
                                        // FIXME: why do I need to clone twice?
                                        <Status status=move || {
                                            TaskStatus::from(last_run.clone()).into()
                                        }/>
                                    },
                                    view! {
                                        // FIXME: why do I need to clone twice?
                                        // FIXME: why do I need to clone twice?
                                        <span>{name}</span>
                                    }
                                        .into_view(),
                                )
                            })
                            .collect::<Vec<_>>()}
                    </ul>
                </section>
            }
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
                        EvaluationTab::Summary,
                        view! { <Icon icon=Icon::from(BiHomeAltRegular)/> },
                        view! { Summary }.into_view(),
                    )}

                </ul>
            </section>
            {job_items}
        </nav>
        // <div>
        <div class="contents">

            {
                let jobs = info.jobs.clone();
                move || {
                    match active_tab() {
                        EvaluationTab::Summary => "Summary page, todo".into_view(),
                        EvaluationTab::Job { handle, log_tab } => {
                            let job = jobs
                                .clone()
                                .into_iter()
                                .find(|(_, info)| info.handle == handle)
                                .unwrap()
                                .1;
                            view! {
                                // FIXME: why do we need to clone twice?
                                <JobSubpage job log_tab/>
                            }
                        }
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
            .unwrap_or_default()
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
