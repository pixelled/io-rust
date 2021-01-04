use futures::stream::select;
use futures::{stream, SinkExt, Stream, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};
use game_shared::{Operation, PlayerState, RenderState, MAP_HEIGHT, MAP_WIDTH};
use gloo::events::EventListener;
use piet::kurbo::{Circle, CircleSegment, Rect};
use piet::{
    Color, FontFamily, RenderContext, Text, TextAlignment, TextAttribute, TextLayout,
    TextLayoutBuilder,
};
use piet_web::{WebRenderContext, WebTextLayout};
use pin_project::pin_project;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::{f32, f64};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast, JsValue};
use web_sys::{ImageData, KeyboardEvent, MouseEvent};
use ws_stream_wasm::{WsMessage, WsMeta, WsStream};

fn with_latest<S, A>(src: S, acc: A) -> WithLatest<S, A>
where
    S: Stream,
    A: Stream,
{
    WithLatest {
        src,
        acc,
        val: None,
    }
}

#[pin_project]
struct WithLatest<S, A>
where
    S: Stream,
    A: Stream,
{
    #[pin]
    src: S,
    #[pin]
    acc: A,
    val: Option<<A as Stream>::Item>,
}

impl<S, A> Stream for WithLatest<S, A>
where
    S: Stream + Unpin,
    A: Stream + Unpin,
{
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
            up: false,
            left: false,
            down: false,
            right: false,
            cursor: (0, 0),
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
        .map_err(|_| ())
        .unwrap();
    let name_state = Mutable::new(false);
    let name_state1 = name_state.clone();
    let _enter_listener = EventListener::new(&name_input, "keydown", move |event| {
        let event: &KeyboardEvent = event.dyn_ref().unwrap_throw();
        // let mut state = name_state1.lock_mut();
        if let "Enter" = event.code().as_ref() {
            name_state1.set(true);
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
    })
    .forget();
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
    })
    .forget();

    // TODO calculate degree based on position
    let control_state3 = control_state.clone();
    let center_x = canvas.width() / 2;
    let center_y = canvas.height() / 2;
    EventListener::new(&document, "mousemove", move |event| {
        let event: &MouseEvent = event.dyn_ref().unwrap_throw();
        let mut state = control_state3.lock_mut();
        state.cursor = (
            event.client_x() - center_x as i32,
            event.client_y() - center_y as i32,
        );
    })
    .forget();

    // Wait for username input.
    let mut name_stream = name_state.signal().to_stream();
    name_stream.next().await;
    name_stream.next().await;
    name_input.style().set_property("display", "none");

    let window = web_sys::window().expect("Window doesn't exist.");
    let can_width = canvas.width() as f32;
    let can_height = canvas.height() as f32;
    let mut piet_ctx = WebRenderContext::new(context, window);

    let render_states = Rc::new(RefCell::new(Vec::new()));
    let render_states1 = render_states.clone();

    let (mut ws_meta, mut ws_stream) = WsMeta::connect("ws://127.0.0.1:8080/ws", None)
        .await
        .expect("Websocket connection failed.");
    ws_stream
        .send(WsMessage::Binary(
            Operation::Join(name_input.value()).serialize(),
        ))
        .await;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let control_state_signal = control_state.signal();
    let mut stream = with_latest(ws_receiver, control_state_signal.to_stream()).filter_map(
        move |(message, state)| {
            if let WsMessage::Binary(data) = message {
                (*render_states1.borrow_mut()).push(RenderState::deserialize(data.as_slice()));
            };
            futures::future::ready(state)
        },
    );

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let s = (*render_states.borrow_mut()).len() as i32;
        if let Some(mut render_state) = (*render_states.borrow_mut()).pop() {
            render_state.render(&mut piet_ctx, can_width, can_height);
        }
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));
    request_animation_frame(g.borrow().as_ref().unwrap());

    while let Some(state) = stream.next().await {
        ws_sender
            .send(WsMessage::Binary(
                Operation::Update(state.state()).serialize(),
            ))
            .await;
    }
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .expect("no global `window` exists")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

trait Render {
    fn render(&mut self, ctx: &mut WebRenderContext, w: f32, h: f32);
}

impl Render for RenderState {
    fn render(&mut self, piet_ctx: &mut WebRenderContext, can_width: f32, can_height: f32) {
        let offset_x = self.self_pos.x - can_width / 2.0;
        let offset_y = self.self_pos.y - can_height / 2.0;

        piet_ctx.clear(Color::rgb8(36, 39, 44));

        self.static_pos.iter().for_each(|pos| {
            let x = (pos.x - offset_x) as f64;
            let y = (pos.y - offset_y) as f64;
            let pt = (x, y);
            let shape = Circle::new(pt, 20.0);
            let brush = piet_ctx.solid_brush(Color::grey(0.5));
            piet_ctx.fill(&shape, &brush);
        });

        self.celestial_pos.iter().for_each(|pos| {
            let x = (pos.x - offset_x) as f64;
            let y = (pos.y - offset_y) as f64;
            let pt = (x, y);
            let shape = Circle::new(pt, 500.0);
            let brush = piet_ctx.solid_brush(Color::grey(1.0));
            piet_ctx.fill(&shape, &brush);
        });

        for (name, pos, ori) in self.positions.drain(..) {
            let x = (pos.x - offset_x) as f64;
            let y = (pos.y - offset_y) as f64;

            // Render body.
            let pt = (x, y);
            let shape = Circle::new(pt, 20.0);
            let brush = piet_ctx.solid_brush(Color::SILVER);
            piet_ctx.fill(&shape, &brush);
            let brush1 = piet_ctx.solid_brush(Color::grey(0.9));
            piet_ctx.stroke(&shape, &brush1, 5.0);

            // Render shield.
            let shape = CircleSegment::new((x, y), 45.0, 40.0, ori as f64 - 0.85, 1.7);
            piet_ctx.stroke(&shape, &brush, 5.0);

            // Render text.
            let layout = piet_ctx
                .text()
                .new_text_layout(name)
                .default_attribute(TextAttribute::FontSize(24.0))
                .default_attribute(TextAttribute::TextColor(Color::grey(0.9)))
                .build()
                .unwrap();
            piet_ctx.draw_text(&layout, (x - layout.size().width / 2.0, y - 80.0));
        }

        let map_len: f64 = 150.0;
        let map_x = can_width as f64 - map_len - 50.0;
        let map_y = can_height as f64 - map_len - 50.0;
        let shape = Rect::new(map_x, map_y, map_x + map_len, map_y + map_len);
        let brush = piet_ctx.solid_brush(Color::grey(0.8));
        piet_ctx.fill(&shape, &brush);
        let brush = piet_ctx.solid_brush(Color::grey(0.3));
        piet_ctx.stroke(&shape, &brush, 7.0);

        let shape = Circle::new(
            (
                map_x + (self.self_pos.x / MAP_WIDTH) as f64 * map_len,
                map_y + (self.self_pos.y / MAP_HEIGHT) as f64 * map_len,
            ),
            2.0,
        );
        let brush = piet_ctx.solid_brush(Color::BLACK);
        piet_ctx.fill(&shape, &brush);

        piet_ctx.finish().unwrap();
    }
}
