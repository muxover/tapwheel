// Annotate (arrows, boxes, circles, lines, pen, highlighter, text, numbered steps) and Redact
// (black box, blur, pixelate). Shapes are kept as vectors in picture pixels, so the saved file
// is drawn at full resolution.

import { apply, fit, group, h, iconEl, keepImageExt, loadImage, outName, prepared, saveCanvas, savesAs, shell, slider, stageLoading } from "../tw.js";

const DRAW_TOOLS = [
	["arrow", "Arrow", "arrow_up_right"],
	["box", "Box", "rectangle_landscape"],
	["ellipse", "Circle", "circle"],
	["line", "Line", "line_horizontal_1"],
	["pen", "Pen", "pen"],
	["marker", "Highlight", "highlight"],
	["text", "Text", "text_t"],
	["step", "Step", "number_circle_1"],
];
const HIDE_TOOLS = [
	["black", "Black box", "rectangle_landscape"],
	["blur", "Blur", "blur"],
	["pixel", "Pixelate", "grid"],
];
const COLOURS = ["#ef4444", "#f97316", "#facc15", "#22c55e", "#3b82f6", "#a855f7", "#ffffff", "#111111"];

export default async function annotate(init) {
	const redact = init.tool === "redact";
	const file = init.files[0];
	const tools = redact ? HIDE_TOOLS : DRAW_TOOLS;
	const st = { tool: tools[0][0], colour: COLOURS[0], size: redact ? 30 : 6 };
	const shapes = [];
	const undone = [];
	let current = null;
	let step = 1;
	const canvas = h("canvas", { style: { cursor: "crosshair", touchAction: "none" } });

	const toolButtons = tools.map(([id, label, ic]) => {
		const b = h("button", { type: "button", "aria-pressed": String(id === st.tool), title: label }, iconEl(ic), label);
		b.addEventListener("click", () => {
			st.tool = id;
			for (const x of toolButtons) x.setAttribute("aria-pressed", String(x === b));
		});
		return b;
	});
	const swatches = COLOURS.map((c) => {
		const b = h("button", { class: "swatch", type: "button", "aria-label": c, "aria-pressed": String(c === st.colour), style: { background: c } });
		b.addEventListener("click", () => {
			st.colour = c;
			for (const x of swatches) x.setAttribute("aria-pressed", String(x === b));
		});
		return b;
	});
	const undoBtn = h("button", { class: "btn", type: "button", onclick: undo }, iconEl("arrow_undo", 16), "Undo");
	const redoBtn = h("button", { class: "btn", type: "button", onclick: redo }, iconEl("arrow_redo", 16), "Redo");
	const clearBtn = h("button", { class: "btn subtle", type: "button", onclick: clearAll }, "Clear");
	const suffix = redact ? "redacted" : "marked";
	const ui = shell({
		title: redact ? "Redact" : "Annotate",
		lead: redact ? "Drag over anything private. Blur and pixelate can't be undone by whoever gets the file." : "Draw on the picture. Ctrl+Z undoes.",
		panel: [
			group("Tool", h("div", { class: "tools" }, toolButtons)),
			redact ? null : group("Colour", h("div", { class: "swatches" }, swatches)),
			slider({ label: redact ? "Strength" : "Size", min: redact ? 6 : 2, max: redact ? 80 : 40, value: st.size, oninput: (v) => (st.size = v) }),
			group(null, h("div", { class: "row" }, undoBtn, redoBtn, clearBtn)),
		],
		onSave: async () => {
			const out = h("canvas", { width: img.naturalWidth, height: img.naturalHeight });
			paint(out.getContext("2d"), 1);
			apply({ do: "rendered", file: await saveCanvas(out), suffix, to: null });
		},
	});
	ui.setOut(savesAs(outName(file, suffix, keepImageExt(file.ext))));
	stageLoading(ui.stage);
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const img = await loadImage(info.url);
	const W = img.naturalWidth;
	const H = img.naturalHeight;
	const unit = Math.max(W, H) / 1000;
	const wrap = h("div", { class: "canvas-wrap" }, canvas);
	ui.stage.replaceChildren(wrap);
	let view = 1;
	let k = 1;

	function layout() {
		k = Math.min(1, 1600 / Math.max(W, H));
		canvas.width = Math.round(W * k);
		canvas.height = Math.round(H * k);
		view = fit(ui.stage, canvas.width, canvas.height);
		canvas.style.width = `${canvas.width * view}px`;
		canvas.style.height = `${canvas.height * view}px`;
		draw();
	}

	function toImage(e) {
		const r = canvas.getBoundingClientRect();
		return [((e.clientX - r.left) / r.width) * W, ((e.clientY - r.top) / r.height) * H];
	}

	/** Draws the picture and every shape at `s` canvas pixels per picture pixel. */
	function paint(ctx, s) {
		ctx.save();
		ctx.clearRect(0, 0, W * s, H * s);
		ctx.drawImage(img, 0, 0, W * s, H * s);
		for (const sh of current ? [...shapes, current] : shapes) shape(ctx, sh, s);
		ctx.restore();
	}

	function rectOf(sh) {
		const x = Math.min(sh.x1, sh.x2);
		const y = Math.min(sh.y1, sh.y2);
		return [x, y, Math.abs(sh.x2 - sh.x1), Math.abs(sh.y2 - sh.y1)];
	}

	function shape(ctx, sh, s) {
		const lw = sh.size * unit * s;
		ctx.lineWidth = lw;
		ctx.strokeStyle = sh.colour;
		ctx.fillStyle = sh.colour;
		ctx.lineCap = "round";
		ctx.lineJoin = "round";
		const [x, y, w, hh] = rectOf(sh).map((v) => v * s);
		switch (sh.tool) {
			case "box":
				ctx.strokeRect(x, y, w, hh);
				break;
			case "ellipse":
				ctx.beginPath();
				ctx.ellipse(x + w / 2, y + hh / 2, w / 2, hh / 2, 0, 0, Math.PI * 2);
				ctx.stroke();
				break;
			case "line":
			case "arrow": {
				const [x1, y1, x2, y2] = [sh.x1 * s, sh.y1 * s, sh.x2 * s, sh.y2 * s];
				const ang = Math.atan2(y2 - y1, x2 - x1);
				const head = lw * 4.2;
				const end = sh.tool === "arrow" ? head * 0.7 : 0;
				ctx.beginPath();
				ctx.moveTo(x1, y1);
				ctx.lineTo(x2 - Math.cos(ang) * end, y2 - Math.sin(ang) * end);
				ctx.stroke();
				if (sh.tool === "arrow") {
					ctx.beginPath();
					ctx.moveTo(x2, y2);
					ctx.lineTo(x2 - head * Math.cos(ang - 0.45), y2 - head * Math.sin(ang - 0.45));
					ctx.lineTo(x2 - head * Math.cos(ang + 0.45), y2 - head * Math.sin(ang + 0.45));
					ctx.closePath();
					ctx.fill();
				}
				break;
			}
			case "pen":
			case "marker": {
				if (sh.tool === "marker") {
					ctx.globalAlpha = 0.38;
					ctx.lineWidth = lw * 4;
					ctx.lineCap = "square";
				}
				ctx.beginPath();
				sh.points.forEach(([px, py], i) => (i ? ctx.lineTo(px * s, py * s) : ctx.moveTo(px * s, py * s)));
				ctx.stroke();
				ctx.globalAlpha = 1;
				break;
			}
			case "text": {
				const size = Math.max(14, sh.size * 4) * unit * s;
				ctx.font = `600 ${size}px "Segoe UI Variable Text", "Segoe UI", sans-serif`;
				ctx.textBaseline = "top";
				// a soft outline keeps the text readable on any picture
				ctx.lineWidth = size / 8;
				ctx.strokeStyle = sh.colour === "#111111" ? "rgba(255,255,255,.85)" : "rgba(0,0,0,.55)";
				sh.text.split("\n").forEach((line, i) => {
					ctx.strokeText(line, sh.x1 * s, sh.y1 * s + i * size * 1.2);
					ctx.fillText(line, sh.x1 * s, sh.y1 * s + i * size * 1.2);
				});
				break;
			}
			case "step": {
				const r = Math.max(10, sh.size * 2.6) * unit * s;
				ctx.beginPath();
				ctx.arc(sh.x1 * s, sh.y1 * s, r, 0, Math.PI * 2);
				ctx.fill();
				ctx.lineWidth = r / 6;
				ctx.strokeStyle = "#ffffff";
				ctx.stroke();
				ctx.fillStyle = sh.colour === "#ffffff" || sh.colour === "#facc15" ? "#111" : "#fff";
				ctx.font = `700 ${r * 1.1}px "Segoe UI Variable Text", "Segoe UI", sans-serif`;
				ctx.textAlign = "center";
				ctx.textBaseline = "middle";
				ctx.fillText(String(sh.n), sh.x1 * s, sh.y1 * s + r * 0.05);
				ctx.textAlign = "start";
				break;
			}
			case "black":
				ctx.fillStyle = "#000";
				ctx.fillRect(x, y, w, hh);
				break;
			case "blur":
			case "pixel": {
				if (w < 1 || hh < 1) break;
				ctx.save();
				ctx.beginPath();
				ctx.rect(x, y, w, hh);
				ctx.clip();
				if (sh.tool === "blur") {
					// heavy enough that it can't be sharpened back
					ctx.filter = `blur(${Math.max(4, sh.size * unit * s * 0.6)}px)`;
					for (let i = 0; i < 3; i++) ctx.drawImage(img, 0, 0, W * s, H * s);
				} else {
					const block = Math.max(4, sh.size * unit * s * 0.5);
					const tw = Math.max(1, Math.round(w / block));
					const th = Math.max(1, Math.round(hh / block));
					const tmp = h("canvas", { width: tw, height: th });
					tmp.getContext("2d").drawImage(img, x / s, y / s, w / s, hh / s, 0, 0, tw, th);
					ctx.imageSmoothingEnabled = false;
					ctx.drawImage(tmp, x, y, w, hh);
				}
				ctx.restore();
				break;
			}
		}
	}

	function draw() {
		paint(canvas.getContext("2d"), k);
		undoBtn.disabled = shapes.length === 0;
		redoBtn.disabled = undone.length === 0;
		clearBtn.disabled = shapes.length === 0;
	}

	function commit(sh) {
		shapes.push(sh);
		undone.length = 0;
		current = null;
		draw();
	}
	function undo() {
		const s = shapes.pop();
		if (s) {
			undone.push(s);
			if (s.tool === "step") step = Math.max(1, step - 1);
		}
		draw();
	}
	function redo() {
		const s = undone.pop();
		if (s) {
			shapes.push(s);
			if (s.tool === "step") step++;
		}
		draw();
	}
	function clearAll() {
		undone.push(...shapes.splice(0).reverse());
		step = 1;
		draw();
	}

	function addText(x, y) {
		const r = canvas.getBoundingClientRect();
		const box = h("textarea", {
			rows: 1,
			style: {
				position: "fixed",
				left: `${r.left + (x / W) * r.width}px`,
				top: `${r.top + (y / H) * r.height}px`,
				minWidth: "160px",
				font: '600 18px "Segoe UI Variable Text","Segoe UI",sans-serif',
				color: st.colour,
				background: "rgba(0,0,0,.35)",
				border: "1px dashed #fff",
				outline: "none",
				resize: "none",
				padding: "2px 4px",
			},
			"aria-label": "Text",
		});
		document.body.append(box);
		box.focus();
		const done = () => {
			const text = box.value.trim();
			box.remove();
			if (text) commit({ tool: "text", colour: st.colour, size: st.size, x1: x, y1: y, text });
		};
		box.addEventListener("keydown", (e) => {
			e.stopPropagation();
			if (e.key === "Enter" && !e.shiftKey) {
				e.preventDefault();
				done();
			}
			if (e.key === "Escape") box.remove();
		});
		box.addEventListener("blur", done);
	}

	canvas.addEventListener("pointerdown", (e) => {
		const [x, y] = toImage(e);
		if (st.tool === "text") {
			e.preventDefault();
			addText(x, y);
			return;
		}
		if (st.tool === "step") {
			commit({ tool: "step", colour: st.colour, size: st.size, x1: x, y1: y, n: step++ });
			return;
		}
		canvas.setPointerCapture(e.pointerId);
		current = { tool: st.tool, colour: st.colour, size: st.size, x1: x, y1: y, x2: x, y2: y, points: [[x, y]] };
	});
	canvas.addEventListener("pointermove", (e) => {
		if (!current) return;
		const [x, y] = toImage(e);
		current.x2 = x;
		current.y2 = y;
		// Shift keeps lines straight and boxes square
		if (e.shiftKey && ["box", "ellipse", "black", "blur", "pixel"].includes(current.tool)) {
			const d = Math.max(Math.abs(x - current.x1), Math.abs(y - current.y1));
			current.x2 = current.x1 + Math.sign(x - current.x1) * d;
			current.y2 = current.y1 + Math.sign(y - current.y1) * d;
		}
		if (e.shiftKey && ["line", "arrow"].includes(current.tool)) {
			const ang = Math.round(Math.atan2(y - current.y1, x - current.x1) / (Math.PI / 4)) * (Math.PI / 4);
			const len = Math.hypot(x - current.x1, y - current.y1);
			current.x2 = current.x1 + Math.cos(ang) * len;
			current.y2 = current.y1 + Math.sin(ang) * len;
		}
		if (current.tool === "pen" || current.tool === "marker") current.points.push([x, y]);
		draw();
	});
	const up = () => {
		if (!current) return;
		const tiny = Math.hypot(current.x2 - current.x1, current.y2 - current.y1) < 3 && current.points.length < 3;
		if (tiny) {
			current = null;
			draw();
		} else commit(current);
	};
	canvas.addEventListener("pointerup", up);
	canvas.addEventListener("pointercancel", up);
	addEventListener("keydown", (e) => {
		if (e.ctrlKey && e.key.toLowerCase() === "z") {
			e.preventDefault();
			undo();
		}
		if (e.ctrlKey && e.key.toLowerCase() === "y") {
			e.preventDefault();
			redo();
		}
	});
	addEventListener("resize", layout);
	layout();
}
