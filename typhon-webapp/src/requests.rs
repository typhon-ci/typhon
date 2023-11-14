use typhon_types::*;

use leptos::*;

#[allow(dead_code)]
pub fn resource(
    event: ReadSignal<Option<Event>>,
    req: requests::Request,
) -> Resource<Option<bool>, Result<responses::Response, responses::ResponseError>> {
    use crate::streams::filter_events;
    let source = create_signal_from_stream(filter_events(req.clone(), event.to_stream()));
    let fetcher = {
        async fn aux(
            req: requests::Request,
        ) -> Result<responses::Response, responses::ResponseError> {
            handle_request(&req).await
        }
        move |_| aux(req.clone())
    };
    create_resource(source, fetcher)
}

pub async fn handle_request(
    request: &requests::Request,
) -> Result<responses::Response, responses::ResponseError> {
    use crate::secrets;
    use crate::settings;

    use gloo_net::http;

    let settings = settings::Settings::load();
    let token = secrets::get_token();
    let req = http::RequestBuilder::new(&settings.api_url).method(http::Method::POST);
    let req = match token {
        None => req,
        Some(token) => req.header("token", &token),
    };
    req.json(request)
        .unwrap()
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}
