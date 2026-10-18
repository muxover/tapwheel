// Backdrops shared by Add background, Code shot and Combine. Same names as the app's own.

export const GRADIENTS = {
	sunset: ["#ff6b6b", "#ffa94d", "#845ef7"],
	ocean: ["#22d3ee", "#6366f1"],
	lime: ["#b6f23a", "#10b981"],
	grape: ["#a855f7", "#ec4899"],
	dusk: ["#1e293b", "#4c1d95"],
	sand: ["#fde68a", "#fb923c"],
	mist: ["#e2e8f0", "#cbd5e1"],
	ink: ["#18181b", "#27272a"],
};

export const SOLIDS = ["#ffffff", "#f5f5f4", "#facc15", "#84cc16", "#14b8a6", "#38bdf8", "#6366f1", "#18181b"];

/** A picture of your own to put behind, chosen with the "Your picture" swatch. */
let own = null;

export function cssFor(name) {
	const g = GRADIENTS[name];
	if (g) return `linear-gradient(135deg, ${g.join(", ")})`;
	if (name === "none") return "repeating-conic-gradient(#ccc 0 25%, #fff 0 50%) 0 0 / 12px 12px";
	if (name === "blur") return "linear-gradient(135deg, #9ca3af, #4b5563)";
	return name;
}

export function paintBackdrop(ctx, name, w, h, source) {
	const g = GRADIENTS[name];
	if (g) {
		const grad = ctx.createLinearGradient(0, 0, w, h);
		g.forEach((c, i) => grad.addColorStop(i / (g.length - 1), c));
		ctx.fillStyle = grad;
		ctx.fillRect(0, 0, w, h);
	} else if (name === "image" && own) {
		// your picture fills the backdrop, cropped to it, never stretched
		const s = Math.max(w / own.naturalWidth, h / own.naturalHeight);
		ctx.drawImage(own, (w - own.naturalWidth * s) / 2, (h - own.naturalHeight * s) / 2, own.naturalWidth * s, own.naturalHeight * s);
	} else if (name === "blur" && source) {
		// the picture itself, blown up and blurred, behind the picture
		const s = Math.max(w / source.width, h / source.height) * 1.15;
		ctx.save();
		ctx.filter = `blur(${Math.round(Math.max(w, h) / 30)}px) saturate(1.2)`;
		ctx.drawImage(source, (w - source.width * s) / 2, (h - source.height * s) / 2, source.width * s, source.height * s);
		ctx.restore();
		ctx.fillStyle = "rgba(0,0,0,0.08)";
		ctx.fillRect(0, 0, w, h);
	} else if (name && name !== "none" && name !== "image") {
		ctx.fillStyle = name;
		ctx.fillRect(0, 0, w, h);
	}
}

export function roundRect(ctx, x, y, w, h, r) {
	r = Math.min(r, w / 2, h / 2);
	ctx.beginPath();
	ctx.moveTo(x + r, y);
	ctx.arcTo(x + w, y, x + w, y + h, r);
	ctx.arcTo(x + w, y + h, x, y + h, r);
	ctx.arcTo(x, y + h, x, y, r);
	ctx.arcTo(x, y, x + w, y, r);
	ctx.closePath();
}

import { h } from "../tw.js";

/** The picker: gradients, a blur of the picture, your own picture, plain colours, any colour, or nothing. */
export function backdropPicker(value, onchange, { blur = true } = {}) {
	const names = [...Object.keys(GRADIENTS), ...(blur ? ["blur"] : []), "none", ...SOLIDS];
	const buttons = [];
	const pick = (n) => {
		for (const b of buttons) b.setAttribute("aria-pressed", String(b.dataset.v === n));
		mine.setAttribute("aria-pressed", String(n === "image"));
		onchange(n);
	};
	for (const n of names) {
		const label = n === "blur" ? "Blur of the picture" : n === "none" ? "No background" : n;
		const b = h("button", { class: "swatch", type: "button", title: label, "aria-label": label, "data-v": n, "aria-pressed": String(n === value), style: { background: cssFor(n) } });
		b.addEventListener("click", () => pick(n));
		buttons.push(b);
	}

	// your own picture, through the Windows file picker
	const file = h("input", { type: "file", accept: "image/*", hidden: true });
	const mine = h("button", { class: "btn wide", type: "button", "aria-pressed": "false" }, "Use your own picture");
	mine.addEventListener("click", () => file.click());
	file.addEventListener("change", () => {
		const f = file.files?.[0];
		if (!f) return;
		const img = new Image();
		img.onload = () => {
			own = img;
			mine.textContent = f.name;
			mine.style.backgroundImage = `linear-gradient(rgba(0,0,0,.45), rgba(0,0,0,.45)), url("${img.src}")`;
			mine.classList.add("has-picture");
			pick("image");
		};
		img.src = URL.createObjectURL(f);
	});

	const colour = colourPicker("#3b82f6", (c) => pick(c));
	return h("div", { class: "backdrops" }, h("div", { class: "swatches" }, buttons), mine, file, colour);
}

/** Any colour, drawn in the panel: a shade square, a hue strip and the hex value. */
export function colourPicker(start, onchange) {
	let [hue, sat, val] = hsv(start);
	const square = h("div", { class: "cp-square", role: "slider", tabindex: "0", "aria-label": "Shade" });
	const dot = h("div", { class: "cp-dot" });
	square.append(dot);
	const strip = h("input", { class: "cp-hue", type: "range", min: 0, max: 359, value: Math.round(hue), "aria-label": "Hue" });
	const chip = h("span", { class: "cp-chip" });
	const hex = h("input", { class: "cp-hex", type: "text", maxlength: 7, spellcheck: "false", "aria-label": "Hex colour" });
	const body = h("div", { class: "cp-body", hidden: true }, square, strip, h("div", { class: "row" }, chip, hex));
	const toggle = h("button", { class: "btn wide cp-toggle", type: "button", "aria-expanded": "false" }, h("span", { class: "cp-chip" }), "Any colour");
	toggle.addEventListener("click", () => {
		const open = body.hidden;
		body.hidden = !open;
		toggle.setAttribute("aria-expanded", String(open));
		if (open) emit();
	});
	const emit = (fromHex = false) => {
		const c = rgbHex(hsvRgb(hue, sat, val));
		square.style.backgroundColor = `hsl(${hue} 100% 50%)`;
		dot.style.left = `${sat * 100}%`;
		dot.style.top = `${(1 - val) * 100}%`;
		dot.style.background = c;
		chip.style.background = c;
		toggle.firstChild.style.background = c;
		if (!fromHex) hex.value = c;
		onchange(c);
	};
	const fromPoint = (e) => {
		const r = square.getBoundingClientRect();
		sat = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
		val = 1 - Math.min(1, Math.max(0, (e.clientY - r.top) / r.height));
		emit();
	};
	square.addEventListener("pointerdown", (e) => {
		square.setPointerCapture(e.pointerId);
		fromPoint(e);
	});
	square.addEventListener("pointermove", (e) => {
		if (square.hasPointerCapture(e.pointerId)) fromPoint(e);
	});
	square.addEventListener("keydown", (e) => {
		const step = e.shiftKey ? 0.1 : 0.02;
		const d = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] }[e.key];
		if (!d) return;
		e.preventDefault();
		sat = Math.min(1, Math.max(0, sat + d[0]));
		val = Math.min(1, Math.max(0, val + d[1]));
		emit();
	});
	strip.addEventListener("input", () => {
		hue = Number(strip.value);
		emit();
	});
	hex.addEventListener("input", () => {
		const v = hex.value.trim();
		if (!/^#?[0-9a-f]{6}$/i.test(v)) return;
		[hue, sat, val] = hsv(v.startsWith("#") ? v : `#${v}`);
		strip.value = Math.round(hue);
		emit(true);
	});
	// draw the starting state without choosing it
	const c = rgbHex(hsvRgb(hue, sat, val));
	square.style.backgroundColor = `hsl(${hue} 100% 50%)`;
	Object.assign(dot.style, { left: `${sat * 100}%`, top: `${(1 - val) * 100}%`, background: c });
	chip.style.background = c;
	toggle.firstChild.style.background = c;
	hex.value = c;
	return h("div", { class: "cp" }, toggle, body);
}

function hsv(hex) {
	const n = parseInt(hex.slice(1), 16);
	const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((x) => x / 255);
	const max = Math.max(r, g, b);
	const d = max - Math.min(r, g, b);
	let hue = 0;
	if (d) hue = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
	return [(hue * 60 + 360) % 360, max ? d / max : 0, max];
}

function hsvRgb(hue, s, v) {
	const f = (n) => {
		const k = (n + hue / 60) % 6;
		return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
	};
	return [f(5), f(3), f(1)].map((x) => Math.round(x * 255));
}

const rgbHex = (rgb) => `#${rgb.map((x) => x.toString(16).padStart(2, "0")).join("")}`;
