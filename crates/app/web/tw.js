import { icon } from "./icons.js";

const TW = { init: null, prepared: null, waiting: [], listeners: {} };
window.TW = TW;

export function send(cmd, data = {}) {
	window.ipc.postMessage(JSON.stringify({ cmd, ...data }));
}

/** The app calls this with every message for the page. */
TW.receive = (msg) => {
	switch (msg.type) {
		case "init":
			TW.init = msg;
			applyTheme(msg.theme);
			document.title = msg.title || "TapWheel";
			start(msg);
			break;
		case "prepared":
			TW.prepared = msg.files;
			for (const cb of TW.waiting.splice(0)) cb(msg.files);
			break;
		case "theme":
			applyTheme(msg.theme);
			break;
		case "error":
			toast(msg.message, true);
			break;
		case "toast":
			toast(msg.message);
			break;
		default:
			for (const cb of TW.listeners[msg.type] || []) cb(msg);
	}
};

/** Calls `cb` with every message of this type the app sends the page. */
export function on(type, cb) {
	(TW.listeners[type] ||= []).push(cb);
}

/** Resolves once the app has prepared previews and details for the files. */
export function prepared() {
	if (TW.prepared) return Promise.resolve(TW.prepared);
	return new Promise((r) => TW.waiting.push(r));
}

function hexToRgb(hex) {
	const n = parseInt(hex.slice(1), 16);
	return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function luminance([r, g, b]) {
	const f = (c) => {
		c /= 255;
		return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
	};
	return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

function contrast(a, b) {
	const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
	return (x + 0.05) / (y + 0.05);
}

const mix = (a, b, t) => a.map((v, i) => Math.round(v + (b[i] - v) * t));

/** Same readable accent the wheel uses: lighter on dark, a touch darker on light. */
export function applyTheme(t) {
	const root = document.documentElement;
	root.toggleAttribute("data-dark", !!t.dark);
	root.toggleAttribute("data-win11", !!t.win11);
	root.toggleAttribute("data-hc", !!t.highContrast);
	if (t.highContrast) return;
	const bg = t.dark ? [43, 43, 43] : [249, 249, 249];
	let c = t.dark ? mix(hexToRgb(t.accent), [255, 255, 255], 0.4) : mix(hexToRgb(t.accent), [0, 0, 0], 0.12);
	for (let i = 0; i < 10 && contrast(c, bg) < 3; i++) c = mix(c, t.dark ? [255, 255, 255] : [0, 0, 0], 0.15);
	const on = contrast(c, [0, 0, 0]) >= contrast(c, [255, 255, 255]) ? "#000000" : "#ffffff";
	root.style.setProperty("--accent", `rgb(${c.join(",")})`);
	root.style.setProperty("--on-accent", on);
}

/** Builds elements: h("div", { class: "x", onclick }, child, "text"). */
export function h(tag, attrs = {}, ...kids) {
	const el = document.createElement(tag);
	for (const [k, v] of Object.entries(attrs || {})) {
		if (v === undefined || v === null || v === false) continue;
		if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
		else if (k === "html") el.innerHTML = v;
		else if (k === "style" && typeof v === "object") Object.assign(el.style, v);
		else el.setAttribute(k, v === true ? "" : v);
	}
	for (const kid of kids.flat()) {
		if (kid === undefined || kid === null || kid === false) continue;
		el.append(kid instanceof Node ? kid : document.createTextNode(String(kid)));
	}
	return el;
}

export { icon };

export function iconEl(name, size = 20) {
	const span = h("span", { style: { display: "inline-flex" }, "aria-hidden": "true" });
	span.innerHTML = icon(name, size);
	return span;
}

let toastTimer;
export function toast(text, error = false) {
	const el = document.getElementById("toast");
	el.textContent = text;
	el.classList.toggle("error", error);
	el.classList.add("show");
	clearTimeout(toastTimer);
	toastTimer = setTimeout(() => el.classList.remove("show"), error ? 5000 : 2600);
}

export function group(label, ...kids) {
	const value = h("span", { class: "value" });
	const box = h("section", { class: "group" }, label ? h("div", { class: "label" }, h("span", {}, label), value) : null, ...kids);
	box.setValue = (v) => (value.textContent = v);
	return box;
}

/** A slider with its value shown in the group's label. */
export function slider({ label, min, max, step = 1, value, format = (v) => v, oninput }) {
	const input = h("input", { type: "range", min, max, step, value, "aria-label": label });
	const box = group(label, input);
	const paint = () => {
		const f = ((input.value - min) / (max - min)) * 100;
		input.style.setProperty("--fill", `${f}%`);
		box.setValue(format(Number(input.value)));
	};
	input.addEventListener("input", () => {
		paint();
		oninput?.(Number(input.value));
	});
	input.addEventListener("dblclick", () => {
		input.value = value;
		paint();
		oninput?.(Number(input.value));
	});
	paint();
	box.input = input;
	box.set = (v) => {
		input.value = v;
		paint();
	};
	return box;
}

/** Choices side by side; one is always picked. */
export function segmented(options, value, onchange) {
	const box = h("div", { class: "seg", role: "radiogroup" });
	const buttons = options.map((o) => {
		const b = h("button", { type: "button", "aria-pressed": String(o.value === value), role: "radio" }, o.icon ? iconEl(o.icon, 16) : null, o.label);
		b.addEventListener("click", () => {
			for (const x of buttons) x.setAttribute("aria-pressed", "false");
			b.setAttribute("aria-pressed", "true");
			onchange(o.value);
		});
		return b;
	});
	box.append(...buttons);
	box.select = (v) => {
		options.forEach((o, i) => buttons[i].setAttribute("aria-pressed", String(o.value === v)));
	};
	return box;
}

export function toggle(label, checked, onchange, hint) {
	const input = h("input", { type: "checkbox", class: "switch", role: "switch", "aria-label": label });
	input.checked = !!checked;
	input.addEventListener("change", () => onchange(input.checked));
	return h("label", { class: "toggle-row" }, h("span", {}, label, hint ? h("div", { class: "hint" }, hint) : null), input);
}

export function numberInput(value, { min = 0, max = 100000, step = 1, onchange, label } = {}) {
	const input = h("input", { type: "number", min, max, step, value, "aria-label": label || "" });
	input.addEventListener("input", () => {
		const v = Number(input.value);
		if (Number.isFinite(v)) onchange?.(v);
	});
	return input;
}

export function stem(name) {
	const lower = name.toLowerCase();
	for (const d of [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"]) if (lower.endsWith(d)) return name.slice(0, -d.length);
	const i = name.lastIndexOf(".");
	return i > 0 ? name.slice(0, i) : name;
}

/** How the result will be named, matching the app's own naming. */
export function outName(file, suffix, ext) {
	const s = stem(file.name);
	return `${suffix ? `${s}-${suffix}` : s}.${ext || file.ext}`;
}

export function loadImage(url) {
	return new Promise((resolve, reject) => {
		const img = new Image();
		img.onload = () => resolve(img);
		img.onerror = () => reject(new Error("The picture couldn't be read"));
		img.src = url;
	});
}

/** Saves a canvas to a file the app can pick up, and returns its path. */
export async function saveCanvas(canvas, type = "image/png", quality = 0.95) {
	const blob = await new Promise((r) => canvas.toBlob(r, type, quality));
	if (!blob) throw new Error("The picture is too large to save");
	const ext = type === "image/jpeg" ? "jpg" : type.split("/")[1];
	const res = await fetch(`/save?ext=${ext}`, { method: "POST", body: blob });
	if (!res.ok) throw new Error(await res.text());
	return (await res.json()).path;
}

export async function saveBlob(blob, ext) {
	const res = await fetch(`/save?ext=${ext}`, { method: "POST", body: blob });
	if (!res.ok) throw new Error(await res.text());
	return (await res.json()).path;
}

/** The editor frame: stage, side panel, and the bottom bar with the result's name and Save. */
export function shell({ title, lead, panel = [], primary = "Save", onSave, out = "" }) {
	const stage = h("div", { class: "stage" });
	const outEl = h("div", { class: "out" });
	// nothing is saved until the editor has its files in front of it
	const save = h("button", { class: "btn primary", type: "button", disabled: true }, primary);
	let ready = false;
	const cancel = h("button", { class: "btn", type: "button", onclick: () => send("close") }, "Cancel");
	const busy = (on) => {
		save.disabled = on || !ready;
		save.textContent = on ? "Saving" : primary;
	};
	save.addEventListener("click", async () => {
		busy(true);
		try {
			await onSave();
		} catch (e) {
			busy(false);
			toast(e.message || String(e), true);
		}
	});
	const side = h("aside", { class: "panel" }, h("h2", {}, title), lead ? h("p", { class: "lead" }, lead) : null, ...panel);
	const root = h("div", { class: "editor rise" }, stage, side, h("footer", { class: "bar" }, outEl, cancel, save));
	const app = document.getElementById("app");
	app.className = "";
	app.replaceChildren(root);
	const setOut = (html) => (outEl.innerHTML = html);
	setOut(out);
	addEventListener("keydown", (e) => {
		if (e.key === "Escape") send("close");
		if (e.key === "Enter" && !save.disabled && (e.ctrlKey || !/INPUT|TEXTAREA|SELECT/.test(document.activeElement?.tagName || ""))) save.click();
	});
	const ui = {
		stage,
		side,
		setOut,
		save,
		busy,
		ready() {
			ready = true;
			save.disabled = false;
		},
	};
	current = ui;
	return ui;
}

/** The editor on screen, so the loader can unlock Save once it has finished loading. */
let current = null;

export function savesAs(name) {
	return `Saves as <b>${escapeHtml(name)}</b> next to the original`;
}

export function escapeHtml(s) {
	return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
}

export function apply(recipe, extra = {}) {
	send("apply", { recipe, ...extra });
}

export function stageLoading(stage, text = "Getting it ready") {
	stage.replaceChildren(h("div", { class: "loading" }, h("div", { class: "spinner" }), h("div", {}, text)));
}

/** Fits a w × h picture inside the stage, returning its on-screen scale. */
export function fit(stage, w, h, pad = 32) {
	const r = stage.getBoundingClientRect();
	return Math.min((r.width - pad * 2) / w, (r.height - pad * 2) / h, 1);
}

export function formatTime(t) {
	if (!Number.isFinite(t)) return "0:00";
	const m = Math.floor(t / 60);
	const s = t - m * 60;
	const whole = Math.floor(s);
	const tenth = Math.floor((s - whole) * 10);
	return `${m}:${String(whole).padStart(2, "0")}.${tenth}`;
}

export function formatSize(bytes) {
	const units = ["KB", "MB", "GB"];
	if (bytes < 1024) return `${bytes} bytes`;
	let v = bytes / 1024;
	let u = 0;
	while (v >= 1024 && u < units.length - 1) {
		v /= 1024;
		u++;
	}
	return `${v >= 100 ? v.toFixed(0) : v.toFixed(1)} ${units[u]}`;
}

const EDITORS = {
	crop: "crop",
	add_background: "background",
	adjust: "adjust",
	annotate: "annotate",
	redact: "annotate",
	resize: "resize",
	compress: "compress",
	trim: "trim",
	make_gif: "trim",
	metadata: "metadata",
	code_image: "code",
	combine: "combine",
	protect: "protect",
	contents: "contents",
	rename: "rename",
	info: "info",
};

async function start(init) {
	try {
		if (init.page === "settings") {
			(await import("./settings.js")).default(init);
			return;
		}
		const name = EDITORS[init.tool];
		if (!name) throw new Error(`No window for ${init.tool}`);
		const run = (await import(`./editors/${name}.js`)).default;
		try {
			await run(init);
			current?.ready();
		} catch (e) {
			// the frame is up: say what went wrong inside it and keep Cancel working
			if (!current) throw e;
			current.stage.replaceChildren(h("div", { class: "hint", style: { padding: "24px", textAlign: "center" } }, e.message || String(e)));
		}
	} catch (e) {
		const app = document.getElementById("app");
		app.className = "loading";
		app.replaceChildren(h("div", {}, e.message || String(e)));
	}
}

send("ready");

const ENCODABLE = ["jpg", "jpeg", "jfif", "jpe", "png", "webp", "avif", "tif", "tiff", "bmp", "gif", "tga", "qoi"];
const AS_JPG = ["heic", "heif", "hif", "jxl", "jp2", "j2k", "dng", "cr2", "cr3", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "raf", "pef", "srw", "3fr", "erf", "kdc", "mrw", "x3f"];

/** The format a picture keeps after an edit, like the app decides it. */
export function keepImageExt(ext) {
	if (ENCODABLE.includes(ext)) return { jpeg: "jpg", jfif: "jpg", jpe: "jpg", tif: "tiff" }[ext] || ext;
	return AS_JPG.includes(ext) ? "jpg" : "png";
}

/** The container a video or sound keeps after an edit. */
export function keepMediaExt(ext, video) {
	if (video) return { mov: "mov", qt: "mov", mkv: "mkv", webm: "webm", avi: "avi", wmv: "wmv", asf: "wmv" }[ext] || "mp4";
	return { m4a: "m4a", aac: "m4a", alac: "m4a", m4b: "m4a", wav: "wav", wave: "wav", flac: "flac", ogg: "ogg", oga: "ogg", opus: "opus", aif: "aiff", aiff: "aiff", aifc: "aiff", wma: "wma", m4r: "m4r" }[ext] || "mp3";
}
