// Code shot: a picture of code, highlighted, in a window, on a backdrop. Drawn at twice the
// size for sharp text anywhere.

import { apply, fit, group, h, prepared, saveCanvas, savesAs, segmented, shell, slider, stageLoading, stem, toggle } from "../tw.js";
import { backdropPicker, paintBackdrop, roundRect } from "./backdrops.js";

export default async function code(init) {
	const file = init.files[0];
	const st = {
		dark: init.defaults.code_theme !== "light",
		bg: init.defaults.code_background || "sunset",
		padding: 56,
		size: 15,
		numbers: true,
		frame: true,
		title: file.name,
	};
	const preview = h("canvas");
	const titleIn = h("input", { type: "text", value: st.title, "aria-label": "Window title" });
	titleIn.addEventListener("input", () => set({ title: titleIn.value }));
	const ui = shell({
		title: "Code shot",
		lead: "A sharp picture of your code to share.",
		panel: [
			group("Theme", segmented([{ label: "Dark", value: true }, { label: "Light", value: false }], st.dark, (v) => set({ dark: v }))),
			group("Background", backdropPicker(st.bg, (v) => set({ bg: v }), { blur: false })),
			slider({ label: "Padding", min: 0, max: 128, value: st.padding, format: (v) => `${v}px`, oninput: (v) => set({ padding: v }) }),
			slider({ label: "Text size", min: 10, max: 26, value: st.size, format: (v) => `${v}px`, oninput: (v) => set({ size: v }) }),
			group(null, toggle("Line numbers", st.numbers, (v) => set({ numbers: v })), toggle("Window frame", st.frame, (v) => set({ frame: v }))),
			group("Title", titleIn),
		],
		onSave: async () => {
			const out = h("canvas");
			render(out, 2);
			apply({ do: "rendered", file: await saveCanvas(out), suffix: "", to: "png" });
		},
	});
	ui.setOut(savesAs(`${stem(file.name)}.png`));
	stageLoading(ui.stage);
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	ui.stage.classList.add("checker");
	ui.stage.replaceChildren(h("div", { class: "canvas-wrap", style: { boxShadow: "none" } }, preview));
	const font = (k) => `${st.size * k}px "Cascadia Mono", "Cascadia Code", Consolas, monospace`;

	function render(canvas, k) {
		const lines = (st.dark ? info.dark : info.light).slice(0, 400);
		const ctx = canvas.getContext("2d");
		ctx.font = font(k);
		const adv = ctx.measureText("M").width;
		const lh = Math.round(st.size * k * 1.55);
		const cols = Math.min(160, Math.max(20, ...lines.map((l) => l.reduce((a, [, t]) => a + t.replace(/\t/g, "    ").length, 0))));
		const gutter = st.numbers ? (String(lines.length).length + 2) * adv : 0;
		const inner = 22 * k;
		const bar = st.frame ? 40 * k : 0;
		const ww = gutter + cols * adv + inner * 2;
		const wh = bar + lines.length * lh + inner * 2;
		const pad = st.bg === "none" ? 0 : st.padding * k;
		canvas.width = Math.ceil(ww + pad * 2);
		canvas.height = Math.ceil(wh + pad * 2);
		paintBackdrop(ctx, st.bg, canvas.width, canvas.height);
		const r = 12 * k;
		const winBg = st.dark ? "#282c34" : "#fafafa";
		const dim = st.dark ? "#636d83" : "#a0a1a7";
		if (pad > 0) {
			ctx.save();
			ctx.shadowColor = "rgba(0,0,0,.35)";
			ctx.shadowBlur = 40 * k;
			ctx.shadowOffsetY = 14 * k;
			roundRect(ctx, pad, pad, ww, wh, r);
			ctx.fillStyle = winBg;
			ctx.fill();
			ctx.restore();
		}
		roundRect(ctx, pad, pad, ww, wh, r);
		ctx.fillStyle = winBg;
		ctx.fill();
		if (st.frame) {
			ctx.fillStyle = dim;
			ctx.font = `600 ${12.5 * k}px "Segoe UI Variable Text", "Segoe UI", sans-serif`;
			ctx.textBaseline = "middle";
			ctx.fillText(st.title, pad + inner, pad + bar / 2);
			// Windows caption buttons
			ctx.strokeStyle = dim;
			ctx.lineWidth = 1.2 * k;
			const cy = pad + bar / 2;
			const gx = pad + ww - inner - 10 * k;
			ctx.beginPath();
			ctx.moveTo(gx - 5 * k, cy - 5 * k);
			ctx.lineTo(gx + 5 * k, cy + 5 * k);
			ctx.moveTo(gx + 5 * k, cy - 5 * k);
			ctx.lineTo(gx - 5 * k, cy + 5 * k);
			ctx.strokeRect(gx - 41 * k, cy - 5 * k, 10 * k, 10 * k);
			ctx.moveTo(gx - 77 * k, cy);
			ctx.lineTo(gx - 67 * k, cy);
			ctx.stroke();
			ctx.fillStyle = st.dark ? "#3b4048" : "#e5e5e6";
			ctx.fillRect(pad, pad + bar, ww, Math.max(1, k));
		}
		ctx.font = font(k);
		ctx.textBaseline = "middle";
		lines.forEach((line, i) => {
			const y = pad + bar + inner + i * lh + lh / 2;
			if (st.numbers) {
				ctx.fillStyle = dim;
				ctx.textAlign = "right";
				ctx.fillText(String(i + 1), pad + inner + gutter - adv * 1.5, y);
				ctx.textAlign = "left";
			}
			let x = pad + inner + gutter;
			let used = 0;
			for (const [colour, text] of line) {
				const t = text.replace(/\t/g, "    ").slice(0, Math.max(0, 160 - used));
				used += t.length;
				ctx.fillStyle = colour;
				ctx.fillText(t, x, y);
				x += ctx.measureText(t).width;
			}
		});
	}

	let queued = false;
	function set(c) {
		Object.assign(st, c);
		if (!queued) {
			queued = true;
			requestAnimationFrame(() => {
				queued = false;
				draw();
			});
		}
	}
	function draw() {
		render(preview, 1);
		const s = fit(ui.stage, preview.width, preview.height, 24);
		preview.style.width = `${preview.width * s}px`;
		preview.style.height = `${preview.height * s}px`;
	}
	addEventListener("resize", draw);
	await document.fonts.ready;
	draw();
}
