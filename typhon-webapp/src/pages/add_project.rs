use crate::prelude::*;

#[component]
pub(crate) fn AddProject() -> impl IntoView {
    let action = request_action!(
        CreateProject,
        |name: String, url: String, flake: Option<String>| requests::Request::CreateProject {
            name,
            decl: requests::ProjectDecl {
                url,
                flake: flake.is_some()
            },
        },
        |result| {
            // TODO: redirect when success
            // if let Ok(Ok(())) = result {
            //     leptos_actix::redirect("/");
            // }
            result
        }
    );
    let value: RwSignal<Option<_>> = action.value();
    let response = move || {
        value().map(|v| match v {
            Ok(Err(ResponseError::BadRequest(message))) => Err(message),
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Fatal error: {e:?}")),
        })
    };
    let style = style! {
        .header {
            font-size: var(--font-size-huge);
            padding-bottom: 2px;
        }
        .header-description {
            color: var(--color-fg-muted);
            padding-top: 4px;
            margin-bottom: 14px;
        }
        .sep {
            border-bottom: 1px solid var(--color-border-default);
            padding-bottom: 14px;
        }
        .page {
            padding-top: 22px;
            max-width: 600px;
            margin: auto;
        }
        .fields {
            display: flex;
            flex-direction: column;
            gap: 14px;
        }
        .field > label {
            display: block;
            padding-left: 2px;
            font-size: var(--font-size-small);
            font-weight: 400;
        }
        .field > input {
            width: 100%;
        }
        .field.flake {
            display: flex;
            flex-direction: column;
            gap: 16px;
        }
        .field.flake .option {
            display: grid;
            grid-template-areas: raw_str("radio icon title") raw_str("radio icon details");
            grid-template-columns: auto auto 1fr;
            align-items: center;
            column-gap: 5px;
        }
        .field.flake .option input {
            grid-area: radio;
        }
        .field.flake :deep(svg) {
            grid-area: icon;
            font-size: 20px;
        }
        .field.flake .option .kind {
            font-size: var(--font-size-small);
            font-weight: 400;
            grid-area: title;
            padding-bottom: 2px;
        }
        .field.flake .option .details {
            font-size: var(--font-size-small);
            grid-area: details;
            color: var(--color-fg-muted);
            padding-top: 2px;
        }
        button {
            background: var(--color-green-button-bg);
            color: white;
            width: auto!important;
            transition: all 100ms;
        }
        button:hover {
            transition: all 100ms;
            background: var(--color-green);
        }
        button:active {
            transition: all 100ms;
            background: var(--color-darkgreen);
        }
        .page :deep(.message) {
            text-align: justify;
            padding-bottom: 14px;
            display: flex;
            align-items: center;
            gap: 6px;
        }
        .page :deep(.error-message) {
            color: var(--color-danger-emphasis);
        }
        .page :deep(.success-message) {
            color: var(--color-success);
        }
        // TODO: Move to `app.rs`
        .page :deep(button) {
            border-radius: 6px;
            border: 1px solid var(--color-border-default);
            padding: 8px 10px;
            cursor: pointer;
            outline: inherit;
            font-size: inherit;
            font-family: inherit;
            font-weight: 400;
        }
    };
    view! { class=style,
        <div class="page">
            <ActionForm action>
                <div class="header">"Add a project"</div>
                <div class="header-description sep">
                    "Add a project declaration to Typhon. Note that the project will be visible to anyone that have access to this Typhon instance."
                </div>
                <div class="fields">
                    <div class="field">
                        <label class="label" for="name">
                            "Name"
                        </label>
                        <input type="text" name="name" class="input" id="name"/>
                    </div>
                    <div class="field sep">
                        <label class="label" for="url">
                            "Flake URL"
                        </label>
                        <input type="text" name="url" class="input" id="url"/>
                    </div>
                    <div class="field sep flake">

                        <div class="option">
                            <input name="flake" class="input" id="flake" type="radio" checked=true/>
                            <Icon icon=icondata::FaSnowflakeRegular/>
                            <label class="kind" for="flake">
                                "Flake"
                            </label>
                            <label class="details">
                                The project has a Typhon compatible <code>flake.nix</code>
                                <span>.</span> The project is a
                                <a href="https://nixos.wiki/wiki/Flakes">Nix Flake</a>
                                <span>.</span>
                            </label>
                        </div>

                        <div class="option">
                            <input name="flake" class="input" id="legacy" type="radio"/>
                            <Icon icon=icondata::BiCodeCurlyRegular/>
                            <label class="kind" for="legacy">
                                "Legacy"
                            </label>
                            <label class="details" for="legacy">
                                The project has a
                                <code>nix/typhon.nix</code>
                                file
                                <span>.</span>
                            </label>
                        </div>

                    </div>
                    <div class="field">
                        {move || {
                            match response() {
                                Some(Err(error)) => {
                                    view! {
                                        <div class="message error-message">
                                            <Icon icon=icondata::BiErrorSolid/>
                                            <div>{error}</div>
                                        </div>
                                    }
                                        .into_view()
                                }
                                Some(Ok(())) => {
                                    view! {
                                        <div class="message success-message">
                                            <Icon icon=icondata::BiCheckCircleSolid/>
                                            <div>"The project have been created successfully."</div>
                                        </div>
                                    }
                                        .into_view()
                                }
                                None => ().into_view(),
                            }
                        }}
                        <button type="submit" style="float: right;">
                            "Add project"
                        </button>
                    </div>
                </div>
            </ActionForm>
        </div>
    }
}
