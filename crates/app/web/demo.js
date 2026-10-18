// A looping, faithful replay of using TapWheel: a file is dragged, a key is tapped, the wheel
// opens, a slice and a ring choice are picked, and the result card arrives. The wheel is drawn
// with the same geometry, icons, colours and timings as the real one (crates/ui/src/wheel.rs),
// so the page shows what the app does rather than a drawing of it. Used by the settings home
// page and by the website.

import { icons } from "./icons.js";

const NS = "http://www.w3.org/2000/svg";
const W = 1100;
const H = 580;

// wheel geometry, in the real window's pixels
const CX = 700;
const CY = 266;
const R_OUT = 172;
const R_IN = 70;
const R_CENTER = 62;
const RING_IN = R_OUT + 8;
const RING_OUT = R_OUT + 58;
const PILL_Y = CY + 254;
const PILL_H = 40;

// the real animation lengths, in milliseconds
const OPEN_MS = 190;
const CLOSE_MS = 140;
const PICK_MS = 230;
const HOVER_MS = 90;
const RING_MS = 150;
const CARD_IN_MS = 220;
const CARD_OUT_MS = 160;

const WHEELS = [
	["Shift", "Convert"],
	["Ctrl", "Tools"],
	["Alt", "Actions"],
];

/** What each loop shows: real slices, labels and ring choices for those files. */
export const SCENES = [
	{
		key: 0,
		file: { name: "photo.heic", kind: "image", tint: ["#ffc178", "#e8506f"] },
		slices: ["PNG", "JPG", "WebP", "AVIF", "GIF", "TIFF", "BMP"].map((label) => ({ label, desc: `Convert to ${label}` })),
		more: true,
		pick: 1,
		running: { title: "Convert to JPG", detail: "photo.heic", icon: "" },
		done: { title: "photo.jpg", detail: "2.4 MB to 1.1 MB, 54% smaller", icon: "" },
		work: 900,
	},
	{
		key: 1,
		file: { name: "photo.jpg", kind: "image", tint: ["#7cc8ff", "#4b5bdc"] },
		slices: [
			{ label: "Crop", icon: "crop", desc: "Cut to a size or shape" },
			{ label: "Add BG", icon: "image_add", desc: "Put it on a backdrop" },
			{ label: "Remove BG", icon: "image_sparkle", desc: "Cut out the subject" },
			{ label: "Compress", icon: "arrow_minimize", desc: "Make it smaller", ring: ["Balanced", "Smaller", "Best", "Custom"] },
			{ label: "Resize", icon: "resize", desc: "Change the size" },
			{ label: "Adjust", icon: "options", desc: "Light, colour and contrast" },
			{ label: "Annotate", icon: "pen", desc: "Draw, point and write" },
		],
		more: true,
		pick: 3,
		ringPick: 0,
		running: { title: "Make it smaller", detail: "photo.jpg", icon: "arrow_minimize" },
		done: { title: "photo-small.jpg", detail: "4.2 MB to 860 KB, 80% smaller", icon: "arrow_minimize" },
		work: 1100,
	},
	{
		key: 2,
		file: { name: "clip.mp4", kind: "video", tint: ["#9be15d", "#00a3a3"] },
		slices: [
			{ label: "Copy", icon: "copy", desc: "Copy to paste elsewhere" },
			{ label: "Copy path", icon: "link", desc: "Copy the full path" },
			{ label: "Move to", icon: "folder_arrow_right", desc: "Move to a folder", ring: ["Desktop", "Projects", "Choose"] },
			{ label: "Share", icon: "share", desc: "Send with Windows share" },
			{ label: "Float", icon: "pin", desc: "Keep it handy for later" },
			{ label: "Checksum", icon: "fingerprint", desc: "Copy a checksum" },
		],
		more: false,
		pick: 2,
		ringPick: 1,
		running: null,
		done: { title: "Moved to Projects", detail: "clip.mp4", icon: "folder_arrow_right" },
		work: 0,
	},
];

const clamp = (t) => Math.max(0, Math.min(1, t));
const easeOut = (t) => 1 - (1 - clamp(t)) ** 3;
const easeBack = (t) => {
	t = clamp(t);
	const c = 1.4;
	return 1 + (c + 1) * (t - 1) ** 3 + c * (t - 1) ** 2;
};
const easeInOut = (t) => {
	t = clamp(t);
	return t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2;
};
const lerp = (a, b, t) => a + (b - a) * t;
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

function angles(n, i) {
	const step = (Math.PI * 2) / n;
	const mid = -Math.PI / 2 + step * i;
	return [mid - step / 2, mid, mid + step / 2];
}

/** An annular sector; stroked with a round join of twice the corner it gets the wheel's soft corners. */
function sector(cx, cy, r0, r1, a0, a1) {
	if (r1 <= r0 || a1 <= a0) return "";
	const big = a1 - a0 > Math.PI ? 1 : 0;
	const p = (r, a) => `${(cx + Math.cos(a) * r).toFixed(2)} ${(cy + Math.sin(a) * r).toFixed(2)}`;
	return `M${p(r1, a0)}A${r1} ${r1} 0 ${big} 1 ${p(r1, a1)}L${p(r0, a1)}A${r0} ${r0} 0 ${big} 0 ${p(r0, a0)}Z`;
}

function petal(cx, cy, r0, r1, a0, a1, corner, fill, extra = "") {
	const mid = (r0 + r1) / 2;
	const pad = Math.min(corner / mid, (a1 - a0) / 2 - 0.001);
	const d = sector(cx, cy, r0 + corner, r1 - corner, a0 + pad, a1 - pad);
	if (!d) return "";
	return `<path d="${d}" style="fill:${fill};stroke:${fill};stroke-width:${corner * 2};stroke-linejoin:round" ${extra}/>`;
}

function glyph(name, x, y, size, fill) {
	const d = icons[name];
	if (!d) return "";
	return `<path d="${d}" transform="translate(${x.toFixed(2)} ${y.toFixed(2)}) scale(${size / 20})" style="fill:${fill}"/>`;
}

function text(str, x, y, size, weight, fill, anchor = "middle", extra = "") {
	return `<text x="${x.toFixed(2)}" y="${y.toFixed(2)}" text-anchor="${anchor}" dominant-baseline="central" style="font:${weight} ${size}px var(--font);fill:${fill}" ${extra}>${esc(str)}</text>`;
}

const mix = (a, b, t) => (t <= 0 ? a : t >= 1 ? b : `color-mix(in srgb, ${b} ${(t * 100).toFixed(1)}%, ${a})`);

/** Picks words into at most `lines` lines that fit `max` pixels, roughly measured. */
function wrap(str, size, max, lines) {
	const per = Math.max(4, Math.floor(max / (size * 0.52)));
	const out = [];
	let line = "";
	for (const word of str.split(" ")) {
		const next = line ? `${line} ${word}` : word;
		if (next.length > per && line) {
			out.push(line);
			line = word;
		} else line = next;
	}
	if (line) out.push(line);
	return out.slice(0, lines);
}

/** The wheel at one moment. `m` follows the real Motion: open, close, hover per slice, ring, picked. */
function wheel(scene, m, win11) {
	const n = scene.slices.length + (scene.more ? 1 : 0);
	const corner = win11 ? 10 : 2;
	const tile = "color-mix(in srgb, var(--text) 5%, transparent)";
	let out = "";
	out += `<circle cx="${CX}" cy="${CY}" r="${R_OUT}" class="disc"/>`;
	for (let k = 0; k < n; k++) {
		const [a0, mid, a1] = angles(n, k);
		const isMore = scene.more && k === n - 1;
		const hv = m.hover[k] || 0;
		const picked = m.picked === k;
		const fade = m.picked !== null && !picked ? 1 - easeOut(m.close * 2) : 1;
		const pop = hv * 5 + (picked ? 8 * Math.sin(m.close * Math.PI) : 0);
		const ox = Math.cos(mid) * pop;
		const oy = Math.sin(mid) * pop;
		const ga = 3 / R_OUT;
		out += `<g style="opacity:${fade.toFixed(3)}">`;
		out += petal(CX + ox, CY + oy, R_IN + 6, R_OUT - 6, a0 + ga, a1 - ga, corner, mix(tile, "var(--accent)", hv));
		const rMid = (R_IN + R_OUT) / 2 + 4;
		const lx = CX + Math.cos(mid) * rMid + ox;
		const ly = CY + Math.sin(mid) * rMid + oy;
		const ink = mix("var(--text)", "var(--on-accent)", hv);
		const label = isMore ? "More" : scene.slices[k].label;
		const ic = isMore ? "more_horizontal" : scene.slices[k].icon || "";
		if (ic) {
			out += glyph(ic, lx - 12, ly - 22, 24, ink);
			out += text(label, lx, ly + 13, 12.5, 600, ink);
		} else {
			out += text(label, lx, ly, 16, 600, ink);
		}
		if (isMore) {
			out += `<circle cx="${lx - 4.5}" cy="${ly + 27}" r="2.6" style="fill:${ink}"/><circle cx="${lx + 4.5}" cy="${ly + 27}" r="2.6" style="fill:${ink};opacity:.35"/>`;
		}
		out += "</g>";
	}

	// ring of choices around the hovered slice
	const slice = m.slice !== null ? scene.slices[m.slice] : null;
	if (slice?.ring && m.picked === null && m.ring > 0) {
		const grow = easeBack(m.ring);
		const r0 = R_OUT + (RING_IN - R_OUT) * grow;
		const r1 = R_OUT + (RING_OUT - R_OUT) * grow;
		const alpha = easeOut(m.ring * 1.6);
		const rc = win11 ? 12 : 2;
		const r = (RING_IN + RING_OUT) / 2;
		const gap = 6 / r;
		const widths = slice.ring.map((l) => Math.max(l.length * 7.2 + 30, 64) / r);
		const total = widths.reduce((a, b) => a + b, 0) + gap * (widths.length - 1);
		let a = angles(n, m.slice)[1] - total / 2;
		out += `<g style="opacity:${alpha.toFixed(3)}">`;
		slice.ring.forEach((l, j) => {
			const s0 = a;
			const s1 = a + widths[j];
			a = s1 + gap;
			const on = m.ringOn === j;
			out += petal(CX, CY + 3, r0, r1, s0, s1, rc, "rgba(0,0,0,.14)");
			out += petal(CX, CY, r0, r1, s0, s1, rc, on ? "var(--accent)" : "var(--raised)");
			if (!on) out += `<path d="${sector(CX, CY, r0 + rc / 2, r1 - rc / 2, s0 + 0.005, s1 - 0.005)}" style="fill:none;stroke:var(--stroke-strong);stroke-width:1"/>`;
			const md = (s0 + s1) / 2;
			const rr = (r0 + r1) / 2;
			out += text(l, CX + Math.cos(md) * rr, CY + Math.sin(md) * rr, 12.5, 600, on ? "var(--on-accent)" : "var(--text)");
		});
		out += "</g>";
	}

	// centre
	out += `<circle cx="${CX}" cy="${CY}" r="${R_CENTER}" style="fill:var(--raised);stroke:var(--stroke-strong);stroke-width:1"/>`;
	let head = WHEELS[scene.key][1];
	let body = scene.file.name;
	if (slice) {
		if (m.ringOn !== null && slice.ring) {
			head = slice.ring[m.ringOn];
			body = slice.label;
		} else {
			head = slice.label;
			body = slice.desc;
		}
	}
	const lines = wrap(body, 12, 104, 3);
	const total = 20 + lines.length * 16;
	let y = CY - total / 2 + 9;
	out += text(head, CX, y, 15, 600, "var(--text)");
	y += 20;
	for (const l of lines) {
		out += text(l, CX, y, 12, 400, "var(--text-2)");
		y += 16;
	}

	// the keys pill
	const parts = WHEELS.map(([k, name], i) => ({ k, name, kw: k.length * 7 + 12, nw: name.length * 6.9, on: i === scene.key }));
	const inner = parts.reduce((s, p) => s + p.kw + 6 + p.nw, 0) + 14 * (parts.length - 1);
	const pw = inner + 20;
	const px = CX - pw / 2;
	const pr = win11 ? PILL_H / 2 : 0;
	out += `<rect x="${px}" y="${PILL_Y}" width="${pw}" height="${PILL_H}" rx="${pr}" class="pill"/>`;
	let x = px + 10;
	for (const p of parts) {
		const ky = PILL_Y + 10;
		const kr = win11 ? 4 : 0;
		out += p.on
			? `<rect x="${x}" y="${ky}" width="${p.kw}" height="20" rx="${kr}" style="fill:var(--accent)"/>`
			: `<rect x="${x + 0.5}" y="${ky + 0.5}" width="${p.kw - 1}" height="19" rx="${kr}" style="fill:none;stroke:var(--text-3)"/>`;
		out += text(p.k, x + p.kw / 2, ky + 10, 11, 600, p.on ? "var(--on-accent)" : "var(--text-2)");
		x += p.kw + 6;
		out += text(p.name, x, PILL_Y + PILL_H / 2, 12, p.on ? 600 : 400, "var(--text)", "start");
		x += p.nw + 14;
	}

	// opening scales up with a little overshoot; closing shrinks and fades
	let scale = 1;
	let alpha = 1;
	if (m.close > 0) {
		const t = easeOut(m.close);
		scale = m.picked !== null ? 1 - 0.06 * t : 1 - 0.1 * t;
		alpha = 1 - t;
	} else if (m.open < 1) {
		scale = 0.86 + 0.14 * easeBack(m.open);
		alpha = easeOut(m.open * 1.4);
	}
	return `<g transform="translate(${CX} ${CY}) scale(${scale.toFixed(4)}) translate(${-CX} ${-CY})" style="opacity:${clamp(alpha).toFixed(3)}" filter="url(#tw-demo-shadow)">${out}</g>`;
}

/** A result card as the real one draws it: tile, title, detail, progress on the bottom edge. */
function card(c, shown, progress, win11, time) {
	if (shown <= 0) return "";
	const t = easeOut(shown);
	const cw = 360;
	const ch = 64;
	const x = W - cw - 28;
	const y = H - ch - 28;
	const r = win11 ? 8 : 0;
	const done = progress === null;
	let s = `<g transform="translate(${((1 - t) * 36).toFixed(2)} 0) translate(${x + cw / 2} ${y + ch / 2}) scale(${(0.96 + 0.04 * t).toFixed(4)}) translate(${-(x + cw / 2)} ${-(y + ch / 2)})" style="opacity:${t.toFixed(3)}" filter="url(#tw-demo-shadow)">`;
	s += `<rect x="${x}" y="${y}" width="${cw}" height="${ch}" rx="${r}" class="card"/>`;
	const tx = x + 14;
	const ty = y + 14;
	s += `<rect x="${tx}" y="${ty}" width="36" height="36" rx="${win11 ? 8 : 0}" style="fill:${done ? "color-mix(in srgb, var(--accent) 16%, transparent)" : "color-mix(in srgb, var(--text) 7%, transparent)"}"/>`;
	s += glyph(c.icon || "checkmark_circle", tx + 8, ty + 8, 20, done ? "var(--accent)" : "var(--text)");
	if (done) {
		const bx = tx + 32;
		const by = ty + 32;
		s += `<circle cx="${bx}" cy="${by}" r="8" style="fill:var(--surface)"/><circle cx="${bx}" cy="${by}" r="6.5" style="fill:var(--accent)"/>`;
		s += `<path d="M${bx - 3} ${by}l2 2 4-4" style="fill:none;stroke:var(--on-accent);stroke-width:1.6;stroke-linecap:round;stroke-linejoin:round"/>`;
	}
	const detail = done ? c.detail : `${Math.round(progress * 100)}%  ·  ${c.detail}`;
	s += text(c.title, x + 62, y + 24, 13.5, 600, "var(--text)", "start");
	s += text(detail, x + 62, y + 42, 12, 400, "var(--text-2)", "start");
	if (!done) {
		const bw = cw - 28;
		s += `<rect x="${x + 14}" y="${y + ch - 7}" width="${bw}" height="3" rx="1.5" style="fill:color-mix(in srgb, var(--text-2) 18%, transparent)"/>`;
		s += `<rect x="${x + 14}" y="${y + ch - 7}" width="${(bw * clamp(progress)).toFixed(2)}" height="3" rx="1.5" style="fill:var(--accent)"/>`;
	}
	void time;
	return `${s}</g>`;
}

function thumb(file, x, y, size) {
	const id = `g-${file.name.replace(/\W/g, "")}`;
	let s = `<defs><linearGradient id="${id}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${file.tint[0]}"/><stop offset="1" stop-color="${file.tint[1]}"/></linearGradient></defs>`;
	s += `<rect x="${x}" y="${y}" width="${size}" height="${size * 0.78}" rx="4" fill="url(#${id})"/>`;
	if (file.kind === "video") {
		const cx = x + size / 2;
		const cy = y + size * 0.39;
		s += `<circle cx="${cx}" cy="${cy}" r="${size * 0.16}" fill="rgba(0,0,0,.35)"/><path d="M${cx - size * 0.05} ${cy - size * 0.08}l${size * 0.13} ${size * 0.08}-${size * 0.13} ${size * 0.08}z" fill="#fff"/>`;
	} else {
		// a sun and hills, so it reads as a photo
		s += `<circle cx="${x + size * 0.7}" cy="${y + size * 0.24}" r="${size * 0.09}" fill="rgba(255,255,255,.85)"/>`;
		s += `<path d="M${x} ${y + size * 0.78}L${x + size * 0.34} ${y + size * 0.4}L${x + size * 0.58} ${y + size * 0.62}L${x + size * 0.74} ${y + size * 0.48}L${x + size} ${y + size * 0.78}Z" fill="rgba(255,255,255,.45)"/>`;
	}
	return s;
}

/** A small Explorer window holding the files; the scene's file is the one picked up. */
function explorer(scene, pressed, lifted, win11) {
	const x = 36;
	const y = 70;
	const w = 340;
	const h = 330;
	const r = win11 ? 8 : 0;
	let s = `<g filter="url(#tw-demo-shadow)"><rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${r}" class="card"/></g>`;
	s += `<rect x="${x}" y="${y}" width="${w}" height="40" rx="${r}" style="fill:var(--surface-2)"/><rect x="${x}" y="${y + 30}" width="${w}" height="10" style="fill:var(--surface-2)"/>`;
	s += glyph("folder", x + 14, y + 10, 20, "#e8b130");
	s += text(scene.file.kind === "video" ? "Videos" : "Pictures", x + 42, y + 20, 13, 600, "var(--text)", "start");
	s += `<line x1="${x}" y1="${y + 40}" x2="${x + w}" y2="${y + 40}" style="stroke:var(--stroke)"/>`;
	const others = [
		{ name: "notes.txt", doc: true },
		{ name: "trip.png", kind: "image", tint: ["#ffd76b", "#ff7a45"] },
		{ name: "song.mp3", doc: "music_note_2" },
		{ name: "logo.svg", kind: "image", tint: ["#c49bff", "#6a4cff"] },
		{ name: "album.zip", doc: "folder_zip" },
	];
	const items = [scene.file, ...others];
	items.forEach((f, i) => {
		const col = i % 3;
		const row = Math.floor(i / 3);
		const ix = x + 22 + col * 104;
		const iy = y + 60 + row * 128;
		const me = i === 0;
		if (me && pressed) s += `<rect x="${ix - 8}" y="${iy - 8}" width="96" height="112" rx="${win11 ? 6 : 0}" style="fill:color-mix(in srgb, var(--accent) 18%, transparent);stroke:color-mix(in srgb, var(--accent) 55%, transparent)"/>`;
		s += `<g style="opacity:${me && lifted ? 0.45 : 1}">`;
		if (f.doc) {
			s += `<rect x="${ix + 10}" y="${iy}" width="60" height="70" rx="4" style="fill:var(--surface-2);stroke:var(--stroke-strong)"/>`;
			s += glyph(f.doc === true ? "document_text" : f.doc, ix + 26, iy + 21, 28, "var(--text-2)");
		} else {
			s += thumb(f, ix, iy + 6, 80);
		}
		s += text(f.name, ix + 40, iy + 88, 12, 400, "var(--text)");
		s += "</g>";
	});
	return s;
}

function cursor(x, y, down) {
	const k = down ? 0.94 : 1;
	return `<g transform="translate(${x.toFixed(2)} ${y.toFixed(2)}) scale(${1.25 * k})"><path d="M0 0V16.5l4.2-4 2.9 6.6 2.6-1.1-2.8-6.4H12.8Z" style="fill:#fff;stroke:#111;stroke-width:1;stroke-linejoin:round"/></g>`;
}

/** The dragged file under the pointer, the way Windows shows it. */
function ghost(file, x, y, alpha) {
	if (alpha <= 0) return "";
	return `<g transform="translate(${(x + 14).toFixed(2)} ${(y + 22).toFixed(2)})" style="opacity:${(0.7 * alpha).toFixed(3)}">${thumb(file, 0, 0, 44)}</g>`;
}

/** A keycap that dips when it's tapped. */
function keycap(label, x, y, press, shown) {
	if (shown <= 0) return "";
	const w = label.length * 10 + 34;
	const dip = Math.sin(clamp(press) * Math.PI) * 4;
	const t = easeOut(shown);
	return `<g style="opacity:${t.toFixed(3)}" transform="translate(${x - w / 2} ${(y + (1 - t) * 10).toFixed(2)})"><rect x="0" y="4" width="${w}" height="44" rx="8" style="fill:color-mix(in srgb, var(--text) 22%, var(--surface))"/><rect x="0" y="${dip.toFixed(2)}" width="${w}" height="44" rx="8" class="key"/>${text(label, w / 2, 22 + dip, 16, 600, "var(--text)")}</g>`;
}

/** Times in the loop, in milliseconds. */
const T = {
	press: 500,
	liftEnd: 1500, // cursor arrives near the wheel's spot
	key: 1350,
	open: 1600,
	toSlice: 1900,
	sliceAt: 2550,
	toRing: 3050,
	ringAt: 3450,
	drop: 3900,
};

function pointOnPetal(scene, k, r) {
	const n = scene.slices.length + (scene.more ? 1 : 0);
	const [, mid] = angles(n, k);
	return [CX + Math.cos(mid) * r, CY + Math.sin(mid) * r];
}

function ringPoint(scene, k, j) {
	const n = scene.slices.length + (scene.more ? 1 : 0);
	const slice = scene.slices[k];
	const r = (RING_IN + RING_OUT) / 2;
	const gap = 6 / r;
	const widths = slice.ring.map((l) => Math.max(l.length * 7.2 + 30, 64) / r);
	const total = widths.reduce((a, b) => a + b, 0) + gap * (widths.length - 1);
	let a = angles(n, k)[1] - total / 2;
	for (let i = 0; i < j; i++) a += widths[i] + gap;
	const md = a + widths[j] / 2;
	return [CX + Math.cos(md) * r, CY + Math.sin(md) * r];
}

/** Everything on screen `ms` into a scene's loop. */
function frame(scene, ms, win11) {
	const hasRing = !!scene.slices[scene.pick].ring;
	const drop = hasRing ? T.drop : T.sliceAt + 600;
	const workEnd = drop + PICK_MS + 120 + scene.work;
	const cardOut = workEnd + 2100;
	const start = [36 + 22 + 40, 70 + 60 + 40];
	const centre = [CX - 6, CY + 70];
	const slice = pointOnPetal(scene, scene.pick, (R_IN + R_OUT) / 2 + 6);
	const ring = hasRing ? ringPoint(scene, scene.pick, scene.ringPick) : slice;

	// the pointer's path
	let p = start;
	if (ms >= T.press) p = lerpP(start, centre, easeInOut((ms - T.press) / (T.liftEnd - T.press)));
	if (ms >= T.toSlice) p = lerpP(centre, slice, easeInOut((ms - T.toSlice) / (T.sliceAt - T.toSlice)));
	if (hasRing && ms >= T.toRing) p = lerpP(slice, ring, easeInOut((ms - T.toRing) / (T.ringAt - T.toRing)));
	const afterDrop = ms >= drop;
	if (ms > drop + 500) p = lerpP(ring, [ring[0] + 40, ring[1] + 160], easeInOut((ms - drop - 500) / 900));

	// wheel motion, as the real one counts it
	const m = { open: 0, close: 0, hover: [], ring: 0, picked: null, slice: null, ringOn: null };
	const open = ms >= T.open && ms < drop + PICK_MS + 20;
	m.open = clamp((ms - T.open) / OPEN_MS);
	const hovering = ms >= T.sliceAt - 120;
	if (hovering) {
		m.slice = scene.pick;
		m.hover[scene.pick] = clamp((ms - (T.sliceAt - 120)) / HOVER_MS);
	}
	if (hasRing && ms >= T.sliceAt + 60) m.ring = clamp((ms - T.sliceAt - 60) / RING_MS);
	if (hasRing && ms >= T.ringAt - 80) m.ringOn = scene.ringPick;
	if (afterDrop) {
		m.picked = scene.pick;
		m.close = clamp((ms - drop) / PICK_MS);
	}

	const scene_ = [];
	scene_.push(explorer(scene, ms >= T.press - 80 && ms < drop, ms >= T.press && ms < drop, win11));
	if (open) scene_.push(wheel(scene, m, win11));
	const keyShown = ms >= T.key - 250 && ms < T.open + 700 ? clamp((ms - (T.key - 250)) / 180) * (1 - clamp((ms - T.open - 500) / 200)) : 0;
	scene_.push(keycap(WHEELS[scene.key][0], 206, 448, (ms - T.key) / 220, keyShown));

	// the card: running while work happens, then done
	const cIn = drop + PICK_MS + 40;
	if (ms >= cIn) {
		let shown = clamp((ms - cIn) / CARD_IN_MS);
		if (ms >= cardOut) shown = 1 - clamp((ms - cardOut) / CARD_OUT_MS);
		const running = scene.running && ms < workEnd;
		const progress = running ? easeInOut((ms - cIn) / (workEnd - cIn)) : null;
		scene_.push(card(running ? scene.running : scene.done, shown, progress, win11, ms));
	}

	const dragging = ms >= T.press + 60 && !afterDrop;
	scene_.push(ghost(scene.file, p[0], p[1], dragging ? 1 : 0));
	const pressed = ms >= T.press && !afterDrop;
	scene_.push(cursor(p[0], p[1], pressed));
	return { svg: scene_.join(""), end: cardOut + CARD_OUT_MS + 500 };
}

function lerpP(a, b, t) {
	return [lerp(a[0], b[0], t), lerp(a[1], b[1], t)];
}

const CSS = `
.tw-demo{position:relative;border-radius:var(--radius);overflow:hidden;border:1px solid var(--stroke);background:
 radial-gradient(120% 90% at 85% 10%, color-mix(in srgb, var(--accent) 26%, transparent), transparent 60%),
 radial-gradient(90% 80% at 0% 100%, color-mix(in srgb, #b6f23a 16%, transparent), transparent 60%), var(--stage)}
.tw-demo svg{display:block;width:100%;height:auto}
.tw-demo{--raised:var(--surface-2)}
.tw-demo .disc{fill:var(--surface);stroke:var(--stroke-strong);stroke-width:1}
.tw-demo .pill,.tw-demo .card,.tw-demo .key{fill:var(--surface);stroke:var(--stroke-strong);stroke-width:1}
.tw-demo .steps{position:absolute;left:16px;top:14px;display:flex;gap:6px}
.tw-demo .steps button{font:600 12px var(--font);color:var(--text-2);background:color-mix(in srgb, var(--surface) 70%, transparent);border:1px solid var(--stroke);border-radius:999px;padding:4px 10px;cursor:pointer}
.tw-demo .steps button[aria-pressed=true]{background:var(--accent);color:var(--on-accent);border-color:transparent}
`;

/**
 * The demo element. Pauses while off screen or hidden; with reduced motion it shows a still of
 * each scene's choice instead of moving.
 */
export function demo({ scenes = SCENES, labels = true, scene = 0, at = null } = {}) {
	if (!document.getElementById("tw-demo-css")) {
		const st = document.createElement("style");
		st.id = "tw-demo-css";
		st.textContent = CSS;
		document.head.append(st);
	}
	const root = document.createElement("div");
	root.className = "tw-demo";
	root.setAttribute("role", "img");
	const svg = document.createElementNS(NS, "svg");
	svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
	svg.innerHTML = `<defs><filter id="tw-demo-shadow" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="6" stdDeviation="9" flood-color="#000" flood-opacity=".22"/></filter></defs><g class="layer"></g>`;
	const layer = svg.querySelector(".layer");
	root.append(svg);

	let index = scene;
	let t0 = performance.now();
	const describe = () => {
		const s = scenes[index];
		const pick = s.slices[s.pick];
		const choice = pick.ring ? `, then ${pick.ring[s.ringPick]}` : "";
		root.setAttribute("aria-label", `Dragging ${s.file.name}, tapping ${WHEELS[s.key][0]} to open ${WHEELS[s.key][1]}, and dropping on ${pick.label}${choice}.`);
	};
	const buttons = [];
	if (labels) {
		const steps = document.createElement("div");
		steps.className = "steps";
		scenes.forEach((s, i) => {
			const b = document.createElement("button");
			b.type = "button";
			b.textContent = `${WHEELS[s.key][0]} · ${WHEELS[s.key][1]}`;
			b.addEventListener("click", () => {
				index = i;
				t0 = performance.now();
				describe();
				if (still) draw(performance.now());
			});
			buttons.push(b);
			steps.append(b);
		});
		root.append(steps);
	}
	const still = at !== null || matchMedia("(prefers-reduced-motion: reduce)").matches;
	let visible = true;
	let raf = 0;
	const win11 = () => document.documentElement.hasAttribute("data-win11");

	function draw(now) {
		const s = scenes[index];
		// a still shows the moment just before the drop
		const ms = at ?? (still ? (s.slices[s.pick].ring ? T.ringAt + 100 : T.sliceAt + 100) : now - t0);
		const f = frame(s, ms, win11());
		layer.innerHTML = f.svg;
		buttons.forEach((b, i) => b.setAttribute("aria-pressed", String(i === index)));
		if (!still && ms > f.end) {
			index = (index + 1) % scenes.length;
			t0 = now;
			describe();
		}
	}
	function loop(now) {
		raf = 0;
		if (!root.isConnected) return;
		draw(now);
		if (visible && !document.hidden && !still) raf = requestAnimationFrame(loop);
	}
	const wake = () => {
		if (!raf && visible && !document.hidden && !still) raf = requestAnimationFrame(loop);
	};
	if ("IntersectionObserver" in window) {
		new IntersectionObserver((es) => {
			visible = es.some((e) => e.isIntersecting);
			wake();
		}).observe(root);
	}
	document.addEventListener("visibilitychange", wake);
	describe();
	requestAnimationFrame((now) => {
		t0 = now;
		draw(now);
		wake();
	});
	return root;
}
