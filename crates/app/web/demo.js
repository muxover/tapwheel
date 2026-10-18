// A looping replay of using TapWheel, drawn with the same geometry, icons, colours and timings
// as crates/ui/src/wheel.rs. Used by the settings home page and by the website.

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
const OPEN_MS = 250;
const CLOSE_MS = 167;
const PICK_MS = 250;
const HOVER_MS = 83;
const RING_MS = 167;
const CARD_IN_MS = 250;
const CARD_OUT_MS = 167;

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
// Windows' decelerate curve, cubic-bezier(0, 0, 0, 1), in closed form
const easeOut = (t) => {
	const s = Math.cbrt(clamp(t));
	return s * s * (3 - 2 * s);
};
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

function petal(cx, cy, r0, r1, a0, a1, corner, fill) {
	const mid = (r0 + r1) / 2;
	const pad = Math.min(corner / mid, (a1 - a0) / 2 - 0.001);
	const d = sector(cx, cy, r0 + corner, r1 - corner, a0 + pad, a1 - pad);
	if (!d) return "";
	// opaque inside a faded group, so the rounding stroke doesn't double the alpha at the edges
	const solid = css([fill[0], fill[1], fill[2], 1]);
	return `<g opacity="${fill[3].toFixed(3)}"><path d="${d}" fill="${solid}" stroke="${solid}" stroke-width="${corner * 2}" stroke-linejoin="round"/></g>`;
}

/** A slice whose sides stay `d` from each divider at every radius, rounded by `corner`. */
function slice(cx, cy, r0, r1, a0, a1, d, corner, fill) {
	const ri = r0 + corner;
	const ro = r1 - corner;
	const dd = d + corner;
	if (ro <= ri) return "";
	const off = (r) => Math.asin(Math.max(-1, Math.min(1, dd / r)));
	const [o0, o1, i0, i1] = [a0 + off(ro), a1 - off(ro), a0 + off(ri), a1 - off(ri)];
	if (o1 <= o0 || i1 <= i0) return "";
	const p = (r, a) => `${(cx + Math.cos(a) * r).toFixed(2)} ${(cy + Math.sin(a) * r).toFixed(2)}`;
	const path = `M${p(ro, o0)}A${ro} ${ro} 0 ${o1 - o0 > Math.PI ? 1 : 0} 1 ${p(ro, o1)}L${p(ri, i1)}A${ri} ${ri} 0 ${i1 - i0 > Math.PI ? 1 : 0} 0 ${p(ri, i0)}Z`;
	const solid = css([fill[0], fill[1], fill[2], 1]);
	return `<g opacity="${fill[3].toFixed(3)}"><path d="${path}" fill="${solid}" stroke="${solid}" stroke-width="${corner * 2}" stroke-linejoin="round"/></g>`;
}

function glyph(name, x, y, size, fill) {
	const d = icons[name];
	if (!d) return "";
	return `<path d="${d}" transform="translate(${x.toFixed(2)} ${y.toFixed(2)}) scale(${size / 20})" fill="${css(fill)}"/>`;
}

function text(str, x, y, size, weight, fill, anchor = "middle", extra = "") {
	return `<text x="${x.toFixed(2)}" y="${y.toFixed(2)}" text-anchor="${anchor}" dominant-baseline="central" font-family="${esc(P.font)}" font-size="${size}" font-weight="${weight}" fill="${css(fill)}" ${extra}>${esc(str)}</text>`;
}

// the page's colour tokens, read once per frame so the drawing needs no inline styles
let P = null;
const probe = document.createElement("canvas").getContext("2d");
function rgba(value) {
	probe.fillStyle = "#000";
	probe.fillStyle = value.trim() || "#000";
	const v = probe.fillStyle;
	if (v[0] === "#") return [parseInt(v.slice(1, 3), 16), parseInt(v.slice(3, 5), 16), parseInt(v.slice(5, 7), 16), 1];
	const n = v.match(/[\d.]+/g).map(Number);
	return [n[0], n[1], n[2], n[3] ?? 1];
}
function palette(el) {
	const cs = getComputedStyle(el);
	const get = (name) => rgba(cs.getPropertyValue(name));
	return {
		text: get("--text"),
		text2: get("--text-2"),
		text3: get("--text-3"),
		accent: get("--accent"),
		onAccent: get("--on-accent"),
		surface: get("--surface"),
		raised: get("--surface-2"),
		stroke: get("--stroke"),
		strong: get("--stroke-strong"),
		font: cs.getPropertyValue("--font").trim() || "Segoe UI, sans-serif",
	};
}
const css = ([r, g, b, a]) => `rgba(${Math.round(r)},${Math.round(g)},${Math.round(b)},${+a.toFixed(3)})`;
const mix = (a, b, t) => a.map((v, i) => v + (b[i] - v) * clamp(t));
const alpha = (c, k) => [c[0], c[1], c[2], c[3] * k];
const NONE = [0, 0, 0, 0];
const shape = (fill, stroke = NONE, width = 1) => `fill="${css(fill)}" stroke="${css(stroke)}" stroke-width="${width}"`;

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

// same fitting as the app's wheel (crates/ui/src/wheel.rs): one line, else two, sizes stepping down
const LABEL_PAD = 8;
const measure = document.createElement("canvas").getContext("2d");
const widthOf = (t, size) => {
	measure.font = `600 ${size}px ${P.font}`;
	return measure.measureText(t).width;
};
const flipped = (mid) => {
	const s = Math.sin(mid);
	return s > 1e-3 || (Math.abs(s) <= 1e-3 && Math.cos(mid) < 0);
};
function fitLabel(label, hasIcon, span, mid) {
	const flip = flipped(mid);
	const sizes = hasIcon ? [12.5, 12, 11.5, 11] : [16, 15, 14, 13];
	const [iconR, textR] = hasIcon ? (flip ? [104, 140] : [140, 104]) : [null, (R_IN + R_OUT) / 2];
	const room = (r) => Math.min(span, Math.PI) * r - 2 * LABEL_PAD;
	for (const size of sizes) if (widthOf(label, size) <= room(textR)) return { lines: [[label, textR]], size, flip, iconR };
	const words = label.split(" ");
	for (const size of sizes) {
		const gap = size * 1.33 * 0.9;
		const [r1, r2] = flip ? [textR - gap / 2, textR + gap / 2] : [textR + gap / 2, textR - gap / 2];
		let best = null;
		for (let k = 1; k < words.length; k++) {
			const a = words.slice(0, k).join(" ");
			const b = words.slice(k).join(" ");
			const w = Math.max(widthOf(a, size), widthOf(b, size));
			if (!best || w < best.w) best = { a, b, w };
		}
		if (best && widthOf(best.a, size) <= room(r1) && widthOf(best.b, size) <= room(r2)) return { lines: [[best.a, r1], [best.b, r2]], size, flip, iconR };
	}
	return { lines: [[label, textR]], size: sizes[sizes.length - 1], flip, iconR };
}

let arcs = 0;
/** Text on an arc, with its letters' middle on the circle; flipped runs the other way. */
function arcText(str, cx, cy, r, mid, flip, size, fill) {
	const cap = size * 0.7;
	const base = flip ? r + cap / 2 : r - cap / 2;
	const reach = 1.5;
	const pt = (a) => `${(cx + Math.cos(a) * base).toFixed(2)} ${(cy + Math.sin(a) * base).toFixed(2)}`;
	const id = `tw-arc-${arcs++}`;
	const d = flip ? `M${pt(mid + reach)}A${base} ${base} 0 0 0 ${pt(mid - reach)}` : `M${pt(mid - reach)}A${base} ${base} 0 0 1 ${pt(mid + reach)}`;
	return `<defs><path id="${id}" d="${d}"/></defs><text font-family="${esc(P.font)}" font-size="${size}" font-weight="600" fill="${css(fill)}"><textPath href="#${id}" startOffset="50%" text-anchor="middle">${esc(str)}</textPath></text>`;
}

/** The wheel at one moment. `m` follows the real Motion: open, close, hover per slice, ring, picked. */
function wheel(scene, m, win11) {
	const n = scene.slices.length + (scene.more ? 1 : 0);
	const corner = win11 ? 8 : 2;
	const tile = alpha(P.text, 0.05);
	let out = "";
	out += `<circle cx="${CX}" cy="${CY}" r="${R_OUT}" ${shape(P.surface, P.strong)}/>`;
	for (let k = 0; k < n; k++) {
		const [a0, mid, a1] = angles(n, k);
		const isMore = scene.more && k === n - 1;
		const hv = m.hover[k] || 0;
		const picked = m.picked === k;
		const fade = m.picked !== null && !picked ? 1 - easeOut(m.close * 2) : 1;
		// hover only colours a slice, as the app does; a pick pops it out
		const pop = picked ? 8 * Math.sin(m.close * Math.PI) : 0;
		const ox = Math.cos(mid) * pop;
		const oy = Math.sin(mid) * pop;
		const ga = 2 / R_OUT;
		out += `<g opacity="${fade.toFixed(3)}">`;
		out += slice(CX + ox, CY + oy, R_IN + 6, R_OUT - 6, a0, a1, 2, corner, mix(tile, P.accent, hv));
		const ink = mix(P.text, P.onAccent, hv);
		const label = isMore ? "More" : scene.slices[k].label;
		const ic = isMore ? "more_horizontal" : scene.slices[k].icon || "";
		const fit = fitLabel(label, !!ic, a1 - a0 - 2 * ga, mid);
		const pcx = CX + ox;
		const pcy = CY + oy;
		if (fit.iconR) out += glyph(ic, pcx + Math.cos(mid) * fit.iconR - 12, pcy + Math.sin(mid) * fit.iconR - 12, 24, ink);
		for (const [line, r] of fit.lines) out += arcText(line, pcx, pcy, r, mid, fit.flip, fit.size, ink);
		if (isMore) {
			// page dots under the label as it reads
			const last = fit.lines[fit.lines.length - 1][1];
			const r = fit.flip ? last + 14 : last - 14;
			[0, 1].forEach((d) => {
				const off = (d - 0.5) * (9 / r);
				const a = fit.flip ? mid - off : mid + off;
				out += `<circle cx="${(pcx + Math.cos(a) * r).toFixed(2)}" cy="${(pcy + Math.sin(a) * r).toFixed(2)}" r="2.6" fill="${css(d === 0 ? ink : alpha(ink, 0.35))}"/>`;
			});
		}
		out += "</g>";
	}

	// ring of choices around the hovered slice
	const slice = m.slice !== null ? scene.slices[m.slice] : null;
	if (slice?.ring && m.picked === null && m.ring > 0) {
		const grow = easeBack(m.ring);
		const r0 = R_OUT + (RING_IN - R_OUT) * grow;
		const r1 = R_OUT + (RING_OUT - R_OUT) * grow;
		const shown = easeOut(m.ring * 1.6);
		const rc = win11 ? 8 : 2;
		const r = (RING_IN + RING_OUT) / 2;
		const gap = 6 / r;
		const widths = slice.ring.map((l) => Math.max(widthOf(l, 12.5) + 30, 64) / r);
		const total = widths.reduce((a, b) => a + b, 0) + gap * (widths.length - 1);
		let a = angles(n, m.slice)[1] - total / 2;
		out += `<g opacity="${shown.toFixed(3)}">`;
		slice.ring.forEach((l, j) => {
			const s0 = a;
			const s1 = a + widths[j];
			a = s1 + gap;
			const on = m.ringOn === j;
			out += petal(CX, CY + 3, r0, r1, s0, s1, rc, [0, 0, 0, 0.14]);
			out += petal(CX, CY, r0, r1, s0, s1, rc, on ? P.accent : P.raised);
			const md = (s0 + s1) / 2;
			const rr = (r0 + r1) / 2;
			out += arcText(l, CX, CY, rr, md, flipped(md), 12.5, on ? P.onAccent : P.text);
		});
		out += "</g>";
	}

	// centre
	out += `<circle cx="${CX}" cy="${CY}" r="${R_CENTER}" ${shape(P.raised, P.strong)}/>`;
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
	out += text(head, CX, y, 15, 600, P.text);
	y += 20;
	for (const l of lines) {
		out += text(l, CX, y, 12, 400, P.text2);
		y += 16;
	}

	// the keys pill
	const parts = WHEELS.map(([k, name], i) => ({ k, name, kw: k.length * 7 + 12, nw: name.length * 6.9, on: i === scene.key }));
	const inner = parts.reduce((s, p) => s + p.kw + 6 + p.nw, 0) + 14 * (parts.length - 1);
	const pw = inner + 20;
	const px = CX - pw / 2;
	const pr = win11 ? PILL_H / 2 : 0;
	out += `<rect x="${px}" y="${PILL_Y}" width="${pw}" height="${PILL_H}" rx="${pr}" ${shape(P.surface, P.strong)}/>`;
	let x = px + 10;
	for (const p of parts) {
		const ky = PILL_Y + 10;
		const kr = win11 ? 4 : 0;
		out += p.on
			? `<rect x="${x}" y="${ky}" width="${p.kw}" height="20" rx="${kr}" ${shape(P.accent)}/>`
			: `<rect x="${x + 0.5}" y="${ky + 0.5}" width="${p.kw - 1}" height="19" rx="${kr}" ${shape(NONE, P.text3)}/>`;
		out += text(p.k, x + p.kw / 2, ky + 10, 11, 600, p.on ? P.onAccent : P.text2);
		x += p.kw + 6;
		out += text(p.name, x, PILL_Y + PILL_H / 2, 12, p.on ? 600 : 400, P.text, "start");
		x += p.nw + 14;
	}

	// opening scales up with a little overshoot; closing shrinks and fades
	let scale = 1;
	let seen = 1;
	if (m.close > 0) {
		const t = easeOut(m.close);
		scale = m.picked !== null ? 1 - 0.06 * t : 1 - 0.1 * t;
		seen = 1 - t;
	} else if (m.open < 1) {
		scale = 0.86 + 0.14 * easeBack(m.open);
		seen = easeOut(m.open * 1.4);
	}
	return `<g transform="translate(${CX} ${CY}) scale(${scale.toFixed(4)}) translate(${-CX} ${-CY})" opacity="${clamp(seen).toFixed(3)}" filter="url(#tw-demo-shadow)">${out}</g>`;
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
	let s = `<g transform="translate(${((1 - t) * 36).toFixed(2)} 0) translate(${x + cw / 2} ${y + ch / 2}) scale(${(0.96 + 0.04 * t).toFixed(4)}) translate(${-(x + cw / 2)} ${-(y + ch / 2)})" opacity="${t.toFixed(3)}" filter="url(#tw-demo-shadow)">`;
	s += `<rect x="${x}" y="${y}" width="${cw}" height="${ch}" rx="${r}" ${shape(P.surface, P.strong)}/>`;
	const tx = x + 14;
	const ty = y + 14;
	s += `<rect x="${tx}" y="${ty}" width="36" height="36" rx="${win11 ? 8 : 0}" ${shape(done ? alpha(P.accent, 0.16) : alpha(P.text, 0.07))}/>`;
	s += glyph(c.icon || "checkmark_circle", tx + 8, ty + 8, 20, done ? P.accent : P.text);
	if (done) {
		const bx = tx + 32;
		const by = ty + 32;
		s += `<circle cx="${bx}" cy="${by}" r="8" ${shape(P.surface)}/><circle cx="${bx}" cy="${by}" r="6.5" ${shape(P.accent)}/>`;
		s += `<path d="M${bx - 3} ${by}l2 2 4-4" ${shape(NONE, P.onAccent, 1.6)} stroke-linecap="round" stroke-linejoin="round"/>`;
	}
	const detail = done ? c.detail : `${Math.round(progress * 100)}%  ·  ${c.detail}`;
	s += text(c.title, x + 62, y + 24, 13.5, 600, P.text, "start");
	s += text(detail, x + 62, y + 42, 12, 400, P.text2, "start");
	if (!done) {
		const bw = cw - 28;
		s += `<rect x="${x + 14}" y="${y + ch - 7}" width="${bw}" height="3" rx="1.5" ${shape(alpha(P.text2, 0.18))}/>`;
		s += `<rect x="${x + 14}" y="${y + ch - 7}" width="${(bw * clamp(progress)).toFixed(2)}" height="3" rx="1.5" ${shape(P.accent)}/>`;
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
	let s = `<g filter="url(#tw-demo-shadow)"><rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${r}" ${shape(P.surface, P.strong)}/></g>`;
	s += `<rect x="${x}" y="${y}" width="${w}" height="40" rx="${r}" ${shape(P.raised)}/><rect x="${x}" y="${y + 30}" width="${w}" height="10" ${shape(P.raised)}/>`;
	s += glyph("folder", x + 14, y + 10, 20, [232, 177, 48, 1]);
	s += text(scene.file.kind === "video" ? "Videos" : "Pictures", x + 42, y + 20, 13, 600, P.text, "start");
	s += `<line x1="${x}" y1="${y + 40}" x2="${x + w}" y2="${y + 40}" ${shape(NONE, P.stroke)}/>`;
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
		if (me && pressed) s += `<rect x="${ix - 8}" y="${iy - 8}" width="96" height="112" rx="${win11 ? 6 : 0}" ${shape(alpha(P.accent, 0.18), alpha(P.accent, 0.55))}/>`;
		s += `<g opacity="${me && lifted ? 0.45 : 1}">`;
		if (f.doc) {
			s += `<rect x="${ix + 10}" y="${iy}" width="60" height="70" rx="4" ${shape(P.raised, P.strong)}/>`;
			s += glyph(f.doc === true ? "document_text" : f.doc, ix + 26, iy + 21, 28, P.text2);
		} else {
			s += thumb(f, ix, iy + 6, 80);
		}
		s += text(f.name, ix + 40, iy + 88, 12, 400, P.text);
		s += "</g>";
	});
	return s;
}

function cursor(x, y, down) {
	const k = down ? 0.94 : 1;
	return `<g transform="translate(${x.toFixed(2)} ${y.toFixed(2)}) scale(${1.25 * k})"><path d="M0 0V16.5l4.2-4 2.9 6.6 2.6-1.1-2.8-6.4H12.8Z" fill="#fff" stroke="#111" stroke-width="1" stroke-linejoin="round"/></g>`;
}

/** The dragged file under the pointer, the way Windows shows it. */
function ghost(file, x, y, shown) {
	if (shown <= 0) return "";
	return `<g transform="translate(${(x + 14).toFixed(2)} ${(y + 22).toFixed(2)})" opacity="${(0.7 * shown).toFixed(3)}">${thumb(file, 0, 0, 44)}</g>`;
}

/** A keycap that dips when it's tapped. */
function keycap(label, x, y, press, shown) {
	if (shown <= 0) return "";
	const w = label.length * 10 + 34;
	const dip = Math.sin(clamp(press) * Math.PI) * 4;
	const t = easeOut(shown);
	return `<g opacity="${t.toFixed(3)}" transform="translate(${x - w / 2} ${(y + (1 - t) * 10).toFixed(2)})"><rect x="0" y="4" width="${w}" height="44" rx="8" ${shape(mix(P.surface, P.text, 0.22))}/><rect x="0" y="${dip.toFixed(2)}" width="${w}" height="44" rx="8" ${shape(P.surface, P.strong)}/>${text(label, w / 2, 22 + dip, 16, 600, P.text)}</g>`;
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
	const widths = slice.ring.map((l) => Math.max(widthOf(l, 12.5) + 30, 64) / r);
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



/** Pauses while off screen or hidden; with reduced motion it shows a still of each scene's choice. */
export function demo({ scenes = SCENES, labels = true, scene = 0, at = null } = {}) {
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
	// the look comes from the nearest themed container, or the page
	const win11 = () => (root.closest("[data-tw-scope]") || document.documentElement).hasAttribute("data-win11");

	function draw(now) {
		const s = scenes[index];
		// a still shows the moment just before the drop
		const ms = at ?? (still ? (s.slices[s.pick].ring ? T.ringAt + 100 : T.sliceAt + 100) : now - t0);
		P = palette(root);
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
