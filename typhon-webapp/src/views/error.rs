use typhon_types::*;

use seed::{prelude::*, *};

pub fn view<Ms: Clone + 'static>(
    base_url: &Url,
    err: &responses::ResponseError,
    msg_ignore: Ms,
) -> Node<Ms> {
    use crate::Urls;

    let urls = Urls::new(base_url);
    div![
        h2!["Error"],
        p![format!("{}", err)],
        button!["Go back", ev(Ev::Click, |_| msg_ignore)],
        a!["Home", attrs! { At::Href => urls.home() }],
    ]
}
