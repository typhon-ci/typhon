use leptos::*;
use leptos_icons::*;
use stylers::style;

#[component]
pub fn Status(success: bool) -> impl IntoView {
    let styler_class = style! {
        .status {
            color: red;
        }
        .status.success {
            color: green;
        }
    };
    let icon = if success {
        icondata::BiCheckCircleSolid
    } else {
        icondata::BiErrorCircleSolid
    };
    view! { class=styler_class,
        <span class="status" class:success=success>
            <Icon icon />
        </span>
    }
}
