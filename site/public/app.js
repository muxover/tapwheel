// The homepage: the wheel in motion, the app's real tool windows with sample files, and every
// kind of file with what the wheel offers for it (from data/wheel.json, made from the app).

import { demo } from "/app/demo.js";
import { icons } from "/app/icons.js";

const $ = (s, el = document) => el.querySelector(s);
const h = (tag, attrs = {}, ...kids) => {
	const el = document.createElement(tag);
	for (const [k, v] of Object.entries(attrs)) {
		if (v === null || v === undefined || v === false) continue;
		if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
		else if (k === "html") el.innerHTML = v;
		else el.setAttribute(k, v === true ? "" : v);
	}
	for (const kid of kids.flat()) if (kid !== null && kid !== undefined && kid !== false) el.append(kid instanceof Node ? kid : String(kid));
	return el;
};
const icon = (name, size = 20) => {
	const d = icons[name];
	const el = h("span", { class: "ic", "aria-hidden": "true" });
	if (d) el.innerHTML = `<svg width="${size}" height="${size}" viewBox="0 0 20 20"><path d="${d}" fill="currentColor"/></svg>`;
	return el;
};

// one look for the whole page: the hero, the tool window and the stills follow it
const look = { win11: true, dark: true };
const scopes = () => document.querySelectorAll("[data-tw-scope][data-follow]");

function applyLook() {
	for (const el of [$("#hero-scope"), ...scopes()]) {
		el.toggleAttribute("data-win11", look.win11);
		el.toggleAttribute("data-dark", look.dark);
	}
	for (const seg of document.querySelectorAll("[data-look]")) {
		for (const b of seg.querySelectorAll("button")) b.setAttribute("aria-pressed", String(b.dataset.value === (look[seg.dataset.look] ? "1" : "0")));
	}
	const frame = $("#try-window");
	frame?.contentWindow?.postMessage({ source: "tapwheel-site", type: "theme", theme: { dark: look.dark, win11: look.win11 } }, location.origin);
	$(".try-frame")?.toggleAttribute("data-win11", look.win11);
	$(".try-frame")?.toggleAttribute("data-dark", look.dark);
}

for (const seg of document.querySelectorAll("[data-look]")) {
	seg.addEventListener("click", (e) => {
		const b = e.target.closest("button");
		if (!b) return;
		look[seg.dataset.look] = b.dataset.value === "1";
		applyLook();
	});
}

$("#hero-demo")?.append(demo());

const TRY = [
	["crop", "Crop", "crop", "photo.jpg"],
	["add_background", "Add background", "image_add", "plant.png"],
	["adjust", "Adjust", "options", "photo.jpg"],
	["annotate", "Annotate", "pen", "photo.jpg"],
	["redact", "Redact", "eye_off", "photo.jpg"],
	["resize", "Resize", "resize", "photo.jpg"],
	["compress", "Compress", "arrow_minimize", "photo.jpg"],
	["metadata", "Photo details", "info", "photo.jpg"],
	["combine", "Combine", "grid", "2 files"],
	["trim", "Trim video", "cut", "clip.mp4"],
	["make_gif", "Make GIF", "gif", "clip.mp4"],
	["crop-video", "Crop video", "crop", "clip.mp4"],
	["trim-audio", "Trim sound", "music_note_2", "song.m4a"],
	["tags", "Song tags", "tag", "song.m4a"],
	["code_image", "Code shot", "code_block", "app.ts"],
	["protect", "Password", "lock_closed", "album"],
	["contents", "Archive contents", "archive", "album.zip"],
];

let current = null;
let lastUrl = null;

function openTool(id, focus = false) {
	const t = TRY.find((x) => x[0] === id) || TRY[0];
	current = t[0];
	for (const b of document.querySelectorAll("#try-tools button")) {
		const on = b.dataset.tool === current;
		b.setAttribute("aria-selected", String(on));
		b.tabIndex = on ? 0 : -1;
		if (on && focus) b.focus();
	}
	$("#try-title").textContent = `${t[1]} · ${t[3]}`;
	hideCard();
	const frame = $("#try-window");
	frame.src = `/try/host.html?tool=${encodeURIComponent(current)}&dark=${look.dark ? 1 : 0}&win11=${look.win11 ? 1 : 0}`;
}

function hideCard() {
	const card = $("#try-card");
	card.hidden = true;
	card.replaceChildren();
	if (lastUrl) URL.revokeObjectURL(lastUrl);
	lastUrl = null;
}

function showCard(msg) {
	const card = $("#try-card");
	card.replaceChildren();
	const tile = h("div", { class: "card-tile" }, icon(msg.url ? "checkmark_circle" : "info", 20));
	const text = h("div", { class: "card-text" }, h("strong", {}, msg.name || "In the app"), h("span", {}, msg.detail || msg.note || ""));
	const buttons = h("div", { class: "card-buttons" });
	if (msg.url) {
		lastUrl = msg.url;
		buttons.append(h("a", { class: "button small", href: msg.url, download: msg.name }, "Download"));
		if (msg.image) card.append(h("img", { class: "card-preview", src: msg.url, alt: `The result, ${msg.name}` }));
	}
	buttons.append(h("button", { class: "button small ghost", type: "button", onclick: () => openTool(current) }, "Again"));
	card.append(h("div", { class: "card-row" }, tile, text, buttons));
	card.hidden = false;
}

addEventListener("message", (e) => {
	if (e.origin !== location.origin || e.data?.source !== "tapwheel-try") return;
	if (e.data.type === "result") showCard(e.data);
	if (e.data.type === "close") openTool(current);
});

function setupTry() {
	const list = $("#try-tools");
	if (!list) return;
	for (const [id, label, ic] of TRY) {
		list.append(h("button", { type: "button", role: "tab", "data-tool": id, "aria-selected": "false", "aria-controls": "try-body", onclick: () => openTool(id) }, icon(ic, 18), label));
	}
	list.addEventListener("keydown", (e) => {
		const i = TRY.findIndex((x) => x[0] === current);
		const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
		if (!step) return;
		e.preventDefault();
		openTool(TRY[(i + step + TRY.length) % TRY.length][0], true);
	});
	// the windows are drawn for a computer screen; smaller screens see them scaled down
	const body = $("#try-body");
	const frame = $("#try-window");
	const fitFrame = () => {
		const k = Math.min(1, body.clientWidth / 1140);
		frame.style.transform = `scale(${k})`;
		body.style.height = `${740 * k}px`;
	};
	new ResizeObserver(fitFrame).observe(body);
	// load the first window when it comes near, so the page opens fast
	new IntersectionObserver((es, io) => {
		if (es.some((x) => x.isIntersecting)) {
			io.disconnect();
			openTool(location.hash.startsWith("#try-") ? location.hash.slice(5) : "crop");
		}
	}, { rootMargin: "400px" }).observe(body);
}

// what each kind's wheel animation picks, and what the app's card says after
const PICKS = {
	images: { tool: "compress", ring: 0, done: ["photo-small.jpg", "4.2 MB to 860 KB, 80% smaller"] },
	video: { tool: "compress", ring: 2, done: ["clip-small.mp4", "84 MB to 24.6 MB, 71% smaller"] },
	audio: { tool: "compress", ring: 1, done: ["song-small.mp3", "8.2 MB to 3.6 MB, 56% smaller"] },
	icons: { tool: "windows_icon", done: ["logo.ico", "3.4 KB to 106 KB, every size from 16 to 256"] },
	text: { tool: "read_aloud", done: ["notes.mp3", "1.2 KB to 640 KB"] },
	code: { tool: "pretty", done: ["app-pretty.ts", "2.1 KB to 2.4 KB, 14% larger"] },
	archives: { tool: "extract", done: ["album", "4 files, 1.2 MB"] },
};
const FILE_LOOK = {
	images: { kind: "image", tint: ["#7cc8ff", "#4b5bdc"] },
	video: { kind: "video", tint: ["#9be15d", "#00a3a3"] },
	audio: { kind: "image", tint: ["#ff9a8b", "#ff6a88"] },
	icons: { kind: "image", tint: ["#c49bff", "#6a4cff"] },
	text: { kind: "image", tint: ["#e0e0e0", "#9e9e9e"] },
	code: { kind: "image", tint: ["#5ee7df", "#3a7bd5"] },
	archives: { kind: "image", tint: ["#ffd76b", "#e8a33a"] },
};

function sceneFor(k) {
	const pick = PICKS[k.id];
	const many = k.tools.length > 8;
	const shown = many ? k.tools.slice(0, 7) : k.tools;
	let at = shown.findIndex((t) => t.id === pick.tool);
	if (at < 0) at = 0;
	const tool = shown[at];
	const ring = tool.ring.length ? tool.ring : null;
	return {
		key: 1,
		file: { name: k.sample, ...FILE_LOOK[k.id] },
		slices: shown.map((t) => ({ label: t.label, icon: t.icon, desc: t.desc, ring: t.ring.length ? t.ring : undefined })),
		more: many,
		pick: at,
		ringPick: ring ? Math.min(pick.ring || 0, ring.length - 1) : undefined,
		running: { title: tool.desc, detail: k.sample, icon: tool.icon },
		done: { title: pick.done[0], detail: pick.done[1], icon: tool.icon },
		work: 900,
	};
}

function chips(list, cls = "chip") {
	return h("div", { class: "chips" }, list.map((x) => h("span", { class: cls }, x)));
}

function toolRow(t) {
	const window = t.window ? h("a", { class: "badge", href: "#try", onclick: (e) => tryTool(e, t) }, "Has a window") : null;
	return h(
		"div",
		{ class: "tool", role: "listitem" },
		h("span", { class: "tool-ic" }, icon(t.icon || "document", 20)),
		h("div", {}, h("div", { class: "tool-name" }, h("strong", {}, t.label), " ", window), h("span", { class: "tool-desc" }, t.desc), t.ring.length ? chips(t.ring.filter((r) => r !== "Custom"), "ring") : null),
	);
}

const TRY_FOR = { crop: "crop", add_background: "add_background", adjust: "adjust", annotate: "annotate", redact: "redact", trim: "trim", make_gif: "make_gif", combine: "combine", code_image: "code_image", protect: "protect", contents: "contents" };

function tryTool(e, t) {
	const id = TRY_FOR[t.id];
	if (!id) return;
	e.preventDefault();
	$("#try").scrollIntoView({ behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
	openTool(id);
}

async function setupKinds(data) {
	const tabs = $("#kind-tabs");
	const panel = $("#kind-panel");
	if (!tabs) return;
	const show = (id, focus = false) => {
		const k = data.kinds.find((x) => x.id === id);
		for (const b of tabs.querySelectorAll("button")) {
			const on = b.dataset.kind === id;
			b.setAttribute("aria-selected", String(on));
			b.tabIndex = on ? 0 : -1;
			if (on && focus) b.focus();
		}
		panel.setAttribute("aria-labelledby", `kind-${id}`);
		const scope = h("div", { class: "tw-scope", "data-tw-scope": true, "data-follow": true }, demo({ scenes: [sceneFor(k)], labels: false }));
		const reads = k.reads.map((x) => x.toUpperCase());
		panel.replaceChildren(
			h("div", { class: "kind-wheel" }, scope),
			h(
				"div",
				{ class: "kind-lists" },
				h("div", { class: "kind-block" }, h("h3", {}, `Opens ${reads.length} formats`), chips(reads, "chip ext")),
				h("div", { class: "kind-block" }, h("h3", {}, h("span", { class: "key" }, "Shift"), "Convert to"), chips(k.convert.map((c) => c.label))),
				h("div", { class: "kind-block" }, h("h3", {}, h("span", { class: "key" }, "Ctrl"), "Tools"), h("div", { class: "tools", role: "list" }, k.tools.map(toolRow))),
				k.many.length ? h("div", { class: "kind-block" }, h("h3", {}, "With several files"), h("div", { class: "tools", role: "list" }, k.many.map(toolRow))) : null,
				h("div", { class: "kind-block" }, h("h3", {}, h("span", { class: "key" }, "Alt"), "Actions"), chips(k.actions.map((a) => a.label))),
			),
		);
		applyLook();
	};
	for (const k of data.kinds) {
		tabs.append(h("button", { type: "button", role: "tab", id: `kind-${k.id}`, "data-kind": k.id, "aria-selected": "false", "aria-controls": "kind-panel", onclick: () => show(k.id) }, k.name));
	}
	tabs.addEventListener("keydown", (e) => {
		const ids = data.kinds.map((k) => k.id);
		const i = ids.indexOf(tabs.querySelector('[aria-selected="true"]')?.dataset.kind);
		const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key];
		if (!step) return;
		e.preventDefault();
		show(ids[(i + step + ids.length) % ids.length], true);
	});
	show(data.kinds[0].id);
}

function setupStats(data) {
	const reads = new Set(data.kinds.flatMap((k) => k.reads));
	const tools = new Set(data.kinds.flatMap((k) => [...k.tools, ...k.many, ...k.actions].map((t) => t.id)));
	const set = (k, v) => {
		const el = document.querySelector(`[data-stat="${k}"]`);
		if (el) el.textContent = String(v);
	};
	set("reads", reads.size);
	set("tools", tools.size);
}

function setupLooks() {
	const grid = $("#looks-grid");
	if (!grid) return;
	// each still is the moment just before the drop
	const looks = [
		["Windows 11, dark", true, true, 1, 3600],
		["Windows 11, light", true, false, 0, 2900],
		["Windows 10, dark", false, true, 2, 3600],
		["Windows 10, light", false, false, 1, 3600],
	];
	for (const [name, win11, dark, scene, at] of looks) {
		const scope = h("div", { class: "tw-scope", "data-tw-scope": true, "data-win11": win11, "data-dark": dark }, demo({ scene, at, labels: false }));
		grid.append(h("figure", {}, scope, h("figcaption", {}, name)));
	}
}

setupTry();
setupLooks();
applyLook();
fetch("/data/wheel.json")
	.then((r) => r.json())
	.then((data) => {
		setupStats(data);
		setupKinds(data);
	})
	.catch(() => {});
