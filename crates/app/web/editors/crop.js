// Crop pictures and videos: drag the box or its handles, snap to a ratio, type exact sizes,
// nudge with the arrow keys. The original resolution is kept.

import { apply, fit, h, keepImageExt, keepMediaExt, numberInput, outName, prepared, savesAs, segmented, shell, stageLoading, group, formatTime } from "../tw.js";

const RATIOS = [
	{ label: "Free", value: 0 },
	{ label: "Original", value: -1 },
	{ label: "1:1", value: 1 },
	{ label: "4:3", value: 4 / 3 },
	{ label: "3:2", value: 3 / 2 },
	{ label: "16:9", value: 16 / 9 },
	{ label: "9:16", value: 9 / 16 },
	{ label: "4:5", value: 4 / 5 },
];

export default async function crop(init) {
	const file = init.files[0];
	const video = init.kind === "video";
	let box = { x: 0, y: 0, w: 1, h: 1 };
	let W = 1;
	let H = 1;
	let ratio = 0;
	let scale = 1;
	const wIn = numberInput(0, { label: "Width", onchange: (v) => setSize(v, null) });
	const hIn = numberInput(0, { label: "Height", onchange: (v) => setSize(null, v) });
	const ui = shell({
		title: "Crop",
		lead: video ? "Pick a frame to see, then drag the box. The whole video is cropped." : "Drag the box or its corners. Arrow keys nudge it.",
		panel: [
			group("Ratio", segmented(RATIOS, 0, (r) => setRatio(r))),
			group("Size in pixels", h("div", { class: "row" }, wIn, h("span", {}, "×"), hIn)),
			group(
				null,
				h("div", { class: "row" },
					h("button", { class: "btn", type: "button", onclick: () => reset() }, "Reset"),
					h("button", { class: "btn", type: "button", onclick: () => center() }, "Centre"),
				),
			),
		],
		onSave: async () => {
			const r = { x: Math.round(box.x), y: Math.round(box.y), w: Math.round(box.w), h: Math.round(box.h) };
			if (video) apply({ do: "crop_video", ...r });
			else apply({ do: "crop_image", ...r });
		},
	});
	const ext = video ? keepMediaExt(file.ext, true) : keepImageExt(file.ext);
	ui.setOut(savesAs(outName(file, "cropped", ext)));
	stageLoading(ui.stage);
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	W = info.width;
	H = info.height;
	let media;
	if (video) {
		media = h("video", { src: info.url, muted: true, preload: "auto" });
		// the box only needs the size, so a video the page can't play still crops
		await new Promise((r) => {
			media.addEventListener("loadeddata", r, { once: true });
			media.addEventListener("error", r, { once: true });
			setTimeout(r, 8000);
		});
		media.currentTime = Math.min(1, (info.duration || 0) / 2);
	} else {
		media = h("img", { src: info.url, draggable: "false" });
		await media.decode();
	}
	const overlay = h("div", { style: { position: "absolute", inset: "0" } });
	const frame = h("div", {
		style: { position: "absolute", border: "1px solid #fff", boxShadow: "0 0 0 9999px rgba(0,0,0,.55)", cursor: "move" },
	});
	const grid = h("div", {
		style: {
			position: "absolute",
			inset: "0",
			backgroundImage: "linear-gradient(rgba(255,255,255,.35) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.35) 1px, transparent 1px)",
			backgroundSize: "33.333% 33.333%",
			backgroundPosition: "-1px -1px",
			opacity: "0",
			transition: "opacity .15s",
		},
	});
	frame.append(grid);
	const handles = ["nw", "n", "ne", "e", "se", "s", "sw", "w"].map((d) => {
		const el = h("div", { class: "handle", "data-d": d, style: { cursor: `${d}-resize` } });
		overlay.append(el);
		return el;
	});
	overlay.prepend(frame);
	const wrap = h("div", { class: "canvas-wrap", style: { overflow: "hidden" } }, media, overlay);
	const scrub = video
		? h("input", { type: "range", min: 0, max: info.duration || 0, step: 0.04, value: media.currentTime, style: { width: "100%" }, "aria-label": "Frame" })
		: null;
	const label = video ? h("div", { class: "hint", style: { textAlign: "center" } }, formatTime(media.currentTime)) : null;
	if (scrub) {
		scrub.addEventListener("input", () => {
			media.currentTime = Number(scrub.value);
			label.textContent = formatTime(media.currentTime);
		});
	}
	ui.stage.replaceChildren(h("div", { style: { display: "grid", gap: "10px", justifyItems: "center", width: "100%" } }, wrap, scrub ? h("div", { style: { width: "min(560px, 90%)" } }, scrub, label) : null));

	const layout = () => {
		scale = fit(ui.stage, W, H + (video ? 60 : 0));
		media.style.width = `${W * scale}px`;
		media.style.height = `${H * scale}px`;
		draw();
	};
	const draw = () => {
		Object.assign(frame.style, {
			left: `${box.x * scale}px`,
			top: `${box.y * scale}px`,
			width: `${box.w * scale}px`,
			height: `${box.h * scale}px`,
		});
		const pts = {
			nw: [box.x, box.y],
			n: [box.x + box.w / 2, box.y],
			ne: [box.x + box.w, box.y],
			e: [box.x + box.w, box.y + box.h / 2],
			se: [box.x + box.w, box.y + box.h],
			s: [box.x + box.w / 2, box.y + box.h],
			sw: [box.x, box.y + box.h],
			w: [box.x, box.y + box.h / 2],
		};
		for (const el of handles) {
			const [x, y] = pts[el.dataset.d];
			el.style.left = `${x * scale}px`;
			el.style.top = `${y * scale}px`;
		}
		wIn.value = Math.round(box.w);
		hIn.value = Math.round(box.h);
	};
	const clamp = () => {
		box.w = Math.max(8, Math.min(box.w, W));
		box.h = Math.max(8, Math.min(box.h, H));
		box.x = Math.max(0, Math.min(box.x, W - box.w));
		box.y = Math.max(0, Math.min(box.y, H - box.h));
		if (video) {
			box.w -= box.w % 2;
			box.h -= box.h % 2;
		}
	};
	const target = () => (ratio === -1 ? W / H : ratio);
	const setRatio = (r) => {
		ratio = r;
		const t = target();
		if (t > 0) {
			// the biggest box of that shape, centred on the current one
			const cx = box.x + box.w / 2;
			const cy = box.y + box.h / 2;
			let w = W;
			let hh = w / t;
			if (hh > H) {
				hh = H;
				w = hh * t;
			}
			box = { x: cx - w / 2, y: cy - hh / 2, w, h: hh };
		}
		clamp();
		draw();
	};
	const setSize = (w, hh) => {
		const t = target();
		if (w !== null) {
			box.w = w;
			if (t > 0) box.h = w / t;
		}
		if (hh !== null) {
			box.h = hh;
			if (t > 0) box.w = hh * t;
		}
		clamp();
		draw();
	};
	const reset = () => {
		box = { x: 0, y: 0, w: W, h: H };
		setRatio(ratio);
	};
	const center = () => {
		box.x = (W - box.w) / 2;
		box.y = (H - box.h) / 2;
		draw();
	};

	let drag = null;
	overlay.addEventListener("pointerdown", (e) => {
		const d = e.target.dataset?.d || (e.target === frame || e.target === grid ? "move" : null);
		if (!d) return;
		overlay.setPointerCapture(e.pointerId);
		drag = { d, x: e.clientX, y: e.clientY, start: { ...box } };
		grid.style.opacity = "1";
	});
	overlay.addEventListener("pointermove", (e) => {
		if (!drag) return;
		const dx = (e.clientX - drag.x) / scale;
		const dy = (e.clientY - drag.y) / scale;
		const s = drag.start;
		let { x, y, w, h: hh } = s;
		if (drag.d === "move") {
			x += dx;
			y += dy;
		} else {
			if (drag.d.includes("e")) w = s.w + dx;
			if (drag.d.includes("s")) hh = s.h + dy;
			if (drag.d.includes("w")) {
				w = s.w - dx;
				x = s.x + dx;
			}
			if (drag.d.includes("n")) {
				hh = s.h - dy;
				y = s.y + dy;
			}
			const t = target();
			if (t > 0) {
				if (drag.d === "n" || drag.d === "s") w = hh * t;
				else hh = w / t;
				if (drag.d.includes("n")) y = s.y + s.h - hh;
				if (drag.d.includes("w")) x = s.x + s.w - w;
			}
			// stay inside the picture without the box flipping over
			if (x < 0) {
				w += x;
				x = 0;
			}
			if (y < 0) {
				hh += y;
				y = 0;
			}
			w = Math.min(Math.max(w, 8), W - x);
			hh = Math.min(Math.max(hh, 8), H - y);
			if (t > 0) {
				const fitW = Math.min(w, hh * t);
				w = fitW;
				hh = fitW / t;
			}
		}
		box = { x, y, w, h: hh };
		clamp();
		draw();
	});
	const end = () => {
		drag = null;
		grid.style.opacity = "0";
	};
	overlay.addEventListener("pointerup", end);
	overlay.addEventListener("pointercancel", end);
	addEventListener("keydown", (e) => {
		const step = e.shiftKey ? 10 : 1;
		const moves = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
		const m = moves[e.key];
		if (!m || /INPUT/.test(document.activeElement?.tagName || "")) return;
		e.preventDefault();
		box.x += m[0];
		box.y += m[1];
		clamp();
		draw();
	});
	addEventListener("resize", layout);
	box = { x: 0, y: 0, w: W, h: H };
	layout();
}
