use std::f64;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use game_shared::{GameState, Movement};
use ws_stream_wasm::{WsStream, WsMeta, WsMessage};
use futures::{stream, StreamExt, SinkExt};
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;


#[wasm_bindgen(start)]
pub async fn start() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let mut command_queue = Rc::new(RefCell::new(VecDeque::new()));
    {
        let mut queue = command_queue.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            match event.code().as_ref() {
                "KeyW" => queue.borrow_mut().push_back(Movement::Up),
                "KeyA" => queue.borrow_mut().push_back(Movement::Left),
                "KeyS" => queue.borrow_mut().push_back(Movement::Down),
                "KeyD" => queue.borrow_mut().push_back(Movement::Right),
                _ => {}
            }
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
        closure.forget();
    }
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
        ws_stream.send_all(&mut stream::iter(command_queue.clone().borrow_mut().drain(..).map(|m| Ok(WsMessage::Binary(m.serialize())))))
            .await.expect("Failed to send events.");
        let state = match ws_stream.next().await {
            None => unreachable!(),
            Some(message) => match message {
                WsMessage::Text(_) => unreachable!(),
                WsMessage::Binary(data) => GameState::deserialize(data.as_slice())
            }
        };
        state.render(&mut context);
    }
}

trait Render {
    fn render(&self, ctx: &mut web_sys::CanvasRenderingContext2d);
}

impl Render for GameState {
    fn render(&self, ctx: &mut web_sys::CanvasRenderingContext2d) {
        ctx.clear_rect(0.0, 0.0, ctx.canvas().unwrap().width().into(), ctx.canvas().unwrap().height().into());
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
            ctx.fill_text("Peach", (pos.x + 30.0).into(), (pos.y - 15.0).into()).unwrap();
        }
    }
}
