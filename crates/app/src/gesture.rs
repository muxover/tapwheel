//! What the keyboard and mouse hooks decide, apart from Windows so it can be tested anywhere.
//! The hook thread feeds every event in and acts on the answer; nothing here may block.

pub const KEY_SHIFT: usize = 0;
pub const KEY_CTRL: usize = 1;
pub const KEY_ALT: usize = 2;

/// A press and release within this counts as a tap.
const TAP_MS: u32 = 400;
/// Input Windows saw this long before the hooks did means they were removed.
const MISSED_MS: i32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
	Modifier(usize),
	Escape,
	Windows,
	Tab,
	Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mouse {
	LeftDown,
	Move,
	LeftUp,
	OtherDown,
	Wheel(i16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
	/// A trigger key went down during a drag: start looking at what the drag carries.
	Press(usize),
	Tap(usize),
	/// A press that turned out not to be a tap (held, or part of a shortcut).
	NoTap(usize),
	Escape,
	Page {
		back: bool,
	},
	DragEnd,
}

/// What the app tells the hooks.
#[derive(Clone, Copy, Debug, Default)]
pub struct Context {
	pub wheel_open: bool,
	pub paused: bool,
	/// The wheel has seen files on this drag.
	pub files_seen: bool,
}

#[derive(Default)]
pub struct Tracker {
	/// The trigger key down now, and when it went down.
	held: Option<(usize, u32)>,
	/// Another key or button joined in, so the held key is a shortcut, not a tap.
	dirty: bool,
	pressed: [bool; 3],
	swallowed: [bool; 3],
	down_at: Option<(i32, i32)>,
	dragging: bool,
	/// The drag started on Explorer or the desktop, where Alt, Ctrl and Shift change the drop.
	pub shell_drag: bool,
	drag_size: (i32, i32),
}

impl Tracker {
	/// `drag_size` is how far the pointer moves before Windows calls it a drag.
	pub fn new(drag_size: (i32, i32)) -> Tracker {
		Tracker {
			drag_size,
			..Tracker::default()
		}
	}

	pub fn dragging(&self) -> bool {
		self.dragging
	}

	/// Whether Windows should drop this key, and what to tell the app.
	pub fn key(&mut self, key: Key, down: bool, time: u32, cx: Context) -> (bool, Option<Event>) {
		match key {
			Key::Modifier(m) if down => {
				if matches!(self.held, Some((h, _)) if h == m) {
					// held down: the key repeats
					return (self.swallowed[m], None);
				}
				self.held = Some((m, time));
				self.dirty = false;
				let drag = self.dragging && !cx.paused;
				// Explorer reads these keys as link, copy and move while it drags files
				self.swallowed[m] = drag && (self.shell_drag || cx.files_seen);
				self.pressed[m] = drag;
				(self.swallowed[m], drag.then_some(Event::Press(m)))
			}
			Key::Modifier(m) => {
				let quick = time.wrapping_sub(self.held.map_or(time, |(_, t)| t)) <= TAP_MS;
				let clean = matches!(self.held, Some((h, _)) if h == m) && !self.dirty;
				self.held = None;
				let swallow = std::mem::take(&mut self.swallowed[m]);
				let pressed = std::mem::take(&mut self.pressed[m]);
				let live = (self.dragging || cx.wheel_open) && !cx.paused;
				let event = if quick && clean && live {
					Some(Event::Tap(m))
				} else {
					pressed.then_some(Event::NoTap(m))
				};
				(swallow, event)
			}
			_ if down => {
				self.dirty = true;
				let event = match key {
					// Esc, or the Windows key (Start is about to cover the wheel), closes it
					Key::Escape | Key::Windows if cx.wheel_open => Some(Event::Escape),
					// Tab turns the wheel's page while a drag holds the mouse
					Key::Tab if cx.wheel_open => Some(Event::Page {
						back: matches!(self.held, Some((KEY_SHIFT, _))),
					}),
					_ => None,
				};
				(false, event)
			}
			_ => (false, None),
		}
	}

	pub fn mouse(&mut self, m: Mouse, x: i32, y: i32, cx: Context) -> Option<Event> {
		match m {
			Mouse::LeftDown => {
				self.down_at = Some((x, y));
				self.dragging = false;
				self.shell_drag = false;
				None
			}
			Mouse::Move => {
				if let Some((x0, y0)) = self.down_at
					&& !self.dragging
					&& ((x - x0).abs() > self.drag_size.0 || (y - y0).abs() > self.drag_size.1)
				{
					self.dragging = true;
				}
				None
			}
			Mouse::LeftUp => {
				self.down_at = None;
				std::mem::take(&mut self.dragging).then_some(Event::DragEnd)
			}
			Mouse::OtherDown => {
				self.dirty = true;
				None
			}
			Mouse::Wheel(delta) => cx.wheel_open.then_some(Event::Page { back: delta > 0 }),
		}
	}
}

/// True when Windows saw input after the hooks last did, by more than a moment: Windows removes
/// a hook that once answered too slowly, without saying so.
pub fn hooks_missed(last_input: u32, last_seen: u32) -> bool {
	(last_input.wrapping_sub(last_seen) as i32) > MISSED_MS
}

#[cfg(test)]
mod tests {
	use super::*;

	const ALT: Key = Key::Modifier(KEY_ALT);
	const SHIFT: Key = Key::Modifier(KEY_SHIFT);

	fn idle() -> Context {
		Context::default()
	}

	fn drag(t: &mut Tracker) {
		t.mouse(Mouse::LeftDown, 100, 100, idle());
		t.mouse(Mouse::Move, 120, 100, idle());
		assert!(t.dragging());
	}

	#[test]
	fn a_tap_during_a_drag_from_explorer_is_kept_from_explorer() {
		let mut t = Tracker::new((4, 4));
		drag(&mut t);
		t.shell_drag = true;
		assert_eq!(t.key(ALT, true, 1000, idle()), (true, Some(Event::Press(KEY_ALT))));
		// the key repeats while held
		assert_eq!(t.key(ALT, true, 1030, idle()), (true, None));
		assert_eq!(t.key(ALT, false, 1090, idle()), (true, Some(Event::Tap(KEY_ALT))));
	}

	#[test]
	fn outside_a_drag_every_key_goes_on() {
		let mut t = Tracker::new((4, 4));
		assert_eq!(t.key(ALT, true, 1000, idle()), (false, None));
		assert_eq!(t.key(ALT, false, 1090, idle()), (false, None));
		// a click without moving isn't a drag either
		t.mouse(Mouse::LeftDown, 100, 100, idle());
		t.mouse(Mouse::Move, 102, 101, idle());
		t.shell_drag = true;
		assert_eq!(t.key(ALT, true, 2000, idle()), (false, None));
		assert_eq!(t.key(ALT, false, 2050, idle()), (false, None));
	}

	#[test]
	fn other_apps_keep_their_keys_until_the_wheel_sees_files() {
		let mut t = Tracker::new((4, 4));
		drag(&mut t);
		assert_eq!(t.key(SHIFT, true, 1000, idle()), (false, Some(Event::Press(KEY_SHIFT))));
		assert_eq!(t.key(SHIFT, false, 1050, idle()), (false, Some(Event::Tap(KEY_SHIFT))));
		let seen = Context {
			files_seen: true,
			wheel_open: true,
			..idle()
		};
		assert_eq!(t.key(ALT, true, 2000, seen), (true, Some(Event::Press(KEY_ALT))));
		assert_eq!(t.key(ALT, false, 2050, seen), (true, Some(Event::Tap(KEY_ALT))));
	}

	#[test]
	fn a_held_key_or_a_shortcut_is_no_tap() {
		let mut t = Tracker::new((4, 4));
		drag(&mut t);
		t.shell_drag = true;
		t.key(ALT, true, 1000, idle());
		assert_eq!(t.key(ALT, false, 1600, idle()), (true, Some(Event::NoTap(KEY_ALT))));
		t.key(ALT, true, 2000, idle());
		t.key(Key::Other, true, 2020, idle());
		assert_eq!(t.key(ALT, false, 2060, idle()), (true, Some(Event::NoTap(KEY_ALT))));
		t.key(ALT, true, 3000, idle());
		t.mouse(Mouse::OtherDown, 120, 100, idle());
		assert_eq!(t.key(ALT, false, 3050, idle()), (true, Some(Event::NoTap(KEY_ALT))));
	}

	#[test]
	fn a_key_swallowed_going_down_is_swallowed_coming_up_after_the_drop() {
		let mut t = Tracker::new((4, 4));
		drag(&mut t);
		t.shell_drag = true;
		t.key(ALT, true, 1000, idle());
		assert_eq!(t.mouse(Mouse::LeftUp, 120, 100, idle()), Some(Event::DragEnd));
		assert_eq!(t.key(ALT, false, 1050, idle()), (true, Some(Event::NoTap(KEY_ALT))));
		// and the next one, with no drag, goes on
		assert_eq!(t.key(ALT, true, 2000, idle()), (false, None));
		assert_eq!(t.key(ALT, false, 2050, idle()), (false, None));
	}

	#[test]
	fn pause_lets_everything_through() {
		let mut t = Tracker::new((4, 4));
		drag(&mut t);
		t.shell_drag = true;
		let ctrl = Key::Modifier(KEY_CTRL);
		assert_eq!(t.key(ctrl, true, 500, idle()), (true, Some(Event::Press(KEY_CTRL))));
		assert_eq!(t.key(ctrl, false, 550, idle()), (true, Some(Event::Tap(KEY_CTRL))));
		let paused = Context { paused: true, ..idle() };
		assert_eq!(t.key(ALT, true, 1000, paused), (false, None));
		assert_eq!(t.key(ALT, false, 1050, paused), (false, None));
	}

	#[test]
	fn an_open_wheel_hears_taps_tab_esc_and_the_mouse_wheel() {
		let mut t = Tracker::new((4, 4));
		let open = Context {
			wheel_open: true,
			..idle()
		};
		assert_eq!(t.key(ALT, true, 1000, open), (false, None));
		assert_eq!(t.key(ALT, false, 1050, open), (false, Some(Event::Tap(KEY_ALT))));
		t.key(SHIFT, true, 2000, open);
		assert_eq!(
			t.key(Key::Tab, true, 2010, open),
			(false, Some(Event::Page { back: true }))
		);
		assert_eq!(t.key(Key::Escape, true, 2100, open), (false, Some(Event::Escape)));
		assert_eq!(t.key(Key::Windows, true, 2200, open), (false, Some(Event::Escape)));
		assert_eq!(
			t.mouse(Mouse::Wheel(-120), 0, 0, open),
			Some(Event::Page { back: false })
		);
		assert_eq!(t.mouse(Mouse::Wheel(120), 0, 0, idle()), None);
	}

	#[test]
	fn a_new_click_forgets_where_the_last_drag_started() {
		let mut t = Tracker::new((4, 4));
		drag(&mut t);
		t.shell_drag = true;
		t.mouse(Mouse::LeftUp, 120, 100, idle());
		t.mouse(Mouse::LeftDown, 300, 300, idle());
		assert!(!t.shell_drag);
	}

	#[test]
	fn timer_wrap_doesnt_break_taps() {
		let mut t = Tracker::new((4, 4));
		let open = Context {
			wheel_open: true,
			..idle()
		};
		t.key(ALT, true, u32::MAX - 20, open);
		assert_eq!(t.key(ALT, false, 30, open), (false, Some(Event::Tap(KEY_ALT))));
	}

	#[test]
	fn missed_input_means_removed_hooks() {
		assert!(!hooks_missed(5000, 5000));
		assert!(!hooks_missed(5000, 4500));
		// the hooks saw something after Windows' last input reading
		assert!(!hooks_missed(5000, 5200));
		assert!(hooks_missed(7000, 5000));
		assert!(hooks_missed(1500, u32::MAX - 100));
	}
}
