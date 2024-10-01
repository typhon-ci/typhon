use crate::prelude::*;

use im::Vector;
use std::collections::HashSet;

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.lines.len() == other.lines.len()
    }
}

impl Default for State {
    fn default() -> State {
        State {
            lines: Vector::new(),
            group_stack: Vector::new(),
            group_id: 0,
        }
    }
}
impl State {
    fn parse_line(self, line: String) -> State {
        let mut state = self;
        struct Command {
            name: String,
            rest: String,
        }
        fn parse_nix_phase(line: &str) -> Option<Command> {
            match &line
                .split(r#"@nix { "action": "setPhase", "phase": ""#)
                .collect::<Vec<_>>()[..]
            {
                ["", phase] if let Some(phase) = phase.split(r#"""#).next() => Some(Command {
                    name: "nix-phase".into(),
                    rest: phase.into(),
                }),
                _ => None,
            }
        }
        fn parse_command_line(line: &str) -> Option<Command> {
            match &line.split("::").collect::<Vec<_>>()[..] {
                ["", name, rest @ ..] => Some(Command {
                    name: name.to_string(),
                    rest: rest.join("::"),
                }),
                _ => None,
            }
        }
        match parse_command_line(line.as_str()).or_else(|| parse_nix_phase(line.as_str())) {
            Some(cmd) if cmd.name == "group" => {
                state.group_id += 1;
                let group_stack = state.group_stack.clone();
                state.group_stack.push_front(state.group_id);
                state.lines.push_back(Line {
                    contents: cmd.rest,
                    starts_group: Some(state.group_id),
                    group_stack,
                })
            }
            Some(cmd) if cmd.name == "nix-phase" => {
                state.group_id += 1;
                state.group_stack = Vector::new();
                state.group_stack.push_front(state.group_id);
                state.lines.push_back(Line {
                    contents: cmd.rest,
                    starts_group: Some(state.group_id),
                    group_stack: Vector::new(),
                });
                return state;
            }
            Some(cmd) if cmd.name == "endgroup" => {
                state.group_stack.pop_front();
            }
            _ => state.lines.push_back(Line {
                contents: line,
                starts_group: None,
                group_stack: state.group_stack.clone(),
            }),
        }
        state
    }
}

type GroupId = usize;
#[derive(Debug, Clone)]
struct Line {
    contents: String,
    starts_group: Option<GroupId>,
    group_stack: Vector<GroupId>,
}
#[derive(Debug, Clone)]
struct State {
    lines: Vector<Line>,
    group_stack: Vector<GroupId>,
    group_id: GroupId,
}

#[component]
pub fn LiveLog(#[prop(into)] lines: leptos::ReadSignal<Option<String>>) -> impl IntoView {
    let contents = Signal::derive(move || {
        lines()
            .map(|contents| {
                strip_ansi_escapes::strip_str(contents)
                    .split("\n")
                    .map(|s| s.to_owned())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(|| vec![])
    });
    view! { <Log contents /> }
}

#[component]
pub fn Log(#[prop(into)] contents: Signal<Vec<String>>) -> impl IntoView {
    let styler_class = style! {
        .log :deep(.line.hidden) {
            display: none;
        }
        .log :deep(.line span.n) {
            overflow: hidden;
            text-align: right;
            text-overflow: ellipsis;
            white-space: nowrap;
            user-select: nowrap;
            display: inline-block;
            opacity: 0.5;
        }
        .log :deep(.line) {
            display: grid;
            align-content: baseline;
            grid-template-columns: "35px" 1fr;
            white-space: normal;
        }
        .log :deep(.line pre) {
            display: inline-block;
            margin: 0px;
        }
        .log {
            font-family: JetBrains Mono, sans-serif;
        }
        .log :deep(.icon svg) {
            width: "0.8em";
            padding-right: 4px;
            transform: translateY("0.1em");
        }
    };
    let (close, set_close) = create_signal(HashSet::<GroupId>::new());
    let state: Memo<State> = create_memo(move |state: Option<&State>| {
        let mut state = state.cloned().unwrap_or_default();
        for line in contents.get() {
            state = state.parse_line(line)
        }
        state
    });
    let is_hidden = move |line: &Line| line.group_stack.iter().any(|id| close().contains(id));
    view! { class=styler_class,
        <div class="log">
            <For
                each=move || {
                    state.get().lines.into_iter().take(3000).enumerate().collect::<Vec<_>>()
                }

                key=move |(i, _)| *i
                children=move |(i, line)| {
                    view! {
                        <div
                            class="line"
                            class:hidden={
                                let line = line.clone();
                                move || is_hidden(&line)
                            }

                            on:click=move |_| {
                                if let Some(id) = line.starts_group {
                                    set_close
                                        .update(move |map| {
                                            if !map.insert(id) {
                                                map.remove(&id);
                                            }
                                        })
                                }
                            }
                        >

                            <span class="n">{i + 1}</span>
                            <pre style=format!(
                                "margin-left: {}px;",
                                (line.group_stack.len() + 1) * 16,
                            )>
                                {move || {
                                    line.starts_group
                                        .map(|id| {
                                            let icon = if close().contains(&id) {
                                                icondata::BiRightArrowSolid
                                            } else {
                                                icondata::BiDownArrowSolid
                                            };
                                            view! {
                                                <span class="icon">
                                                    <Icon icon />
                                                </span>
                                            }
                                        })
                                }} {line.contents}
                            </pre>
                        </div>
                    }
                }
            />

        </div>
    }
}
