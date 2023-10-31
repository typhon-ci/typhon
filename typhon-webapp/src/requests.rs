use typhon_types::*;

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
