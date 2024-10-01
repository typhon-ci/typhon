use crate::prelude::*;
use data::TaskStatusKind;

#[derive(Clone, Debug)]
pub enum HybridStatusKind {
    EvalPending,
    EvalStopped,
    EvalSucceeded { build: TaskStatusKind },
}

#[component]
pub fn Status(#[prop(into)] status: Signal<TaskStatusKind>) -> impl IntoView {
    let status = Signal::derive(move || HybridStatusKind::EvalSucceeded { build: status() });
    view! { <HybridStatus status=status /> }
}

#[component]
pub fn HybridStatus(#[prop(into)] status: Signal<HybridStatusKind>) -> impl IntoView {
    let style = style! {
        .status {
            display: flex;
            display: flex;
            height: 100%;
            aspect-ratio: "1 / 1";
            text-align: center;
            align-items: flex-start;
            width: "1em";
            height: "1em";
            font-size: var(--status-font-size);
            color: var(--color-task-status);
        }
        .status[data-status=EvalPending] {
            color: var(--color-fg-muted);
        }
        .status[data-status=Pending] {
            position: relative;
            display: inline-block;
            animation-name: spin;
            animation-duration: 2000ms;
            animation-iteration-count: infinite;
            animation-timing-function: linear;
        }
        @keyframes spin {
            from { transform:rotate(0deg); }
            to { transform:rotate(360deg); }
        }
    };
    let data_status = match status() {
        HybridStatusKind::EvalSucceeded { build } => format!("{:?}", build),
        status => format!("{:?}", status),
    };
    view! { class=style,
        <span class="status" data-status=data_status>
            <span class="icon-wrapper">
                {move || {
                    use icondata::*;
                    let icon = match status() {
                        HybridStatusKind::EvalPending => BiLoaderRegular,
                        HybridStatusKind::EvalStopped => BiErrorAltRegular,
                        HybridStatusKind::EvalSucceeded { build } => {
                            match build {
                                TaskStatusKind::Success => BiCheckCircleSolid,
                                TaskStatusKind::Pending => BiLoaderAltRegular,
                                TaskStatusKind::Failure => BiXCircleSolid,
                                TaskStatusKind::Canceled => BiStopCircleRegular,
                            }
                        }
                    };
                    view! { <Icon icon=Icon::from(icon) /> }
                }}

            </span>
        </span>
    }
}
