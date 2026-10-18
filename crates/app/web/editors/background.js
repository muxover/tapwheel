// Add background: the picture on a backdrop with padding, rounded corners and a shadow,
// sized to a ratio. Saved at full resolution as PNG.

import { apply, fit, group, h, loadImage, outName, prepared, saveCanvas, savesAs, segmented, shell, slider, stageLoading } from "../tw.js";
import { backdropPicker, paintBackdrop, roundRect } from "./backdrops.js";

const RATIOS = [
	{ label: "Auto", value: 0 },
	{ label: "1:1", value: 1 },
	{ label: "4:3", value: 4 / 3 },
	{ label: "16:9", value: 16 / 9 },
	{ label: "4:5", value: 4 / 5 },
	{ label: "9:16", value: 9 / 16 },
];

export default async function background(init) {
	const file = init.files[0];
	const st = { bg: init.defaults.background_style || "sunset", padding: 10, corners: 4, shadow: 40, ratio: 0 };
	const preview = h("canvas");
	const ui = shell({
		title: "Add background",
		lead: "Put the picture on a backdrop, ready to post or share.",
		panel: [
			group("Background", backdropPicker(st.bg, (v) => set({ bg: v }))),
			slider({ label: "Padding", min: 0, max: 40, value: st.padding, format: (v) => `${v}%`, oninput: (v) => set({ padding: v }) }),
			slider({ label: "Corners", min: 0, max: 12, value: st.corners, format: (v) => `${v}%`, oninput: (v) => set({ corners: v }) }),
			slider({ label: "Shadow", min: 0, max: 100, value: st.shadow, format: (v) => `${v}%`, oninput: (v) => set({ shadow: v }) }),
			group("Ratio", segmented(RATIOS, 0, (v) => set({ ratio: v }))),
		],
		onSave: async () => {
			const out = h("canvas");
			render(out, 1);
			const path = await saveCanvas(out);
			apply({ do: "rendered", file: path, suffix: "bg", to: "png" });
		},
	});
	ui.setOut(savesAs(outName(file, "bg", "png")));
	stageLoading(ui.stage);
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const img = await loadImage(info.url);
	ui.stage.classList.add("checker");
	ui.stage.replaceChildren(h("div", { class: "canvas-wrap", style: { boxShadow: "none" } }, preview));

	/** Draws at `k` times the picture's own resolution. */
	function render(canvas, k) {
		const W = img.naturalWidth * k;
		const H = img.naturalHeight * k;
		const pad = (Math.max(W, H) * st.padding) / 100;
		let cw = W + pad * 2;
		let ch = H + pad * 2;
		if (st.ratio > 0) {
			if (cw / ch > st.ratio) ch = cw / st.ratio;
			else cw = ch * st.ratio;
		}
		canvas.width = Math.round(cw);
		canvas.height = Math.round(ch);
		const ctx = canvas.getContext("2d");
		ctx.clearRect(0, 0, cw, ch);
		paintBackdrop(ctx, st.bg, cw, ch, img);
		const x = (cw - W) / 2;
		const y = (ch - H) / 2;
		const r = (Math.min(W, H) * st.corners) / 100;
		if (st.shadow > 0 && st.bg !== "none") {
			ctx.save();
			ctx.shadowColor = `rgba(0,0,0,${0.12 + st.shadow * 0.0035})`;
			ctx.shadowBlur = (Math.max(W, H) * st.shadow) / 900;
			ctx.shadowOffsetY = (Math.max(W, H) * st.shadow) / 4000;
			roundRect(ctx, x, y, W, H, r);
			ctx.fillStyle = "#000";
			ctx.fill();
			ctx.restore();
		}
		ctx.save();
		roundRect(ctx, x, y, W, H, r);
		ctx.clip();
		ctx.drawImage(img, x, y, W, H);
		ctx.restore();
	}

	let queued = false;
	function set(change) {
		Object.assign(st, change);
		if (queued) return;
		queued = true;
		requestAnimationFrame(() => {
			queued = false;
			draw();
		});
	}

	function draw() {
		// the preview renders smaller than the original for speed, then fits the stage
		const big = Math.max(img.naturalWidth, img.naturalHeight);
		const k = Math.min(1, 1600 / big);
		render(preview, k);
		const s = fit(ui.stage, preview.width, preview.height, 24);
		preview.style.width = `${preview.width * s}px`;
		preview.style.height = `${preview.height * s}px`;
	}
	addEventListener("resize", draw);
	draw();
}
