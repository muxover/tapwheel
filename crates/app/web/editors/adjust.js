// Adjust: light, colour and contrast, previewed live. The same pixel maths runs on the
// preview and on the full picture, so what you see is what you get.

import { apply, fit, h, keepImageExt, loadImage, outName, prepared, saveCanvas, savesAs, shell, slider, stageLoading, group } from "../tw.js";

const SLIDERS = [
	["exposure", "Exposure", -2, 2, 0.05, (v) => `${v > 0 ? "+" : ""}${v.toFixed(2)} EV`],
	["contrast", "Contrast", -100, 100, 1],
	["highlights", "Highlights", -100, 100, 1],
	["shadows", "Shadows", -100, 100, 1],
	["saturation", "Saturation", -100, 100, 1],
	["vibrance", "Vibrance", -100, 100, 1],
	["warmth", "Warmth", -100, 100, 1],
	["tint", "Tint", -100, 100, 1],
	["hue", "Hue", -180, 180, 1, (v) => `${v}°`],
	["gamma", "Gamma", 0.4, 2.5, 0.01, (v) => v.toFixed(2)],
	["vignette", "Vignette", 0, 100, 1],
];

const ZERO = Object.fromEntries(SLIDERS.map(([k, , , , , ]) => [k, k === "gamma" ? 1 : 0]));

/** Applies the adjustments to RGBA pixels in place. */
export function adjust(data, w, h, a) {
	const ev = 2 ** a.exposure;
	const con = 1 + a.contrast / 100;
	const sat = 1 + a.saturation / 100;
	const vib = a.vibrance / 100;
	const warm = a.warmth / 100;
	const tint = a.tint / 100;
	const hl = a.highlights / 100;
	const sh = a.shadows / 100;
	const inv = 1 / a.gamma;
	const rad = (a.hue * Math.PI) / 180;
	const cosA = Math.cos(rad);
	const sinA = Math.sin(rad);
	// hue rotation matrix around the grey axis
	const k = (1 - cosA) / 3;
	const sq = Math.sqrt(1 / 3) * sinA;
	const m = [cosA + k, k - sq, k + sq, k + sq, cosA + k, k - sq, k - sq, k + sq, cosA + k];
	const lut = new Float32Array(256);
	for (let i = 0; i < 256; i++) lut[i] = i / 255;
	const vig = a.vignette / 100;
	const cx = w / 2;
	const cy = h / 2;
	const maxd = Math.hypot(cx, cy);
	for (let y = 0; y < h; y++) {
		for (let x = 0; x < w; x++) {
			const i = (y * w + x) * 4;
			let r = lut[data[i]] * ev;
			let g = lut[data[i + 1]] * ev;
			let b = lut[data[i + 2]] * ev;
			r += warm * 0.08;
			b -= warm * 0.08;
			g -= tint * 0.06;
			if (a.hue) {
				const nr = r * m[0] + g * m[1] + b * m[2];
				const ng = r * m[3] + g * m[4] + b * m[5];
				const nb = r * m[6] + g * m[7] + b * m[8];
				r = nr;
				g = ng;
				b = nb;
			}
			let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
			// lift or pull shadows and highlights with smooth weights
			const ws = Math.max(0, 1 - l * 2) ** 2;
			const wh = Math.max(0, l * 2 - 1) ** 2;
			const lift = sh * ws * 0.35 - hl * wh * 0.35;
			r += lift;
			g += lift;
			b += lift;
			l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
			r = (r - 0.5) * con + 0.5;
			g = (g - 0.5) * con + 0.5;
			b = (b - 0.5) * con + 0.5;
			l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
			const mx = Math.max(r, g, b);
			const mn = Math.min(r, g, b);
			const s = sat + vib * (1 - (mx - mn));
			r = l + (r - l) * s;
			g = l + (g - l) * s;
			b = l + (b - l) * s;
			if (inv !== 1) {
				r = r > 0 ? r ** inv : 0;
				g = g > 0 ? g ** inv : 0;
				b = b > 0 ? b ** inv : 0;
			}
			if (vig) {
				const d = Math.hypot(x - cx, y - cy) / maxd;
				const f = 1 - vig * 0.7 * d * d;
				r *= f;
				g *= f;
				b *= f;
			}
			data[i] = r * 255;
			data[i + 1] = g * 255;
			data[i + 2] = b * 255;
		}
	}
}

export default async function adjustEditor(init) {
	const file = init.files[0];
	const st = { ...ZERO };
	const canvas = h("canvas");
	const sliders = SLIDERS.map(([key, label, min, max, step, format]) =>
		slider({ label, min, max, step, value: st[key], format: format || ((v) => `${v > 0 ? "+" : ""}${v}`), oninput: (v) => set(key, v) }),
	);
	const compare = h("button", { class: "btn", type: "button" }, "Hold to compare");
	const ui = shell({
		title: "Adjust",
		lead: "Double-click a slider to reset it.",
		panel: [...sliders, group(null, h("div", { class: "row" }, compare, h("button", { class: "btn subtle", type: "button", onclick: resetAll }, "Reset all")))],
		onSave: async () => {
			const out = h("canvas", { width: img.naturalWidth, height: img.naturalHeight });
			const ctx = out.getContext("2d", { willReadFrequently: true });
			ctx.drawImage(img, 0, 0);
			const d = ctx.getImageData(0, 0, out.width, out.height);
			adjust(d.data, out.width, out.height, st);
			ctx.putImageData(d, 0, 0);
			apply({ do: "rendered", file: await saveCanvas(out), suffix: "adjusted", to: null });
		},
	});
	ui.setOut(savesAs(outName(file, "adjusted", keepImageExt(file.ext))));
	stageLoading(ui.stage);
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const img = await loadImage(info.url);
	ui.stage.replaceChildren(h("div", { class: "canvas-wrap" }, canvas));
	const k = Math.min(1, 1400 / Math.max(img.naturalWidth, img.naturalHeight));
	canvas.width = Math.round(img.naturalWidth * k);
	canvas.height = Math.round(img.naturalHeight * k);
	const ctx = canvas.getContext("2d", { willReadFrequently: true });
	ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
	const base = ctx.getImageData(0, 0, canvas.width, canvas.height);

	let queued = false;
	function set(key, v) {
		st[key] = v;
		if (!queued) {
			queued = true;
			requestAnimationFrame(() => {
				queued = false;
				draw();
			});
		}
	}
	function draw(original = false) {
		const d = new ImageData(new Uint8ClampedArray(base.data), base.width, base.height);
		if (!original) adjust(d.data, d.width, d.height, st);
		ctx.putImageData(d, 0, 0);
	}
	function resetAll() {
		Object.assign(st, ZERO);
		SLIDERS.forEach(([key], i) => sliders[i].set(st[key]));
		draw();
	}
	compare.addEventListener("pointerdown", () => draw(true));
	compare.addEventListener("pointerup", () => draw());
	compare.addEventListener("pointerleave", () => draw());
	const layout = () => {
		const s = fit(ui.stage, canvas.width, canvas.height);
		canvas.style.width = `${canvas.width * s}px`;
		canvas.style.height = `${canvas.height * s}px`;
	};
	addEventListener("resize", layout);
	layout();
	draw();
}
