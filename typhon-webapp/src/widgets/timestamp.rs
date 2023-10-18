use seed::{prelude::*, *};
use time;

#[derive(Clone, Debug)]
pub enum Msg {
    Tick,
}

pub struct Model {
    date_time: time::OffsetDateTime,
    timer_handle: StreamHandle,
}

fn now() -> time::OffsetDateTime {
    let timestamp = js_sys::Date::now();
    let duration = time::Duration::new(timestamp as i64 / 1000, 0);
    time::OffsetDateTime::UNIX_EPOCH + duration
}

pub fn init(orders: &mut impl Orders<Msg>, timestamp: &i64) -> Model {
    let date_time = time::OffsetDateTime::UNIX_EPOCH + time::Duration::new(*timestamp, 0);
    let timer_handle = orders.stream_with_handle(streams::interval(100, || Msg::Tick));
    Model {
        date_time,
        timer_handle,
    }
}

pub fn update(msg: Msg, _model: &mut Model, _orders: &mut impl Orders<Msg>) {
    use Msg::*;
    match msg {
        Tick => (),
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    // the stream is canceled on the handle drop
    let _ = model.timer_handle;

    let duration = now() - model.date_time;
    span![
        if duration > time::Duration::WEEK {
            format!(
                "{}-{:02}-{:02}",
                model.date_time.year(),
                model.date_time.month() as u8,
                model.date_time.day()
            )
        } else if duration >= 2 * time::Duration::DAY {
            format!("{} days ago", duration.whole_days())
        } else if duration >= time::Duration::DAY {
            format!("1 day ago")
        } else if duration >= 2 * time::Duration::HOUR {
            format!("{} hours ago", duration.whole_hours())
        } else if duration >= time::Duration::HOUR {
            format!("1 hour ago")
        } else if duration >= 2 * time::Duration::MINUTE {
            format!("{} minutes ago", duration.whole_minutes())
        } else if duration >= time::Duration::MINUTE {
            format!("1 minute ago")
        } else if duration >= 2 * time::Duration::SECOND {
            format!("{} seconds ago", duration.whole_seconds())
        } else {
            format!("just now")
        },
        attrs! { At::Title => format!(
                "{}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
                model.date_time.year(),
                model.date_time.month() as u8,
                model.date_time.day(),
                model.date_time.hour(),
                model.date_time.minute(),
                model.date_time.second(),
            ),
        },
    ]
}
