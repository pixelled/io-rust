use crate::render::{Interpolator, Render, RenderState};
use either::Either;
use futures::stream::select;
use futures::{stream, SinkExt, Stream, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};
use game_shared::{Operation, PlayerState, ViewSnapshot, MAP_HEIGHT, MAP_WIDTH};
use gloo::events::EventListener;
use piet::kurbo::{Circle, CircleSegment, Rect};
use piet::{
	Color, FontFamily, RenderContext, Text, TextAlignment, TextAttribute, TextLayout,
	TextLayoutBuilder,
};
use piet_web::{WebRenderContext, WebTextLayout};
use pin_project::pin_project;
use std::cell::{RefCell, UnsafeCell};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};
use std::{f32, f64};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast, JsValue};
use web_sys::{ImageData, KeyboardEvent, MouseEvent};
use ws_stream_wasm::{WsMessage, WsMeta, WsStream};

mod render;
mod util;

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
		ControlState { up: false, left: false, down: false, right: false, cursor: (0, 0) }
	}

	pub fn state(&self) -> PlayerState {
		let mut dx = 0;
		let mut dy = 0;
		dx -= self.left as i32;
		dx += self.right as i32;
		dy -= self.up as i32;
		dy += self.down as i32;
		if dx == 0 && dy == 0 {
			PlayerState { dir: None, ori: (self.cursor.1 as f32).atan2(self.cursor.0 as f32) }
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
	let canvas: web_sys::HtmlCanvasElement =
		canvas.dyn_into::<web_sys::HtmlCanvasElement>().map_err(|_| ()).unwrap();
	let mut context = canvas
		.get_context("2d")
		.unwrap()
		.unwrap()
		.dyn_into::<web_sys::CanvasRenderingContext2d>()
		.unwrap();

	let name_input = document.get_element_by_id("nameInput").unwrap();
	let name_input: web_sys::HtmlInputElement =
		name_input.dyn_into::<web_sys::HtmlInputElement>().map_err(|_| ()).unwrap();
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
		state.cursor = (event.client_x() - center_x as i32, event.client_y() - center_y as i32);
	})
	.forget();

	// Wait for username input.
	let mut name_stream = name_state.signal().to_stream();
	name_stream.next().await;
	name_stream.next().await;
	name_input.style().set_property("display", "none");

	let window = web_sys::window().expect("Window doesn't exist.");
	let perf = window.performance().expect("No Performance found.");
	let mut piet_ctx = WebRenderContext::new(context, window);

	let (mut ws_meta, mut ws_stream) = WsMeta::connect("ws://127.0.0.1:8080/ws", None)
		.await
		.expect("Websocket connection failed.");
	ws_stream.send(WsMessage::Binary(Operation::Join(name_input.value()).serialize())).await;
	let (mut ws_sender, mut ws_receiver) = ws_stream.split();
	let control_state_signal = control_state.signal();

	let mut key_frames = ws_receiver.filter_map(|message| match message {
		WsMessage::Binary(data) => futures::future::ready(Some(RenderState::from(
			ViewSnapshot::deserialize(data.as_slice()),
		))),
		_ => futures::future::ready(None),
	});
	let prev_frame = key_frames.next().await.unwrap();
	let next_frame = key_frames.next().await.unwrap();
	let mut interpolator = Interpolator::new(perf.now(), prev_frame, next_frame);
	let mut stream = util::merge(
		AnimationFrame::new(),
		util::with_latest(key_frames, control_state_signal.to_stream()),
	);

	while let Some(data) = stream.next().await {
		match data {
			Either::Left(time) => {
				interpolator.interpolate(time, &canvas).render(&mut piet_ctx)
			}
			Either::Right((render_state, control)) => {
				interpolator.update(perf.now(), render_state);
				if let Some(state) = control {
					ws_sender.send(WsMessage::Binary(Operation::Update(state.state()).serialize())).await;
				}
			}
		}
	}
}

struct AnimationState {
	pub timestamp: f64,
	pub waker: Option<Waker>,
	pub id: i32,
	pub closure: Closure<dyn FnMut(f64)>,
}

struct AnimationFrame {
	state: Rc<RefCell<Option<AnimationState>>>,
}

impl Stream for AnimationFrame {
	type Item = f64;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let mut state = self.get_mut().state.borrow_mut();
		let state = state.as_mut().unwrap();
		if state.timestamp.is_nan() {
			state.waker = Some(cx.waker().clone());
			Poll::Pending
		} else {
			let time = state.timestamp;
			state.timestamp = f64::NAN;
			Poll::Ready(Some(time))
		}
	}
}

impl AnimationFrame {
	fn new() -> AnimationFrame {
		let state = Rc::new(RefCell::new(None));

		let closure = {
			let state = state.clone();
			Closure::wrap(Box::new(move |time| {
				let mut state = state.borrow_mut();
				let state: &mut AnimationState = state.as_mut().unwrap();
				state.timestamp = time;
				state.id = request_animation_frame(&state.closure);
				if let Some(waker) = &state.waker {
					waker.wake_by_ref();
				}
			}) as Box<dyn FnMut(f64)>)
		};

		let id = request_animation_frame(&closure);
		*state.borrow_mut() =
			Some(AnimationState { timestamp: f64::NAN, waker: None, id, closure });
		AnimationFrame { state }
	}
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) -> i32 {
	web_sys::window()
		.expect("no global `window` exists")
		.request_animation_frame(f.as_ref().unchecked_ref())
		.expect("should register `requestAnimationFrame` OK")
}
