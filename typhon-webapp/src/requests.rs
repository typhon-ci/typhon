use typhon_types::*;

use seed::prelude::*;

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

pub fn perform_request_aux<Ms: 'static, MsO: 'static>(
    orders: &mut impl Orders<MsO>,
    req: requests::Request,
    succ: impl FnOnce(responses::Response) -> Ms + 'static,
    err: impl FnOnce(responses::ResponseError) -> Ms + 'static,
) {
    orders.perform_cmd(async move { handle_request(&req).await.map(succ).unwrap_or_else(err) });
}

macro_rules! perform_request {
    ($orders: expr , $req: expr , $pat: pat => $body: expr , $err: expr $(,)?) => {
        let req = $req.clone();
        crate::requests::perform_request_aux(
            $orders,
            $req,
            move |rsp| match rsp {
                $pat => $body,
                rsp => {
                    gloo_console::log!(format!(
                        "perform_request: unexpected response {:#?} to request {:#?}",
                        rsp, req
                    ));
                    $err(responses::ResponseError::InternalError)
                }
            },
            $err,
        )
    };
}

pub(crate) use perform_request;
