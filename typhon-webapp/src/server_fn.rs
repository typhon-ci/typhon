use typhon_types::*;

use leptos::*;

#[cfg(feature = "ssr")]
use actix_session::Session;
#[cfg(feature = "ssr")]
use leptos_actix::{extract, redirect};
#[cfg(feature = "ssr")]
use typhon_lib::User;

#[server(HandleRequest, "/leptos")]
pub async fn handle_request(
    request: requests::Request,
) -> Result<Result<responses::Response, responses::ResponseError>, ServerFnError> {
    use typhon_lib::User;
    let session = extract!(Session);
    let user: User = session
        .get("user")
        .map_err(|_| ServerFnError::ServerError("TODO".to_string()))?
        .unwrap_or(User::Anonymous);
    Ok(typhon_lib::handle_request(user, request).await)
}

#[server(Login, "/leptos")]
pub async fn login(
    password: String,
) -> Result<Result<(), responses::ResponseError>, ServerFnError> {
    let res = handle_request(requests::Request::Login { password }).await;
    if matches!(res, Ok(Ok(responses::Response::Ok))) {
        let session = extract!(Session);
        session
            .insert("user", User::Admin)
            .map_err(|_| ServerFnError::ServerError("TODO".to_string()))?;
        redirect("/");
    }
    match res {
        Ok(Ok(responses::Response::Ok)) => {
            let session = extract!(Session);
            session
                .insert("user", User::Admin)
                .map_err(|_| ServerFnError::ServerError("TODO".to_string()))?;
            redirect("/");
            Ok(Ok(()))
        }
        Ok(Ok(_)) => Err(ServerFnError::ServerError(
            "inconsistant server response".to_string(),
        )),
        Ok(Err(e)) => Ok(Err(e)),
        Err(e) => Err(e),
    }
}

#[server(Logout, "/leptos")]
pub async fn logout() -> Result<(), ServerFnError> {
    let session = extract!(Session);
    session.remove("user");
    redirect("/");
    Ok(())
}

#[server(CreateProject, "/leptos")]
pub async fn create_project(
    name: String,
    url: String,
    flake: Option<String>,
) -> Result<Result<(), responses::ResponseError>, ServerFnError> {
    let flake = flake.is_some();
    let res = handle_request(requests::Request::CreateProject {
        name,
        decl: requests::ProjectDecl { url, flake },
    })
    .await;
    match res {
        Ok(Ok(responses::Response::Ok)) => Ok(Ok(())),
        Ok(Ok(_)) => Err(ServerFnError::ServerError(
            "inconsistant server response".to_string(),
        )),
        Ok(Err(e)) => Ok(Err(e)),
        Err(e) => Err(e),
    }
}
