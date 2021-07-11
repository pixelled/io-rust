use game_shared::{
	CelestialView, PlayerView, Position, ShieldView, StaticView, ViewSnapshot, CELESTIAL_RADIUS,
	INIT_RADIUS, MAP_HEIGHT, MAP_WIDTH, SHIELD_RADIUS, VIEW_X, VIEW_Y,
};
use piet::kurbo::{Circle, CircleSegment, Line, Rect};
use piet::{Color, RenderContext, Text, TextAttribute, TextLayout, TextLayoutBuilder};
use piet_web::WebRenderContext;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct PlayerState {
	pub name: String,
	pub pos: Position,
	pub ori: f32,
	pub shield_pos: Position,
}

/// The scene ready for interpolation.
/// (All in absolute positions.)
pub struct RenderState {
	pub time: Duration,
	pub self_pos: Position,
	pub players: HashMap<u64, PlayerState>,
	pub static_pos: HashMap<u64, StaticView>,
	pub celestial_pos: HashMap<u64, CelestialView>,
}

/// The interpolated scene ready for rendering.
/// (All in relative positions except being specified.)
pub struct FinalView {
	/// Absolute position of the player.
	pub self_abs_pos: Position,
	/// Absolute position - offset = relative position
	pub offset: Position,
	pub self_pos: Position,
	pub players: Vec<PlayerState>,
	pub static_pos: Vec<StaticView>,
	pub celestial_pos: Vec<CelestialView>,
	pub map: MiniMap,
}

impl From<ViewSnapshot> for RenderState {
	fn from(view: ViewSnapshot) -> Self {
		RenderState {
			time: view.time,
			self_pos: view.self_pos,
			// Convert `PlayerView` into `PlayerState` to include shields' info.
			players: {
				let shields: HashMap<u64, ShieldView> = view.shield_info.into_iter().collect();
				view.players
					.into_iter()
					.map(|(id, player_view)| {
						(
							id,
							PlayerState {
								name: player_view.name,
								pos: player_view.pos,
								ori: player_view.ori,
								shield_pos: shields
									.get(&player_view.shield_id)
									.expect("No shields match this player.")
									.pos,
							},
						)
					})
					.collect()
			},
			static_pos: view.static_pos.into_iter().collect(),
			celestial_pos: view.celestial_pos.into_iter().collect(),
		}
	}
}

/// Every object that displays on the canvas should implement this trait.
pub trait Render {
	fn render(&self, ctx: &mut WebRenderContext);
}

impl Render for PlayerState {
	/// Render Players.
	fn render(&self, piet_ctx: &mut WebRenderContext) {
		let x = self.pos.x as f64;
		let y = self.pos.y as f64;

		// Render body.
		let shape = Circle::new((x, y), INIT_RADIUS as f64);
		let brush = piet_ctx.solid_brush(Color::SILVER);
		piet_ctx.fill(&shape, &brush);
		let brush1 = piet_ctx.solid_brush(Color::grey(0.9));
		piet_ctx.stroke(&shape, &brush1, 5.0);

		// Render shield.
		let x_shield = self.shield_pos.x;
		let y_shield = self.shield_pos.y;
		let shape = Circle::new((x_shield as f64, y_shield as f64), SHIELD_RADIUS as f64);
		let brush = piet_ctx.solid_brush(Color::grey(0.7));
		piet_ctx.fill(&shape, &brush);

		// Render text.
		let layout = piet_ctx
			.text()
			.new_text_layout(self.name.clone())
			.default_attribute(TextAttribute::FontSize(24.0))
			.default_attribute(TextAttribute::TextColor(Color::grey(0.9)))
			.build()
			.unwrap();
		piet_ctx.draw_text(&layout, (x - layout.size().width / 2.0, y - 80.0));
	}
}

impl Render for StaticView {
	/// Render non-player objects.
	fn render(&self, piet_ctx: &mut WebRenderContext) {
		let pt = (self.pos.x as f64, self.pos.y as f64);
		let shape = Circle::new(pt, INIT_RADIUS as f64);
		let brush = piet_ctx.solid_brush(Color::grey(0.5));
		piet_ctx.fill(&shape, &brush);
	}
}

impl Render for CelestialView {
	/// Render celestial bodies.
	fn render(&self, piet_ctx: &mut WebRenderContext) {
		let pt = (self.pos.x as f64, self.pos.y as f64);
		let shape = Circle::new(pt, CELESTIAL_RADIUS as f64);
		let brush = piet_ctx.solid_brush(Color::grey(1.0));
		piet_ctx.fill(&shape, &brush);
	}
}

struct Background {
	/// Absolute position of the player in the map.
	abs_pos: Position,
	offset_x: f64,
	offset_y: f64,
}

impl Render for Background {
	fn render(&self, piet_ctx: &mut WebRenderContext) {
		// TODO: highlight boundaries.
		let left: f64 = (self.abs_pos.x - VIEW_X / 2.0) as f64;
		let right: f64 = left + VIEW_X as f64;
		let up: f64 = (self.abs_pos.y - VIEW_Y / 2.0) as f64;
		let down: f64 = up + VIEW_Y as f64;

		let step_x = 100.0;
		let step_y = 100.0;

		let x_min = (left - left % step_x).max(0.0) - self.offset_x;
		let y_min = (up - up % step_y).max(0.0) - self.offset_y;
		let x_max = (right).min(MAP_WIDTH as f64) - self.offset_x;
		let y_max = (down).min(MAP_HEIGHT as f64) - self.offset_y;

		let brush = piet_ctx.solid_brush(Color::grey(0.5));

		let mut x = x_min;
		let mut y = y_min;
		while x < x_max {
			// Vertical
			let line = Line::new((x, y_min), (x, y_max));
			piet_ctx.stroke(&line, &brush, 1.0);
			x += step_x;
		}
		while y < y_max {
			// Horizontal
			let line = Line::new((x_min, y), (x_max, y));
			piet_ctx.stroke(&line, &brush, 1.0);
			y += step_y;
		}
	}
}

impl Render for FinalView {
	/// Render the final scene.
	fn render(&self, piet_ctx: &mut WebRenderContext) {
		piet_ctx.clear(Color::rgb8(36, 39, 44));

		let bg = Background {
			abs_pos: self.self_abs_pos,
			offset_x: self.offset.x as f64,
			offset_y: self.offset.y as f64,
		};
		bg.render(piet_ctx);

		self.players.iter().for_each(|player_view| {
			player_view.render(piet_ctx);
		});
		self.static_pos.iter().for_each(|static_pos| {
			static_pos.render(piet_ctx);
		});
		self.celestial_pos.iter().for_each(|celestial_pos| {
			celestial_pos.render(piet_ctx);
		});
		self.map.render(piet_ctx);

		piet_ctx.finish().unwrap();
	}
}

pub struct MiniMap {
	/// Center position.
	pub pos: Position,
	/// Player position.
	pub self_pos: Position,
	/// Celestial positions.
	pub cele_views: Vec<CelestialView>,
}

impl Render for MiniMap {
	/// Render the minimap.
	fn render(&self, piet_ctx: &mut WebRenderContext) {
		let map_x = self.pos.x as f64;
		let map_y = self.pos.y as f64;
		let len = 75.0;
		let shape = Rect::new(map_x - len, map_y - len, map_x + len, map_y + len);
		let brush = piet_ctx.solid_brush(Color::grey(0.8));
		piet_ctx.fill(&shape, &brush);
		let brush = piet_ctx.solid_brush(Color::grey(0.3));
		piet_ctx.stroke(&shape, &brush, 3.0);

		let shape = Circle::new(
			(
				map_x - len + (self.self_pos.x / MAP_WIDTH) as f64 * 2.0 * len,
				map_y - len + (self.self_pos.y / MAP_HEIGHT) as f64 * 2.0 * len,
			),
			2.0,
		);
		let brush = piet_ctx.solid_brush(Color::BLACK);
		piet_ctx.fill(&shape, &brush);

		for cele_view in self.cele_views.iter() {
			let shape = Circle::new(
				(
					map_x - len + (cele_view.pos.x / MAP_WIDTH) as f64 * 2.0 * len,
					map_y - len + (cele_view.pos.y / MAP_HEIGHT) as f64 * 2.0 * len,
				),
				4.0,
			);
			let brush = piet_ctx.solid_brush(Color::grey(0.5));
			piet_ctx.fill(&shape, &brush);
		}
	}
}

trait Interpolate: Sized {
	type Output;

	/// Interpolate between `self` and `other` with an interval of `t`.
	fn interp_with(&self, other: &Self, t: f32) -> Self::Output;
}

impl Interpolate for Position {
	type Output = Position;

	fn interp_with(&self, other: &Self, t: f32) -> Self {
		Position { x: (1.0 - t) * self.x + t * other.x, y: (1.0 - t) * self.y + t * other.y }
	}
}

impl Interpolate for PlayerState {
	type Output = PlayerState;

	fn interp_with(&self, other: &PlayerState, t: f32) -> PlayerState {
		PlayerState {
			name: other.name.clone(),
			pos: self.pos.interp_with(&other.pos, t),
			ori: other.ori,
			shield_pos: self.shield_pos.interp_with(&other.shield_pos, t),
		}
	}
}

impl Interpolate for StaticView {
	type Output = StaticView;

	fn interp_with(&self, other: &StaticView, t: f32) -> StaticView {
		StaticView { pos: self.pos.interp_with(&other.pos, t) }
	}
}

impl Interpolate for CelestialView {
	type Output = CelestialView;

	fn interp_with(&self, other: &CelestialView, t: f32) -> CelestialView {
		CelestialView { pos: self.pos.interp_with(&other.pos, t) }
	}
}

impl Interpolate for RenderState {
	type Output = FinalView;

	fn interp_with(&self, other: &RenderState, t: f32) -> FinalView {
		fn interp_items<T: Interpolate<Output = T> + Clone>(
			prev: &HashMap<u64, T>,
			next: &HashMap<u64, T>,
			t: f32,
		) -> Vec<T> {
			prev.iter()
				.map(|(id, prev_elem)| match next.get(id) {
					Some(next_elem) => prev_elem.interp_with(&next_elem, t),
					None => prev_elem.clone(),
				})
				.collect()
		}

		let self_pos = self.self_pos.interp_with(&other.self_pos, t);
		let cele_views = interp_items(&self.celestial_pos, &other.celestial_pos, t);

		// This one contains absolute positions.
		FinalView {
			self_abs_pos: self_pos,
			offset: Position::default(),
			self_pos,
			players: interp_items(&self.players, &other.players, t),
			static_pos: interp_items(&self.static_pos, &other.static_pos, t),
			celestial_pos: cele_views.clone(),
			map: MiniMap { pos: self_pos, self_pos, cele_views },
		}
	}
}

/// Interpolate between `prev` and `next` to prepare for rendering.
pub struct Interpolator {
	/// Initial timestamp set at the beginning.
	base_time: f64,
	/// Previous frame.
	prev: RenderState,
	/// Next frame.
	next: RenderState,
}

impl Interpolator {
	pub fn new(now: f64, prev: RenderState, next: RenderState) -> Self {
		Interpolator { base_time: prev.time.as_millis() as f64 - now, prev, next }
	}

	/// Interpolate based on `time` and compute offsets based on the size of `canvas`.
	pub fn interpolate(&self, time: f64, canvas: &web_sys::HtmlCanvasElement) -> FinalView {
		let t = (self.base_time + time - self.prev.time.as_millis() as f64) as f32
			/ (self.next.time - self.prev.time).as_millis() as f32;
		let mut view = self.prev.interp_with(&self.next, t);

		// Compute positions relative to the canvas (centered at the player's position) before rendering.
		view.map.pos =
			Position { x: canvas.width() as f32 - 100.0, y: canvas.height() as f32 - 100.0 };
		view.celestial_pos.iter().filter(|cele_view| {
			(view.self_pos.x - cele_view.pos.x).abs() < VIEW_X
				&& (view.self_pos.y - cele_view.pos.y).abs() < VIEW_Y
		});

		// Compute the offsets between relative and absolute positions.
		let offset_x = view.self_pos.x - canvas.width() as f32 / 2.0;
		let offset_y = view.self_pos.y - canvas.height() as f32 / 2.0;

		view.offset = Position { x: offset_x, y: offset_y };

		for celestial in view.celestial_pos.iter_mut() {
			celestial.pos.x -= offset_x;
			celestial.pos.y -= offset_y;
		}
		for body in view.static_pos.iter_mut() {
			body.pos.x -= offset_x;
			body.pos.y -= offset_y;
		}
		for player in view.players.iter_mut() {
			player.pos.x -= offset_x;
			player.pos.y -= offset_y;
			player.shield_pos.x -= offset_x;
			player.shield_pos.y -= offset_y;
		}
		view
	}

	pub fn update(&mut self, now: f64, next: RenderState) {
		self.prev = std::mem::replace(&mut self.next, next);
		self.base_time = self.prev.time.as_millis() as f64 - now;
	}
}
