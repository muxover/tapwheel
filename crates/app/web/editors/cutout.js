// Touch up a Remove BG cut-out. The mask stays full size, so the saved PNG keeps every hair.
// B toggles the brush, [ and ] change its size, Ctrl+Z undoes a stroke.

import { apply, fit, group, h, loadImage, outName, prepared, saveCanvas, savesAs, segmented, shell, slider, stageLoading, toggle } from "../tw.js";
import { backdropPicker, paintBackdrop } from "./backdrops.js";

const UNDO = 8;

export default async function cutout(init) {
	const file = init.files[0];
	const st = { brush: "remove", size: 40, soft: 0, smart: true, fill: "none", ghost: true };
	const sizeBox = slider({ label: "Brush size", min: 4, max: 300, value: st.size, format: (v) => `${v} px`, oninput: (v) => (st.size = v) });
	const brushSeg = segmented(
		[
			{ label: "Remove", value: "remove" },
			{ label: "Keep", value: "keep" },
		],
		st.brush,
		(v) => (st.brush = v),
	);
	const undoBtn = h("button", { class: "btn", type: "button", disabled: true }, "Undo stroke");
	const resetBtn = h("button", { class: "btn", type: "button" }, "Start over");
	const ui = shell({
		title: "Touch up",
		lead: "Paint over what the cut-out got wrong. Remove takes parts away, Keep brings them back.",
		panel: [
			group("Brush", brushSeg, h("div", { style: { height: "8px" } }), toggle("Follow edges", st.smart, (on) => (st.smart = on), "Only paints colours like the one under the middle of the brush")),
			sizeBox,
			slider({ label: "Edge softness", min: 0, max: 20, value: st.soft, format: (v) => (v ? `${v} px` : "Sharp"), oninput: (v) => ((st.soft = v), draw()) }),
			group("Show", toggle("The removed parts, faintly", st.ghost, (on) => ((st.ghost = on), draw()))),
			group("Fill behind", backdropPicker(st.fill, (v) => ((st.fill = v), draw()), { blur: false })),
			group("", h("div", { class: "row" }, undoBtn, resetBtn)),
		],
		onSave: async () => {
			const out = h("canvas");
			out.width = W;
			out.height = H;
			const ctx = out.getContext("2d");
			paintBackdrop(ctx, st.fill, W, H);
			ctx.drawImage(composite(W, H, 1), 0, 0);
			const path = await saveCanvas(out);
			apply({ do: "rendered", file: path, suffix: "cutout", to: "png" });
		},
	});
	ui.setOut(savesAs(outName(file, "cutout", "png")));
	stageLoading(ui.stage, "Cutting out the subject");
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const [orig, cut] = await Promise.all([loadImage(info.url), loadImage(info.cutout)]);
	const W = orig.naturalWidth;
	const H = orig.naturalHeight;

	// the original, readable for Follow edges, and the mask: only its alpha matters
	const origCv = new OffscreenCanvas(W, H);
	const origCtx = origCv.getContext("2d", { willReadFrequently: true });
	origCtx.drawImage(orig, 0, 0);
	const mask = new OffscreenCanvas(W, H);
	const mctx = mask.getContext("2d", { willReadFrequently: true });
	const first = () => {
		mctx.globalCompositeOperation = "copy";
		mctx.drawImage(cut, 0, 0, W, H);
		mctx.globalCompositeOperation = "source-over";
	};
	first();

	const view = h("canvas", { class: "cutout-view" });
	ui.stage.classList.add("checker");
	ui.stage.replaceChildren(h("div", { class: "canvas-wrap", style: { boxShadow: "none" } }, view));
	const ring = h("div", { class: "brush-ring" });
	ui.stage.append(ring);
	let k = 1;

	/** The original with the mask applied, at `w` × `h`, edges softened by `soft` scaled with it. */
	function composite(w, hh, scale) {
		const c = new OffscreenCanvas(w, hh);
		const ctx = c.getContext("2d");
		ctx.drawImage(origCv, 0, 0, w, hh);
		ctx.globalCompositeOperation = "destination-in";
		if (st.soft) ctx.filter = `blur(${st.soft * scale}px)`;
		ctx.drawImage(mask, 0, 0, w, hh);
		return c;
	}

	let queued = false;
	function draw() {
		if (queued) return;
		queued = true;
		requestAnimationFrame(() => {
			queued = false;
			k = fit(ui.stage, W, H, 24);
			const w = Math.max(1, Math.round(W * k));
			const hh = Math.max(1, Math.round(H * k));
			const dpr = devicePixelRatio || 1;
			view.width = Math.round(w * dpr);
			view.height = Math.round(hh * dpr);
			view.style.width = `${w}px`;
			view.style.height = `${hh}px`;
			const ctx = view.getContext("2d");
			ctx.clearRect(0, 0, view.width, view.height);
			paintBackdrop(ctx, st.fill, view.width, view.height);
			if (st.ghost) {
				// what's been removed, faintly, so you can see what Keep would bring back
				ctx.globalAlpha = 0.22;
				ctx.drawImage(origCv, 0, 0, view.width, view.height);
				ctx.globalAlpha = 1;
			}
			ctx.drawImage(composite(view.width, view.height, view.width / W), 0, 0);
		});
	}

	// painting: soft round dabs along the stroke, on the full-size mask
	const undo = [];
	const snapshot = async () => {
		undo.push(await createImageBitmap(mask));
		if (undo.length > UNDO) undo.shift().close();
		undoBtn.disabled = false;
	};
	function dab(x, y) {
		const r = st.size / 2 / k;
		const x0 = Math.max(0, Math.floor(x - r));
		const y0 = Math.max(0, Math.floor(y - r));
		const x1 = Math.min(W, Math.ceil(x + r));
		const y1 = Math.min(H, Math.ceil(y + r));
		if (x1 <= x0 || y1 <= y0) return;
		const bw = x1 - x0;
		const bh = y1 - y0;
		const m = mctx.getImageData(x0, y0, bw, bh);
		const md = m.data;
		let src = null;
		let ref = null;
		if (st.smart) {
			src = origCtx.getImageData(x0, y0, bw, bh).data;
			const cx = Math.min(bw - 1, Math.max(0, Math.round(x - x0)));
			const cy = Math.min(bh - 1, Math.max(0, Math.round(y - y0)));
			const i = (cy * bw + cx) * 4;
			ref = [src[i], src[i + 1], src[i + 2]];
		}
		const keep = st.brush === "keep";
		for (let py = 0; py < bh; py++) {
			for (let px = 0; px < bw; px++) {
				const dx = x0 + px + 0.5 - x;
				const dy = y0 + py + 0.5 - y;
				const d = Math.sqrt(dx * dx + dy * dy) / r;
				if (d >= 1) continue;
				// full strength inside, fading over the outer quarter
				let a = d < 0.75 ? 1 : (1 - d) / 0.25;
				const i = (py * bw + px) * 4;
				if (ref) {
					const dist = Math.abs(src[i] - ref[0]) + Math.abs(src[i + 1] - ref[1]) + Math.abs(src[i + 2] - ref[2]);
					// colours close to the middle one change fully, far ones not at all
					a *= Math.max(0, Math.min(1, (150 - dist) / 60));
				}
				if (a <= 0) continue;
				const now = md[i + 3];
				md[i + 3] = keep ? now + (255 - now) * a : now * (1 - a);
				if (keep) md[i] = md[i + 1] = md[i + 2] = 255;
			}
		}
		mctx.putImageData(m, x0, y0);
	}
	const toImage = (e) => {
		const r = view.getBoundingClientRect();
		return [((e.clientX - r.left) / r.width) * W, ((e.clientY - r.top) / r.height) * H];
	};
	let last = null;
	view.addEventListener("pointerdown", async (e) => {
		view.setPointerCapture(e.pointerId);
		await snapshot();
		last = toImage(e);
		dab(...last);
		draw();
	});
	view.addEventListener("pointermove", (e) => {
		const r = view.getBoundingClientRect();
		const sr = ui.stage.getBoundingClientRect();
		ring.style.width = ring.style.height = `${st.size}px`;
		ring.style.left = `${e.clientX - sr.left}px`;
		ring.style.top = `${e.clientY - sr.top}px`;
		ring.hidden = e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom;
		if (!last) return;
		const p = toImage(e);
		const step = Math.max(1, st.size / 4 / k);
		const dist = Math.hypot(p[0] - last[0], p[1] - last[1]);
		for (let s = step; s <= dist; s += step) {
			const t = s / dist;
			dab(last[0] + (p[0] - last[0]) * t, last[1] + (p[1] - last[1]) * t);
		}
		if (dist >= step) last = p;
		draw();
	});
	const end = () => (last = null);
	view.addEventListener("pointerup", end);
	view.addEventListener("pointercancel", end);
	view.addEventListener("pointerleave", () => (ring.hidden = true));

	undoBtn.addEventListener("click", () => {
		const b = undo.pop();
		if (!b) return;
		mctx.globalCompositeOperation = "copy";
		mctx.drawImage(b, 0, 0);
		mctx.globalCompositeOperation = "source-over";
		b.close();
		undoBtn.disabled = !undo.length;
		draw();
	});
	resetBtn.addEventListener("click", async () => {
		await snapshot();
		first();
		draw();
	});
	addEventListener("keydown", (e) => {
		if (/INPUT|TEXTAREA/.test(document.activeElement?.tagName || "")) return;
		if (e.key === "z" && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			undoBtn.click();
		} else if (e.key === "b") {
			st.brush = st.brush === "keep" ? "remove" : "keep";
			brushSeg.select(st.brush);
		} else if (e.key === "[" || e.key === "]") {
			st.size = Math.max(4, Math.min(300, st.size + (e.key === "]" ? 8 : -8)));
			sizeBox.set(st.size);
		}
	});
	addEventListener("resize", draw);
	draw();
}
