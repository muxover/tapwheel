// The homepage: a wheel to play with, the app's real tool windows with sample files, and every
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
// text written here gets the same line ties as the page: its last words never sit alone
const tie = (text, words = 2) => {
	const parts = String(text).split(" ");
	if (parts.length <= words) return parts.join("\u00a0");
	return `${parts.slice(0, -words).join(" ")} ${parts.slice(-words).join("\u00a0")}`;
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

// the hero's wheels are the real ones for a photo; what each drop would do, in the app's words
const MOVE_TO = ["Desktop", "Documents", "Choose"];
function heroScenes(k) {
	const file = { name: k.sample, kind: "image", tint: ["#ffc178", "#e8506f"] };
	const ext = k.sample.split(".").pop();
	const slice = (t) => ({ id: t.id, label: t.label, icon: t.icon, desc: t.desc, window: t.window, ring: t.id === "move_to" ? MOVE_TO : t.ring.length ? t.ring : undefined });
	const paged = (list) => {
		const all = list.map(slice);
		if (all.length <= 8) return [all];
		const pages = [];
		for (let i = 0; i < all.length; i += 7) pages.push(all.slice(i, i + 7));
		return pages;
	};
	const scene = (key, list, pickId, ringPick, done) => {
		const pages = paged(list);
		const pick = Math.max(0, pages[0].findIndex((t) => t.id === pickId));
		return { key, file, pages, slices: pages[0], more: pages.length > 1, pick, ringPick, running: key === 1 ? { title: pages[0][pick].desc, detail: k.sample, icon: pages[0][pick].icon } : null, done, work: key === 1 ? 900 : 0 };
	};
	return [
		scene(0, k.convert, "to-jpg", undefined, { title: "photo.jpg", detail: "Saved next to photo.heic", icon: "" }),
		scene(1, k.tools, "compress", 0, { title: "photo-small.jpg", detail: "Saved next to photo.heic", icon: "arrow_minimize" }),
		scene(2, k.actions, "copy_path", 1, { title: "Copied the name", detail: "photo.heic", icon: "link" }),
	].map((sc) => ({ ...sc, ext }));
}

function heroDone(key, sl, ring) {
	const name = "photo.heic";
	if (key === 0) return { title: `photo.${sl.label.toLowerCase()}`, detail: tie(`Saved next to ${name}`), icon: "checkmark_circle", work: 700, running: sl.desc };
	if (sl.window) return { title: `The ${sl.label} window opens`, detail: "You can try it further down", icon: sl.icon };
	const said = {
		copy: ["Copied 1 item, ready to paste", name],
		copy_path: [`Copied the ${(ring || "Path").toLowerCase()}`, name],
		move_to: [ring === "Choose" ? "A folder picker opens" : `Moved to ${ring || "a folder"}`, name],
		duplicate: ["photo - Copy.heic", "Saved next to the original"],
		share: ["The Windows share sheet opens", name],
		float: ["Floating on screen", "Drag it out again whenever you need it"],
		hash: [`Copied the ${ring || "SHA-256"} checksum`, name],
		properties: ["Properties opens", name],
		open_with: ["Open with opens", name],
		zip: ["photo.zip", "Saved next to the original"],
	}[sl.id];
	if (said) return { title: said[0], detail: said[1], icon: sl.icon };
	return { title: ring ? `${sl.label}: ${ring}` : sl.label, detail: `The new file lands next to ${name}`, icon: sl.icon, work: 900, running: sl.desc };
}

function setupHero(data) {
	const el = $("#hero-demo");
	if (!el) return;
	const k = data?.kinds.find((x) => x.id === "images");
	el.replaceChildren(k ? demo({ scenes: heroScenes(k), play: { done: heroDone, hint: "Hold the photo, then tap Shift, Ctrl or Alt. Or use the keys below." } }) : demo());
}

// the header marks the section on screen
function setupNav() {
	const links = [...document.querySelectorAll('.nav a[href^="#"]')];
	const sections = links.map((a) => document.querySelector(a.getAttribute("href"))).filter(Boolean);
	if (!sections.length || !("IntersectionObserver" in window)) return;
	const shown = new Map();
	const io = new IntersectionObserver(
		(es) => {
			for (const e of es) shown.set(e.target.id, e.isIntersecting);
			const on = sections.find((x) => shown.get(x.id));
			for (const a of links) {
				if (on && a.getAttribute("href") === `#${on.id}`) a.setAttribute("aria-current", "true");
				else a.removeAttribute("aria-current");
			}
		},
		{ rootMargin: "-45% 0px -50% 0px" },
	);
	for (const x of sections) io.observe(x);
}

// the windows to try, grouped the way the wheel groups them: [id, label, icon, sample]
const TRY_GROUPS = [
	["Pictures", [
		["crop", "Crop", "crop", "photo.jpg"],
		["touchup", "Remove BG", "image_sparkle", "plant.jpg"],
		["add_background", "Add background", "image_add", "plant.png"],
		["adjust", "Adjust", "options", "photo.jpg"],
		["annotate", "Annotate", "pen", "photo.jpg"],
		["redact", "Redact", "eye_off", "photo.jpg"],
		["resize", "Resize", "resize", "photo.jpg"],
		["compress", "Compress", "arrow_minimize", "photo.jpg"],
		["metadata", "Photo details", "info", "photo.jpg"],
		["combine", "Combine", "grid", "2 files"],
	]],
	["Video", [
		["trim", "Trim", "cut", "clip.mp4"],
		["make_gif", "Make GIF", "gif", "clip.mp4"],
		["crop-video", "Crop", "crop", "clip.mp4"],
		["snapshot", "Snapshot", "camera", "clip.mp4"],
	]],
	["Audio", [
		["trim-audio", "Trim", "music_note_2", "song.m4a"],
		["tags", "Song tags", "tag", "song.m4a"],
	]],
	["Any file", [
		["code_image", "Code shot", "code_block", "app.ts"],
		["rename", "Rename", "rename", "2 files"],
		["protect", "Password", "lock_closed", "album"],
		["contents", "Archive contents", "archive", "album.zip"],
		["info", "File info", "document_search", "photo.jpg"],
	]],
];
const TRY = TRY_GROUPS.flatMap(([, list]) => list);

let current = null;
let lastUrl = null;

function openTool(id, focus = false) {
	const t = TRY.find((x) => x[0] === id) || TRY[0];
	current = t[0];
	for (const b of document.querySelectorAll("#try-tools button")) {
		const on = b.dataset.tool === current;
		b.setAttribute("aria-pressed", String(on));
		if (on && focus) b.focus();
		if (on) b.scrollIntoView({ block: "nearest", inline: "nearest" });
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
	for (const [group, tools] of TRY_GROUPS) {
		const name = `try-group-${group.toLowerCase().replace(/ /g, "-")}`;
		list.append(
			h(
				"div",
				{ class: "rail-group", role: "group", "aria-labelledby": name },
				h("span", { class: "rail-head", id: name }, group),
				tools.map(([id, label, ic]) => h("button", { type: "button", "data-tool": id, "aria-pressed": "false", "aria-controls": "try-body", onclick: () => openTool(id) }, icon(ic, 18), label)),
			),
		);
	}
	list.addEventListener("keydown", (e) => {
		const i = TRY.findIndex((x) => x[0] === current);
		const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
		if (!step) return;
		e.preventDefault();
		openTool(TRY[(i + step + TRY.length) % TRY.length][0], true);
	});
	const body = $("#try-body");
	const frame = $("#try-window");
	const fitFrame = () => {
		// the window lays itself out at the width it gets, as the app's own window does when
		// resized: side by side when there's room, stacked on a phone
		const w = body.clientWidth;
		const tall = w < 640 ? 720 : 640;
		frame.style.width = `${w}px`;
		frame.style.height = `${tall}px`;
		body.style.height = `${tall}px`;
		list.style.height = innerWidth > 900 ? `${tall}px` : "";
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
	images: { tool: "compress", ring: 0, done: ["photo-small.jpg", "Saved next to the original"] },
	video: { tool: "compress", ring: 2, done: ["clip-small.mp4", "Saved next to the original"] },
	audio: { tool: "compress", ring: 1, done: ["song-small.mp3", "Saved next to the original"] },
	icons: { tool: "windows_icon", done: ["logo.ico", "Every size from 16 to 256"] },
	text: { tool: "read_aloud", done: ["notes.mp3", "Read in a Windows voice"] },
	code: { tool: "pretty", done: ["app-pretty.ts", "Saved next to the original"] },
	archives: { tool: "extract", done: ["album", "Unpacked into a folder"] },
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

/** A tool on the kind's wheel: its icon and name, a link to its window when it has one here. */
function toolItem(t, kind) {
	const id = tryFor(kind, t.id);
	const inner = [icon(t.icon || "document", 18), h("span", {}, t.label)];
	if (!id) return h("li", { title: t.desc }, h("span", { class: "tool-item" }, inner));
	return h("li", {}, h("a", { class: "tool-item live", href: `#try-${id}`, onclick: (e) => tryTool(e, id), title: `Open the ${t.label} window here` }, inner, h("span", { class: "go", "aria-hidden": "true" }, "Try")));
}

/** Names in a line, wrapping between names and never inside one. */
function inlineList(names, cls = "") {
	return h("ul", { class: `inline-list ${cls}`.trim() }, names.map((n) => h("li", {}, n)));
}

/** "Opens 48 formats", with the full list a click away. */
function formats(reads) {
	const all = inlineList(reads, "mono");
	all.hidden = true;
	const more = h("button", { type: "button", class: "text-button", "aria-expanded": "false" }, `Opens ${reads.length} formats`);
	more.addEventListener("click", () => {
		const open = all.hidden;
		all.hidden = !open;
		more.setAttribute("aria-expanded", String(open));
	});
	return h("div", { class: "kind-block formats" }, more, all);
}

// the window to open for a tool, by kind of file where the same tool has its own window
const TRY_FOR = { crop: "crop", add_background: "add_background", adjust: "adjust", annotate: "annotate", redact: "redact", resize: "resize", compress: "compress", metadata: "metadata", trim: "trim", make_gif: "make_gif", combine: "combine", code_image: "code_image", protect: "protect", contents: "contents", rename: "rename", info: "info", remove_background: "touchup", snapshot: "snapshot" };
const TRY_BY_KIND = { "video:crop": "crop-video", "audio:trim": "trim-audio", "audio:metadata": "tags", "video:compress": null, "audio:compress": null, "video:resize": null };
const tryFor = (kind, id) => (`${kind}:${id}` in TRY_BY_KIND ? TRY_BY_KIND[`${kind}:${id}`] : TRY_FOR[id]);

function tryTool(e, id) {
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
		const d = demo({ scenes: [sceneFor(k)], labels: false });
		// a phone shows the wheel itself, big enough to read
		if (matchMedia("(max-width: 700px)").matches) d.querySelector("svg").setAttribute("viewBox", "400 0 620 580");
		const scope = h("div", { class: "tw-scope", "data-tw-scope": true, "data-follow": true }, d);
		const reads = k.reads.map((x) => x.toUpperCase());
		const tools = [...k.tools, ...k.many];
		panel.replaceChildren(
			h("div", { class: "kind-wheel" }, scope),
			h(
				"div",
				{ class: "kind-lists" },
				h("div", { class: "kind-block" }, h("h3", {}, h("span", { class: "key" }, "Ctrl"), "Tools"), h("ul", { class: "toolgrid" }, tools.map((t) => toolItem(t, k.id)))),
				h("div", { class: "kind-block" }, h("h3", {}, h("span", { class: "key" }, "Shift"), "Converts to"), inlineList(k.convert.map((c) => c.label))),
				h("div", { class: "kind-block" }, h("h3", {}, h("span", { class: "key" }, "Alt"), "Actions"), inlineList(k.actions.map((a) => a.label))),
				formats(reads),
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

// sections ease in once as they reach the screen; without script or with reduced motion they are simply there
function setupReveal() {
	if (!("IntersectionObserver" in window) || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
	const items = [...document.querySelectorAll("main section .wrap > *, .hero-stage, .facts")];
	const io = new IntersectionObserver(
		(es) => {
			for (const e of es) {
				if (!e.isIntersecting) continue;
				e.target.classList.add("shown");
				io.unobserve(e.target);
			}
		},
		{ rootMargin: "0px 0px -8% 0px" },
	);
	for (const x of items) {
		const r = x.getBoundingClientRect();
		if (r.top < innerHeight) continue;
		x.classList.add("reveal");
		io.observe(x);
	}
}

/** The three wheels, each cropped to the wheel itself at the moment before the drop. */
function setupTrio() {
	for (const pic of document.querySelectorAll(".trio-pic")) {
		const d = demo({ scene: Number(pic.dataset.scene), at: Number(pic.dataset.at), labels: false });
		d.querySelector("svg").setAttribute("viewBox", "400 0 620 580");
		pic.append(h("div", { class: "tw-scope", "data-tw-scope": true, "data-follow": true }, d));
	}
}

function setupIcons() {
	for (const el of document.querySelectorAll("[data-icon]")) el.replaceChildren(icon(el.dataset.icon, 22));
}

function setupLooks() {
	const grid = $("#looks-grid");
	if (!grid) return;
	// each still is the moment just before the drop
	const looks = [
		["Rounded, dark", true, true, 1, 3600],
		["Rounded, light", true, false, 0, 2900],
		["Square, dark", false, true, 2, 3600],
		["Square, light", false, false, 1, 3600],
	];
	for (const [name, win11, dark, scene, at] of looks) {
		const scope = h("div", { class: "tw-scope", "data-tw-scope": true, "data-win11": win11, "data-dark": dark }, demo({ scene, at, labels: false }));
		grid.append(h("figure", {}, scope, h("figcaption", {}, name)));
	}
}

setupTry();
setupTrio();
setupIcons();
setupLooks();
setupReveal();
setupNav();
applyLook();
fetch("/data/wheel.json")
	.then((r) => r.json())
	.then((data) => {
		setupHero(data);
		setupStats(data);
		setupKinds(data);
		applyLook();
	})
	.catch(() => setupHero(null));
