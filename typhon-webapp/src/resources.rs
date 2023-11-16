use crate::server_fn;

use typhon_types::*;

use leptos::*;

#[allow(dead_code)]
pub fn request(
    event: ReadSignal<Option<Event>>,
    req: requests::Request,
) -> Resource<
    Option<bool>,
    Result<Result<responses::Response, responses::ResponseError>, ServerFnError>,
> {
    use crate::streams::filter_events;
    let source = create_signal_from_stream(filter_events(req.clone(), event.to_stream()));
    let fetcher = {
        async fn aux(
            req: requests::Request,
        ) -> Result<Result<responses::Response, responses::ResponseError>, ServerFnError> {
            server_fn::handle_request(req).await
        }
        move |_| aux(req.clone())
    };
    create_resource(source, fetcher)
}
