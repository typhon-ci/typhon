use crate::prelude::*;
use typhon_types::responses::TaskStatus;

#[component]
pub fn RelativeTime(#[prop(into)] datetime: time::OffsetDateTime) -> impl IntoView {
    fn human_approx_duration(duration: time::Duration) -> String {
        let weeks = duration.whole_weeks();
        let plural = |n| if n == 1 { "" } else { "s" };
        if weeks > 0 {
            if weeks < 8 {
                return format!("{weeks} week{} ago", plural(weeks));
            }
            let months = weeks / 4;
            if months < 24 {
                return format!("{months} month{} ago", plural(months));
            }
            let years = months / 12;
            return format!("{years} year{} ago", plural(years));
        }
        let days = duration.whole_days();
        if days > 0 {
            return format!("{days} day{} ago", plural(days));
        }
        let hours = duration.whole_hours();
        let minutes = duration.whole_minutes();
        if minutes >= 100 {
            return format!("{hours} hour{} ago", plural(hours));
        }
        let seconds = duration.whole_seconds();
        if seconds >= 100 {
            return format!("{minutes} min{} ago", plural(minutes));
        }
        format!("{seconds} sec{} ago", plural(seconds))
    }
    let now = use_context::<crate::utils::CurrentTime>().unwrap().0;
    let duration = move || now() - datetime;
    view! {
        <time datetime=move || format!("{}s", duration().whole_seconds())>
            <Icon icon=icondata::BiCalendarEventRegular/>
            {move || human_approx_duration(duration())}
        </time>
    }
}

#[component]
pub fn Duration(#[prop(into)] duration: Signal<Option<time::Duration>>) -> impl IntoView {
    move || match duration() {
        Some(duration) => {
            let seconds = duration.whole_seconds();
            let minutes = duration.whole_minutes();
            let hours = duration.whole_hours();
            view! {
                <time datetime=format!(
                    "{}s",
                    seconds,
                )>
                    {if hours == 0 {
                        if minutes == 0 {
                            format!("{}s", seconds)
                        } else {
                            let seconds = seconds % 60;
                            format!("{}m {}s", minutes, seconds)
                        }
                    } else {
                        let minutes = minutes % 60;
                        format!("{}h {}m", hours, minutes)
                    }}

                </time>
            }
            .into_view()
        }
        _ => ().into_view(),
    }
}

#[component]
pub fn TaskStatusDuration(#[prop(into)] status: Signal<TaskStatus>) -> impl IntoView {
    view! {
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
    }
}
