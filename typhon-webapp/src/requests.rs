use typhon_types::*;

use futures::Future;
use leptos::*;

use std::pin::Pin;

pub fn resource(
    event: ReadSignal<Option<Event>>,
    req: requests::Request,
) -> Resource<Option<bool>, Result<responses::Response, responses::ResponseError>> {
    use crate::streams::filter_events;
    let source = create_signal_from_stream(filter_events(req.clone(), event.to_stream()));
    let fetcher = {
        // FIXME
        // what is this witchcraft?
        // why does the fetcher below not work?
        fn aux(
            req: &requests::Request,
        ) -> Pin<Box<dyn Future<Output = Result<responses::Response, responses::ResponseError>>>>
        {
            let req = req.clone();
            Box::pin(async move { handle_request(&req).await })
        }
        move |_| aux(&req)
    };
    //let fetcher = {
    //    let req = req.clone();
    //    move |_| handle_request(&req)
    //};
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
