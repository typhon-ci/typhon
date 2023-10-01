use futures_core::stream::Stream;
use gloo_net::http;
use gloo_utils::format::JsValueSerdeExt;
use seed::prelude::js_sys::Promise;
use seed::prelude::*;

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

pub fn fetch_as_stream(req: http::Request) -> impl Stream<Item = String> {
    use crate::*;
    use async_stream::stream;
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
