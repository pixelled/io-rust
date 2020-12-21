use std::f64;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use game_shared::State;
use ws_stream_wasm::{WsStream, WsMeta, WsMessage};
use futures::StreamExt;


#[wasm_bindgen(start)]
pub async fn start() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();

    let mut context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let (mut ws_meta, mut ws_stream) = WsMeta::connect("ws://127.0.0.1:8080/ws", None).await.expect("Websocket connection failed.");
    loop {
        let state = match ws_stream.next().await {
            None => unreachable!(),
            Some(message) => match message {
                WsMessage::Text(_) => unreachable!(),
                WsMessage::Binary(data) => State::deserialize(data.as_slice())
            }
        };
        state.render(&mut context);
    }
}

trait Render {
    fn render(&self, ctx: &mut web_sys::CanvasRenderingContext2d);
}

impl Render for State {
    fn render(&self, ctx: &mut web_sys::CanvasRenderingContext2d) {
        for (id, pos) in self.positions.iter() {
            ctx.begin_path();
            // Draw a circle.
            ctx.set_fill_style(&JsValue::from_str("#13579B"));
            ctx.arc(pos.x.into(), pos.y.into(), 50.0, 0.0, f64::consts::PI * 2.0)
               .unwrap();
            ctx.stroke();
            ctx.fill();
            // Render texts.
            ctx.set_fill_style(&JsValue::from_str("#000000"));
            ctx.set_font("30px Comic Sans MS");
            ctx.fill_text("Peach", 30.0, 15.0).unwrap();
        }
    }
}
