use typhon_types::*;

use leptos::*;

#[server(HandleRequest, "/leptos")]
pub async fn handle_request(
    token: Option<String>,
    request: requests::Request,
) -> Result<Result<responses::Response, responses::ResponseError>, ServerFnError> {
    let token: Option<&[u8]> = token.as_ref().map(|password| password.as_bytes());
    let user = typhon_lib::User::from_token(token);
    Ok(typhon_lib::handle_request(user, request).await)
}
