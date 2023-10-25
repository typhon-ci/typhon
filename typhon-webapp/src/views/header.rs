use seed::{prelude::*, *};

pub fn view<Ms: Clone + 'static>(
    base_url: &Url,
    admin: bool,
    login_msg: Ms,
    logout_msg: Ms,
) -> Node<Ms> {
    use crate::Urls;

    let urls_1 = Urls::new(base_url);
    let urls_2 = Urls::new(base_url);
    header![
        main![a![
            raw![std::str::from_utf8(include_bytes!("../../assets/logo.svg")).unwrap()],
            span!["Typhon"],
            attrs! { At::Href => urls_1.home() }
        ]],
        nav![a!["Home", attrs! { At::Href => urls_2.home() }],],
        if admin {
            button!["Logout", ev(Ev::Click, |_| logout_msg)]
        } else {
            button![a!["Login", ev(Ev::Click, |_| login_msg)]]
        },
    ]
}
