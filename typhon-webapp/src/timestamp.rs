use seed::{prelude::*, *};
use time;

#[derive(Clone, Debug)]
pub enum Msg {
    StartHovering,
    StopHovering,
    Tick,
}

pub struct Model {
    date_time: time::OffsetDateTime,
    hovering: bool,
    timer_handle: StreamHandle,
}

fn now() -> time::OffsetDateTime {
    let timestamp = js_sys::Date::now();
    let duration = time::Duration::new(timestamp as i64 / 1000, 0);
    time::OffsetDateTime::UNIX_EPOCH + duration
}

pub fn init(orders: &mut impl Orders<Msg>, timestamp: &i64) -> Model {
    let date_time = time::OffsetDateTime::UNIX_EPOCH + time::Duration::new(*timestamp, 0);
    let hovering = false;
    let timer_handle = orders.stream_with_handle(streams::interval(100, || Msg::Tick));
    Model {
        date_time,
        hovering,
        timer_handle,
    }
}

pub fn update(msg: Msg, model: &mut Model, _orders: &mut impl Orders<Msg>) {
    use Msg::*;
    match msg {
        Tick => (),
        StartHovering => model.hovering = true,
        StopHovering => model.hovering = false,
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    // the stream is canceled on the handle drop
    let _ = model.timer_handle;

    let duration = now() - model.date_time;
    div![
        if model.hovering || duration > time::Duration::WEEK {
            format!("{}", model.date_time)
        } else if duration >= time::Duration::DAY {
            format!("{} days ago", duration.whole_days())
        } else if duration >= time::Duration::HOUR {
            format!("{} hours ago", duration.whole_hours())
        } else if duration >= time::Duration::MINUTE {
            format!("{} minutes ago", duration.whole_minutes())
        } else {
            format!("{} seconds ago", duration.whole_seconds())
        },
        ev(Ev::MouseEnter, |_| Msg::StartHovering),
        ev(Ev::MouseLeave, |_| Msg::StopHovering),
    ]
}
