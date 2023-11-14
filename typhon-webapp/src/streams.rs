use crate::secrets::get_token;

use typhon_types::*;

use async_stream::stream;
use futures_core::stream::Stream;
use gloo_console::log;
use gloo_net::http;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::Promise;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = "export async function read_chunk_by_chunk(reader) {
    let next = async () => {
        let o = await reader.read();
        return o.done ? null : {chunk: new TextDecoder().decode(o.value), next};
    };
    return next();
 }
")]
extern "C" {
    fn read_chunk_by_chunk(reader: js_sys::Object) -> Promise;
}

pub fn fetch_as_stream(req: http::Request) -> impl Stream<Item = String> + 'static {
    stream! {
        let res = req
            .send()
            .await
            .map_err(|e| gloo_console::log!(format!("network error {:?}", e)))
            .unwrap();
        let body = res.body();
        let readable_stream: web_sys::ReadableStream = body.unwrap();
        let reader: js_sys::Object = readable_stream.get_reader();
        let promise = read_chunk_by_chunk(reader);
        let mut maybe_promise = Some(promise);
        while let Some(promise) = maybe_promise {
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            let it = future.await.unwrap();
            if it.is_null() {
                maybe_promise = None;
            } else {
                let o = js_sys::Object::from(it);
                let chunk = js_sys::Reflect::get(&o, &"chunk".into()).unwrap();
                let value = chunk.into_serde().unwrap();
                yield value;
                let next = js_sys::Function::from(js_sys::Reflect::get(&o, &"next".into()).unwrap());
                let promise =
                    js_sys::Reflect::apply(&next, &js_sys::Object::new(), &js_sys::Array::new())
                    .unwrap();
                maybe_promise = Some(promise.into());
            }
        }
    }
}

#[allow(dead_code)]
pub fn events_stream() -> impl Stream<Item = Event> + Unpin + 'static {
    let req = http::RequestBuilder::new("/api/events").method(http::Method::GET);
    let req = match get_token() {
        None => req,
        Some(token) => req.header(&"token", &token),
    };
    let req = req.build().unwrap();
    let s = stream! {
        for await chunk in fetch_as_stream(req) {
            let deserializer = serde_json::Deserializer::from_str(&chunk);
            for maybe_event in deserializer.into_iter() {
                match maybe_event {
                    Ok(event) => yield event,
                    Err(e) => log!(format!("failed to parse event: {:?}", e)),
                }
            }
        }
    };
    Box::pin(s)
}

#[allow(dead_code)]
pub fn filter_events(
    req: requests::Request,
    event: impl Stream<Item = Option<Event>> + 'static,
) -> impl Stream<Item = bool> + Unpin + 'static {
    let s = stream! {
        let mut x = false;
        for await maybe_event in event {
            if maybe_event.map(|event| event.invalidates(&req)).unwrap_or(true) {
                yield x;
                x = !x;
            }
        }
    };
    Box::pin(s)
}
