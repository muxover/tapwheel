// The homepage: the playable wheel, the three keys, every kind of file with what its wheel offers
// (data/wheel.json, made from the app) and the tool windows as screens from the app.
import { demo } from "/app/demo.js";
import { icons } from "/app/icons.js";

const $ = (s, el = document) => el.querySelector(s);
const h = (tag, attrs = {}, ...kids) => {
	const el = document.createElement(tag);
	for (const [k, v] of Object.entries(attrs)) {
		if (v === null || v === undefined || v === false) continue;
		if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
		else el.setAttribute(k, v === true ? "" : v);
	}
	for (const kid of kids.flat()) if (kid !== null && kid !== undefined && kid !== false) el.append(kid instanceof Node ? kid : String(kid));
	return el;
};
const icon = (name, size = 18) => {
	const el = h("span", { class: "ic", "aria-hidden": "true" });
	if (icons[name]) el.innerHTML = `<svg width="${size}" height="${size}" viewBox="0 0 20 20"><path d="${icons[name]}" fill="currentColor"/></svg>`;
	return el;
};
const crop = (d) => {
	d.querySelector("svg").setAttribute("viewBox", "400 0 620 580");
	return d;
};

// the hero: the real wheels for a photo, and what each drop does in the app's words
const MOVE_TO = ["Desktop", "Documents", "Choose"];
function heroScenes(k) {
	const file = { name: k.sample, kind: "image", tint: ["#ffc178", "#e8506f"] };
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
	];
}

function heroDone(key, sl, ring) {
	const name = "photo.heic";
	if (key === 0) return { title: `photo.${sl.label.toLowerCase()}`, detail: `Saved next to ${name}`, icon: "checkmark_circle", work: 700, running: sl.desc };
	if (sl.window) return { title: `${sl.label} opens its window`, detail: "Set it up, then save", icon: sl.icon };
	const said = {
		copy: ["Copied, ready to paste", name],
		copy_path: [`Copied the ${(ring || "Path").toLowerCase()}`, name],
		move_to: [ring === "Choose" ? "A folder picker opens" : `Moved to ${ring || "a folder"}`, name],
		duplicate: ["photo - Copy.heic", "Saved next to the original"],
		share: ["The Windows share sheet opens", name],
		float: ["Floating on screen", "Drag it out again when you need it"],
		hash: [`Copied the ${ring || "SHA-256"} checksum`, name],
		properties: ["Properties opens", name],
		open_with: ["Open with opens", name],
		zip: ["photo.zip", "Saved next to the original"],
	}[sl.id];
	if (said) return { title: said[0], detail: said[1], icon: sl.icon };
	return { title: ring ? `${sl.label}: ${ring}` : sl.label, detail: `Saved next to ${name}`, icon: sl.icon, work: 900, running: sl.desc };
}

// the wheel each kind's still shows, picking its most used tool
const PICKS = { images: ["compress", 0], video: ["compress", 2], audio: ["compress", 1], icons: ["windows_icon"], text: ["read_aloud"], code: ["pretty"], archives: ["extract"] };
const KIND = { images: "image", video: "video", audio: "audio", icons: "icon", text: "text", code: "code", archives: "archive" };
const TINT = { images: ["#7cc8ff", "#4b5bdc"], video: ["#9be15d", "#00a3a3"], audio: ["#ff9a8b", "#ff6a88"], icons: ["#c49bff", "#6a4cff"], text: ["#e0e0e0", "#9e9e9e"], code: ["#5ee7df", "#3a7bd5"], archives: ["#ffd76b", "#e8a33a"] };
function kindScene(k) {
	const [tool, ring] = PICKS[k.id];
	const shown = k.tools.length > 8 ? k.tools.slice(0, 7) : k.tools;
	const at = Math.max(0, shown.findIndex((t) => t.id === tool));
	const t = shown[at];
	return {
		key: 1,
		file: { name: k.sample, kind: KIND[k.id], tint: TINT[k.id] },
		slices: shown.map((x) => ({ label: x.label, icon: x.icon, desc: x.desc, ring: x.ring.length ? x.ring : undefined })),
		more: k.tools.length > 8,
		pick: at,
		ringPick: t.ring.length ? Math.min(ring || 0, t.ring.length - 1) : undefined,
		running: { title: t.desc, detail: k.sample, icon: t.icon },
		done: { title: "Done", detail: k.sample, icon: t.icon },
		work: 900,
	};
}

function setupKeys() {
	const stage = $("#keys-stage");
	const keys = [...document.querySelectorAll("#wheels [data-scene]")];
	const show = (b) => {
		for (const k of keys) k.setAttribute("aria-pressed", String(k === b));
		stage.replaceChildren(crop(demo({ scene: Number(b.dataset.scene), at: Number(b.dataset.at), labels: false })));
	};
	for (const k of keys) k.addEventListener("click", () => show(k));
	show(keys[1]);
}

function setupKinds(data) {
	const tabs = $("#kind-tabs");
	const wheel = $("#kind-wheel");
	const tools = $("#kind-tools");
	const conv = $("#kind-convert");
	const reads = $("#kind-reads");
	const show = (id, focus) => {
		const k = data.kinds.find((x) => x.id === id);
		for (const b of tabs.children) {
			const on = b.dataset.kind === id;
			b.setAttribute("aria-selected", String(on));
			b.tabIndex = on ? 0 : -1;
			if (on && focus) b.focus();
		}
		wheel.replaceChildren(crop(demo({ scenes: [kindScene(k)], labels: false })));
		tools.replaceChildren(...[...k.tools, ...k.many].map((t) => h("li", { title: t.desc }, icon(t.icon || "document"), t.label)));
		conv.replaceChildren(...k.convert.map((c) => h("li", {}, c.label)));
		const all = h("ul", { class: "formats", id: "kind-reads-all", hidden: true }, k.reads.map((r) => h("li", {}, r.toUpperCase())));
		const more = h("button", { type: "button", class: "more", "aria-expanded": "false", "aria-controls": "kind-reads-all" }, `Opens ${k.reads.length} formats`, icon("chevron_down", 16));
		more.addEventListener("click", () => {
			all.hidden = !all.hidden;
			more.setAttribute("aria-expanded", String(!all.hidden));
		});
		reads.replaceChildren(more, all);
	};
	for (const k of data.kinds) tabs.append(h("button", { type: "button", role: "tab", "data-kind": k.id, "aria-selected": "false", "aria-controls": "kind-panel", onclick: () => show(k.id) }, k.name));
	tabs.addEventListener("keydown", (e) => {
		const ids = data.kinds.map((k) => k.id);
		const i = ids.indexOf(tabs.querySelector('[aria-selected="true"]').dataset.kind);
		const step = { ArrowRight: 1, ArrowLeft: -1 }[e.key];
		if (!step) return;
		e.preventDefault();
		show(ids[(i + step + ids.length) % ids.length], true);
	});
	show("images");
}

// the windows, one at a time; the list moves on by itself until someone picks one
const WINDOWS = [
	["touchup", "Remove BG", "image_sparkle", "Cut out the subject on your PC, then paint over anything it missed."],
	["trim", "Trim", "cut", "Keep a part, cut one out or split, on a timeline with a filmstrip."],
	["crop", "Crop", "crop", "Drag the box or type the size. Ratios for every screen and post."],
	["annotate", "Annotate", "pen", "Arrows, boxes, steps and text, straight on the picture."],
	["redact", "Redact", "eye_off", "Blur or black out what nobody else should read."],
	["code_image", "Code shot", "code_block", "A sharp picture of your code, with a theme and a backdrop."],
];
function setupWindows() {
	const list = $("#win-list");
	const say = $("#win-say");
	const title = $("#win-title");
	let at = 0;
	let timer = 0;
	const show = (i, user) => {
		at = i;
		const [id, label, , line] = WINDOWS[i];
		for (const [j, b] of [...list.children].entries()) b.setAttribute("aria-pressed", String(j === i));
		$("#shot-light").src = `/img/windows/${id}-light.webp`;
		$("#shot-dark").src = `/img/windows/${id}-dark.webp`;
		for (const img of [$("#shot-light"), $("#shot-dark")]) img.alt = `The ${label} window`;
		title.textContent = `${label} · ${id === "trim" ? "clip.mp4" : id === "code_image" ? "app.ts" : id === "touchup" ? "plant.jpg" : "photo.jpg"}`;
		say.textContent = line;
		if (user) clearInterval(timer);
	};
	WINDOWS.forEach(([, label, ic], i) => list.append(h("button", { type: "button", "aria-pressed": "false", onclick: () => show(i, true) }, icon(ic), label)));
	show(0);
	if (!matchMedia("(prefers-reduced-motion: reduce)").matches) timer = setInterval(() => show((at + 1) % WINDOWS.length), 5000);
}

function setupHeader() {
	const bar = $(".header");
	const onScroll = () => bar.toggleAttribute("data-scrolled", scrollY > 8);
	addEventListener("scroll", onScroll, { passive: true });
	onScroll();
}

// the header marks the section on screen
function setupNav() {
	const links = [...document.querySelectorAll('.nav a[href^="/#"]')];
	const sections = links.map((a) => document.querySelector(a.getAttribute("href").slice(1))).filter(Boolean);
	if (!sections.length || !("IntersectionObserver" in window)) return;
	const shown = new Map();
	const io = new IntersectionObserver(
		(es) => {
			for (const e of es) shown.set(e.target.id, e.isIntersecting);
			const on = sections.find((x) => shown.get(x.id));
			for (const a of links) {
				if (on && a.getAttribute("href") === `/#${on.id}`) a.setAttribute("aria-current", "true");
				else a.removeAttribute("aria-current");
			}
		},
		{ rootMargin: "-45% 0px -50% 0px" },
	);
	for (const x of sections) io.observe(x);
}

function setupIcons() {
	for (const el of document.querySelectorAll("[data-icon]")) el.replaceChildren(icon(el.dataset.icon, 20));
}

function setupCopy() {
	const button = $("#cmd-copy");
	const text = $("#cmd-text");
	if (!button) return;
	button.addEventListener("click", async () => {
		try {
			await navigator.clipboard.writeText(text.textContent);
			button.textContent = "Copied";
		} catch {
			getSelection().selectAllChildren(text);
			button.textContent = "Selected";
		}
		setTimeout(() => (button.textContent = "Copy"), 1600);
	});
}

setupHeader();
setupNav();
setupIcons();
setupCopy();
setupWindows();
setupKeys();
fetch("/data/wheel.json")
	.then((r) => r.json())
	.then((data) => {
		const k = data.kinds.find((x) => x.id === "images");
		$("#hero-demo").replaceChildren(demo({ scenes: heroScenes(k), play: { done: heroDone, hint: "Hold the photo, tap Shift, Ctrl or Alt, then drop. Or use the keys." } }));
		setupKinds(data);
		const reads = new Set(data.kinds.flatMap((x) => x.reads)).size;
		const tools = new Set(data.kinds.flatMap((x) => [...x.tools, ...x.many, ...x.actions].map((t) => t.id))).size;
		$('[data-stat="reads"]').textContent = reads;
		$('[data-stat="tools"]').textContent = tools;
	})
	.catch(() => $("#hero-demo").replaceChildren(demo()));
