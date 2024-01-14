#![cfg(feature = "hydrate")]

use typhon_types::*;

use async_stream::stream;
use futures::future::FutureExt;
use futures_core::stream::Stream;
use futures_util::stream::StreamExt;
use gloo_console::log;
use gloo_net::http;
//use gloo_utils::format::JsValueSerdeExt;
//use js_sys::Promise;
//use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_streams::readable::*;

pub fn fetch_as_stream(req: http::Request) -> impl Stream<Item = String> {
    async move {
        let res = req
            .send()
            .await
            .map_err(|e| gloo_console::log!(format!("network error {:?}", e)))
            .unwrap();
        let body = res.body();
        let readable_stream: web_sys::ReadableStream = body.unwrap();
        let readable_stream: sys::ReadableStream = readable_stream.unchecked_into();
        let readable_stream: ReadableStream = ReadableStream::from_raw(readable_stream);
        readable_stream
            .into_stream()
            .filter_map(|item| core::future::ready(item.ok()))
            .map(|item| {
                let text_decoder = web_sys::TextDecoder::new().unwrap();
                let buffer = text_decoder
                    .decode_with_buffer_source(&item.into())
                    .unwrap();
                buffer
                    .strip_suffix("\n")
                    .map(|s| s.to_owned())
                    .unwrap_or(buffer)
            })
    }
    .into_stream()
    .flatten()
}

pub fn fetch_as_signal(req: http::Request) -> leptos::ReadSignal<Option<String>> {
    leptos::create_signal_from_stream(Box::pin(fetch_as_stream(req)))
}

pub fn events_stream() -> impl Stream<Item = Event> + Unpin + 'static {
    let req = http::RequestBuilder::new("/api/events").method(http::Method::GET);
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

//pub fn filter_events(
//    req: requests::Request,
//    event: impl Stream<Item = Option<Event>> + 'static,
//) -> impl Stream<Item = bool> + Unpin + 'static {
//    let s = stream! {
//        let mut x = false;
//        for await maybe_event in event {
//            if maybe_event.map(|event| event.invalidates(&req)).unwrap_or(true) {
//                yield x;
//                x = !x;
//            }
//        }
//    };
//    Box::pin(s)
//}
