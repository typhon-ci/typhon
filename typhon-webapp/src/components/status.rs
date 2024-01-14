use crate::prelude::*;
use data::TaskStatusKind;

#[component]
pub fn Status(#[prop(into)] status: Signal<TaskStatusKind>) -> impl IntoView {
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
        }
        .status[data-status=Success] {
            color: var(--color-success);
        }
        .status[data-status=Error] {
            color: var(--color-danger);
        }
        .status[data-status=Canceled] {
            color: var(--color-fg-muted);
        }
        .status[data-status=Pending] {
            position: relative;
            display: inline-block;
            color: var(--color-orange);
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
    view! { class=style,
        <span class="status" data-status=move || format!("{:?}", status())>
            <span class="icon-wrapper">
                {move || {
                    view! {
                        <Icon icon=Icon::from(
                            match status() {
                                TaskStatusKind::Success => BiCheckCircleSolid,
                                TaskStatusKind::Pending => BiLoaderAltRegular,
                                TaskStatusKind::Error => BiXCircleSolid,
                                TaskStatusKind::Canceled => BiStopCircleRegular,
                            },
                        )/>
                    }
                }}

            </span>

        </span>
    }
}
