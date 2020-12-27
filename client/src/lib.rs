use std::f64;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use game_shared::{RenderState, Operation, PlayerState};
use ws_stream_wasm::{WsStream, WsMeta, WsMessage};
use futures::{stream, StreamExt, SinkExt, Stream};
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::stream::select;
use pin_project::pin_project;
use gloo::events::EventListener;
use futures_signals::signal::{Mutable, SignalExt};
use web_sys::{KeyboardEvent, MouseEvent};
use std::fmt::Debug;

fn with_latest<S, A>(src: S, acc: A) -> WithLatest<S, A> where S: Stream, A: Stream {
    WithLatest { src, acc, val: None }
}

#[pin_project]
struct WithLatest<S, A> where S: Stream, A: Stream {
    #[pin]
    src: S,
    #[pin]
    acc: A,
    val: Option<<A as Stream>::Item>,
}

impl<S, A> Stream for WithLatest<S, A> where S: Stream + Unpin, A: Stream + Unpin {
    type Item = (<S as Stream>::Item, Option<<A as Stream>::Item>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if let Poll::Ready(x) = this.acc.poll_next(cx) {
            *this.val = x;
        }
        match this.src.poll_next(cx) {
            Poll::Ready(Some(x)) => {
                let val = std::mem::take(this.val);
                Poll::Ready(Some((x, val)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct ControlState {
    up: bool,
    left: bool,
    down: bool,
    right: bool,
    cursor: (i32, i32),
}

impl ControlState {
    pub fn new() -> Self {
        ControlState {
            up: false, left: false, down: false, right: false, cursor: (0, 0),
        }
    }

    pub fn state(&self) -> PlayerState {
        let mut dx = 0;
        let mut dy = 0;
        dx -= self.left as i32;
        dx += self.right as i32;
        dy -= self.up as i32;
        dy += self.down as i32;
        if dx == 0 && dy == 0 {
            PlayerState {
                dir: None,
                ori: (self.cursor.1 as f32).atan2(self.cursor.0 as f32),
            }
        } else {
            PlayerState {
                dir: Some((dy as f32).atan2(dx as f32)),
                ori: (self.cursor.1 as f32).atan2(self.cursor.0 as f32),
            }
        }
    }

    pub fn press_up(&mut self) {
        self.up = true;
    }

    pub fn press_left(&mut self) {
        self.left = true;
    }

    pub fn press_down(&mut self) {
        self.down = true;
    }

    pub fn press_right(&mut self) {
        self.right = true;
    }

    pub fn release_up(&mut self) {
        self.up = false;
    }

    pub fn release_left(&mut self) {
        self.left = false;
    }

    pub fn release_down(&mut self) {
        self.down = false;
    }

    pub fn release_right(&mut self) {
        self.right = false;
    }
}

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

    let name_input = document.get_element_by_id("nameInput").unwrap();
    let name_input: web_sys::HtmlInputElement = name_input
        .dyn_into::<web_sys::HtmlInputElement>()
        .map_err(|_|())
        .unwrap();
    let name_state = Mutable::new(false);
    let name_state1 = name_state.clone();
    let _enter_listener = EventListener::new(&name_input, "keydown", move |event| {
        let event: &KeyboardEvent = event.dyn_ref().unwrap_throw();
        // let mut state = name_state1.lock_mut();
        match event.code().as_ref() {
            "Enter" => {
                name_state1.set(true);
            },
            _ => (),
        }
    });

    let control_state = Mutable::new(ControlState::new());
    let control_state1 = control_state.clone();
    EventListener::new(&document, "keydown", move |event| {
        let event: &KeyboardEvent = event.dyn_ref().unwrap_throw();
        let mut state = control_state1.lock_mut();
        match event.code().as_ref() {
            "KeyW" => state.press_up(),
            "KeyA" => state.press_left(),
            "KeyS" => state.press_down(),
            "KeyD" => state.press_right(),
            _ => (),
        }
    }).forget();
    let control_state2 = control_state.clone();
    EventListener::new(&document, "keyup", move |event| {
        let event: &KeyboardEvent = event.dyn_ref().unwrap_throw();
        let mut state = control_state2.lock_mut();
        match event.code().as_ref() {
            "KeyW" => state.release_up(),
            "KeyA" => state.release_left(),
            "KeyS" => state.release_down(),
            "KeyD" => state.release_right(),
            _ => (),
        }
    }).forget();

    // TODO calculate degree based on position
    let control_state3 = control_state.clone();
    let center_x = canvas.width() / 2;
    let center_y = canvas.height() / 2;
    EventListener::new(&document, "mousemove", move |event| {
        let event: &MouseEvent = event.dyn_ref().unwrap_throw();
        let mut state = control_state3.lock_mut();
        state.cursor = (event.client_x() - center_x as i32, event.client_y() - center_y as i32);
        let js1: JsValue = state.cursor.0.into();
        let js2: JsValue = state.cursor.1.into();
        web_sys::console::log_2(&js1, &js2);
    }).forget();

    let mut name_stream = name_state.signal().to_stream();
    name_stream.next().await;
    name_stream.next().await;
    name_input.style().set_property("display","none");

    let (mut ws_meta, mut ws_stream) = WsMeta::connect("ws://127.0.0.1:8080/ws", None).await.expect("Websocket connection failed.");
    ws_stream.send(WsMessage::Binary(Operation::Join(name_input.value()).serialize())).await;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let control_state_signal = control_state.signal();
    let mut stream = with_latest(ws_receiver, control_state_signal.to_stream())
        .filter_map(move |(message, state)| {
            if let WsMessage::Binary(data) = message {
                RenderState::deserialize(data.as_slice()).render(&mut context);
            };
            futures::future::ready(state)
        });
    while let Some(state) = stream.next().await {
        ws_sender.send(WsMessage::Binary(Operation::Update(state.state()).serialize())).await;
    }
}

trait Render {
    fn render(&self, ctx: &mut web_sys::CanvasRenderingContext2d);
}

impl Render for RenderState {
    fn render(&self, ctx: &mut web_sys::CanvasRenderingContext2d) {
        ctx.clear_rect(0.0, 0.0, ctx.canvas().unwrap().width().into(), ctx.canvas().unwrap().height().into());
        for (name, pos, ori) in self.positions.iter() {
            ctx.begin_path();
            // Render circle.
            ctx.set_fill_style(&JsValue::from_str("#13579B"));
            ctx.arc(pos.x.into(), pos.y.into(), 30.0, 0.0, f64::consts::PI * 2.0)
               .unwrap();
            ctx.stroke();
            ctx.fill();
            // Render small circle.
            ctx.set_fill_style(&JsValue::from_str("#000000"));
            ctx.arc((pos.x + 20.0 * ori.cos()).into(), (pos.y + 20.0 * ori.sin()).into(), 20.0, 0.0, f64::consts::PI * 2.0);
            ctx.stroke();
            // Render texts.
            ctx.set_font("30px Comic Sans MS");
            ctx.fill_text(name, (pos.x + 30.0).into(), (pos.y - 15.0).into()).unwrap();
        }
    }
}
