#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
	pub r: f32,
	pub g: f32,
	pub b: f32,
	pub a: f32,
}

impl Rgba {
	pub const fn hex(v: u32) -> Rgba {
		Rgba {
			r: ((v >> 16) & 0xff) as f32 / 255.0,
			g: ((v >> 8) & 0xff) as f32 / 255.0,
			b: (v & 0xff) as f32 / 255.0,
			a: 1.0,
		}
	}

	pub fn from_u8(r: u8, g: u8, b: u8) -> Rgba {
		Rgba {
			r: r as f32 / 255.0,
			g: g as f32 / 255.0,
			b: b as f32 / 255.0,
			a: 1.0,
		}
	}

	pub fn alpha(self, a: f32) -> Rgba {
		Rgba { a: self.a * a, ..self }
	}

	pub fn mix(self, other: Rgba, t: f32) -> Rgba {
		Rgba {
			r: self.r + (other.r - self.r) * t,
			g: self.g + (other.g - self.g) * t,
			b: self.b + (other.b - self.b) * t,
			a: self.a + (other.a - self.a) * t,
		}
	}

	fn channel(c: f32) -> f32 {
		if c <= 0.03928 {
			c / 12.92
		} else {
			((c + 0.055) / 1.055).powf(2.4)
		}
	}

	pub fn luminance(self) -> f32 {
		0.2126 * Self::channel(self.r) + 0.7152 * Self::channel(self.g) + 0.0722 * Self::channel(self.b)
	}

	pub fn contrast(self, other: Rgba) -> f32 {
		let (a, b) = (self.luminance(), other.luminance());
		let (hi, lo) = if a > b { (a, b) } else { (b, a) };
		(hi + 0.05) / (lo + 0.05)
	}

	pub fn skia(self) -> tiny_skia::Color {
		tiny_skia::Color::from_rgba(self.r, self.g, self.b, self.a.clamp(0.0, 1.0)).unwrap_or(tiny_skia::Color::BLACK)
	}
}

/// The Windows look the app is drawn in. Built from the system's own settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
	pub dark: bool,
	/// Windows 11 rounds corners and uses layered, softer surfaces; Windows 10 is square and flat.
	pub win11: bool,
	pub accent: Rgba,
	/// The exact shade Windows uses for accent fills in this theme (AccentLight2 on dark,
	/// AccentDark1 on light), when the system shares it.
	pub shade: Option<Rgba>,
	pub high_contrast: bool,
}

pub struct Palette {
	pub surface: Rgba,
	pub raised: Rgba,
	pub stroke: Rgba,
	pub text: Rgba,
	pub text2: Rgba,
	pub accent: Rgba,
	pub on_accent: Rgba,
	pub danger: Rgba,
	pub success: Rgba,
	pub shadow: Rgba,
}

impl Theme {
	pub fn radius(&self) -> f32 {
		if self.win11 { 8.0 } else { 0.0 }
	}

	pub fn palette(&self) -> Palette {
		if self.high_contrast {
			let (bg, fg) = if self.dark {
				(Rgba::hex(0x000000), Rgba::hex(0xffffff))
			} else {
				(Rgba::hex(0xffffff), Rgba::hex(0x000000))
			};
			return Palette {
				surface: bg,
				raised: bg,
				stroke: fg,
				text: fg,
				text2: fg,
				accent: Rgba::hex(0x1aebff),
				on_accent: Rgba::hex(0x000000),
				danger: if self.dark {
					Rgba::hex(0xffff00)
				} else {
					Rgba::hex(0x600000)
				},
				success: fg,
				shadow: Rgba::hex(0).alpha(0.0),
			};
		}
		let accent = self.readable_accent();
		let on_accent = if accent.contrast(Rgba::hex(0x000000)) >= accent.contrast(Rgba::hex(0xffffff)) {
			Rgba::hex(0x000000)
		} else {
			Rgba::hex(0xffffff)
		};
		if self.dark {
			Palette {
				surface: if self.win11 {
					Rgba::hex(0x2b2b2b)
				} else {
					Rgba::hex(0x1f1f1f)
				},
				raised: if self.win11 {
					Rgba::hex(0x383838)
				} else {
					Rgba::hex(0x2b2b2b)
				},
				stroke: Rgba::hex(0xffffff).alpha(0.09),
				text: Rgba::hex(0xffffff),
				text2: Rgba::hex(0xffffff).alpha(0.68),
				accent,
				on_accent,
				danger: Rgba::hex(0xff99a4),
				success: Rgba::hex(0x6ccb5f),
				shadow: Rgba::hex(0x000000).alpha(0.36),
			}
		} else {
			Palette {
				surface: if self.win11 {
					Rgba::hex(0xf9f9f9)
				} else {
					Rgba::hex(0xf2f2f2)
				},
				raised: Rgba::hex(0xffffff),
				stroke: Rgba::hex(0x000000).alpha(0.08),
				text: Rgba::hex(0x1a1a1a),
				text2: Rgba::hex(0x000000).alpha(0.6),
				accent,
				on_accent,
				danger: Rgba::hex(0xc42b1c),
				success: Rgba::hex(0x0f7b0f),
				shadow: Rgba::hex(0x000000).alpha(0.18),
			}
		}
	}

	/// Windows picks a lighter accent shade on dark and a darker one on light; do the same so it stays readable.
	fn readable_accent(&self) -> Rgba {
		let bg = if self.dark {
			Rgba::hex(0x2b2b2b)
		} else {
			Rgba::hex(0xf9f9f9)
		};
		let target = if self.dark {
			Rgba::hex(0xffffff)
		} else {
			Rgba::hex(0x000000)
		};
		// Windows' own AccentLight2 on dark and AccentDark1 on light, or close to them
		let mut c = self.shade.unwrap_or(if self.dark {
			self.accent.mix(Rgba::hex(0xffffff), 0.4)
		} else {
			self.accent.mix(Rgba::hex(0x000000), 0.12)
		});
		for _ in 0..10 {
			if c.contrast(bg) >= 3.0 {
				break;
			}
			c = c.mix(target, 0.15);
		}
		c
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn accent_stays_readable_on_both_themes() {
		for dark in [true, false] {
			for accent in [0x0078d4, 0xffb900, 0x101010, 0xffffff, 0xb6f23a] {
				let t = Theme {
					dark,
					win11: true,
					accent: Rgba::hex(accent),
					shade: None,
					high_contrast: false,
				};
				let p = t.palette();
				assert!(p.accent.contrast(p.surface) >= 3.0, "accent {accent:06x} dark={dark}");
				assert!(
					p.on_accent.contrast(p.accent) >= 4.5,
					"on accent {accent:06x} dark={dark}"
				);
				assert!(p.text.contrast(p.surface) >= 7.0);
				assert!(p.text2.mix(p.surface, 0.0).alpha(1.0).contrast(p.surface) >= 1.0);
			}
		}
	}
}
