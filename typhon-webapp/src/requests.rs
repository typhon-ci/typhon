use crate::secrets::get_token;

use typhon_types::*;

use leptos::*;

#[allow(dead_code)]
pub fn resource(
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
            handle_request(get_token(), req).await
        }
        move |_| aux(req.clone())
    };
    create_resource(source, fetcher)
}

#[server(HandleRequest, "/leptos")]
pub async fn handle_request(
    token: Option<String>,
    request: requests::Request,
) -> Result<Result<responses::Response, responses::ResponseError>, ServerFnError> {
    let token: Option<&[u8]> = token.as_ref().map(|password| password.as_bytes());
    let user = typhon_lib::User::from_token(token);
    Ok(typhon_lib::handle_request(user, request).await)
}
