pub mod core {
    use typhon_types::*;

    use async_stream::stream;
    use either::Either::*;
    use futures::StreamExt;
    use leptos::*;

    pub fn filter_events(
        req: Signal<requests::Request>,
        event: ReadSignal<Option<Event>>,
    ) -> Signal<(requests::Request, usize)> {
        let count_stream = stream! {
            let mut req_stream = req.to_stream();
            let event_stream = event.to_stream();
            let init_req = req_stream.next().await.unwrap(); // There is always at least one request in the stream
            let mut cur_req = init_req;
            let mut select_stream = futures::stream::select(req_stream.map(Left), event_stream.map(Right));
            let mut count = 0;
            while let Some(x) = select_stream.next().await {
                match x {
                    Left(new_req) => {
                        cur_req = new_req;
                    }
                    Right(event) => {
                        if event.map(|event| event.invalidates(&cur_req)).unwrap_or(true) {
                            count += 1;
                        }
                    }
                }
                yield count;
            }
        };
        let count = create_signal_from_stream(Box::pin(count_stream));
        Signal::derive(move || (req(), count().unwrap_or(0)))
    }

    #[server(HandleRequest, "/leptos", "Url", "handle_request")]
    pub async fn handle_request(
        request: requests::Request,
    ) -> Result<Result<responses::Response, responses::ResponseError>, ServerFnError> {
        use actix_session::Session;
        use leptos_actix::extract;
        use typhon_core::User;
        let session: Session = extract().await?;
        let user: User = session
            .get("user")
            .map_err(|_| {
                ServerFnError::<server_fn::error::NoCustomError>::ServerError("TODO".to_string())
            })?
            .unwrap_or(User::Anonymous);
        Ok(typhon_core::handle_request(user, request).await)
    }
}

macro_rules! handle_request {
    ($req:expr, |$res:pat_param| $body:expr) => {
        match $crate::handle_request::core::handle_request($req).await {
            Ok(Ok($res)) => Ok(Ok($body)),
            #[allow(unused)]
            Ok(Ok(_)) => panic!("broken invariant"),
            Ok(Err(e)) => Ok(Err(e)),
            Err(e) => Err(e),
        }
    };
}

macro_rules! resource {
    ($req:expr, |$res:pat_param| $body:expr) => {{
        let req = $req;
        let event: ReadSignal<Option<typhon_types::Event>> =
            use_context::<AllEvents>().unwrap().inner();
        let source = $crate::handle_request::core::filter_events(req, event);
        let fetcher = {
            async fn aux(
                req: requests::Request,
            ) -> Result<Result<responses::Response, responses::ResponseError>, ServerFnError>
            {
                $crate::handle_request::core::handle_request(req).await
            }
            move |(req, _)| aux(req)
        };
        let resource = create_resource(source, fetcher);
        let res = Signal::derive(move || match resource() {
            Some(Ok(Ok($res))) => Some(Ok($body)),
            #[allow(unused)]
            Some(Ok(Ok(_))) => panic!("broken invariant"),
            Some(Ok(Err(e))) => Some(Err(e)),
            Some(Err(_)) => None,
            None => None,
        });
        (
            Signal::derive(move || res().map(|x| x.err()).flatten()),
            Signal::derive(move || res().transpose().ok().flatten()),
        )
    }};
}

macro_rules! search {
    ($offset:expr, $limit:expr, $req:expr, |$total:pat_param, $res:pat_param| $body:expr) => {
        $crate::handle_request::resource!(
            Signal::derive(
                move || requests::Request::Search(requests::search::Request {
                    limit: $limit(),
                    offset: $offset(),
                    kind: $req()
                })
            ),
            |responses::Response::Search(responses::search::Info {
                 total: $total,
                 results: $res,
             })| $body
        )
    };
}

macro_rules! request_action {
    ($name:ident, |$($id:ident : $ty:ty),*| $body: expr) => {
        {
            #[server($name, "/leptos")]
            async fn f(
                $($id : $ty,)*
            ) -> Result<Result<(), responses::ResponseError>, ServerFnError> {
                handle_request!(
                    $body,
                    |_| ()
                )
            }
            create_server_action::<$name>()
        }
    };
}

pub(crate) use core::HandleRequest;
pub(crate) use {handle_request, request_action, resource, search};
