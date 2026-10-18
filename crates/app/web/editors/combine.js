// Combine pictures: a grid, side by side, stacked, or a GIF, in the order you set.

import { apply, group, h, iconEl, loadImage, prepared, savesAs, segmented, shell, slider, stageLoading, fit } from "../tw.js";

export default async function combine(init) {
	const files = init.files;
	const st = { layout: "grid", gap: init.defaults.combine_gap ?? 16, bg: init.defaults.combine_background || "#ffffff" };
	let order = files.map((_, i) => i);
	const list = h("div", { style: { display: "grid", gap: "4px" } });
	const colour = h("input", { type: "color", value: st.bg, "aria-label": "Background", style: { width: "100%", height: "32px", border: "0", background: "transparent" } });
	colour.addEventListener("input", () => {
		st.bg = colour.value;
		draw();
	});
	const ui = shell({
		title: "Combine",
		lead: "Lay the pictures out together in one file.",
		panel: [
			group("Layout", segmented([{ label: "Grid", value: "grid" }, { label: "Side by side", value: "row" }, { label: "Stacked", value: "column" }, { label: "GIF", value: "gif" }], st.layout, (v) => {
				st.layout = v;
				draw();
			})),
			slider({ label: "Gap", min: 0, max: 64, value: st.gap, format: (v) => `${v}px`, oninput: (v) => { st.gap = v; draw(); } }),
			group("Background", colour),
			group("Order", list),
		],
		onSave: async () => apply({ do: "combine", layout: st.layout, gap: st.gap, background: st.bg }, { order }),
	});
	const out = () => ui.setOut(savesAs(st.layout === "gif" ? "Combined.gif" : "Combined.png"));
	out();
	stageLoading(ui.stage);
	const infos = await prepared();
	const imgs = await Promise.all(infos.map((i) => loadImage(i.url)));
	const canvas = h("canvas");
	ui.stage.replaceChildren(h("div", { class: "canvas-wrap" }, canvas));

	function renderList() {
		list.replaceChildren(
			...order.map((i, pos) =>
				h("div", { class: "row", style: { background: "var(--surface-2)", borderRadius: "var(--radius-s)", padding: "4px" } },
					h("img", { src: infos[i].url, style: { width: "40px", height: "40px", objectFit: "cover", borderRadius: "4px" } }),
					h("span", { style: { flex: "1", overflow: "hidden", whiteSpace: "nowrap", textOverflow: "clip" } }, files[i].name),
					h("button", { class: "btn icon subtle", type: "button", "aria-label": "Move up", disabled: pos === 0, onclick: () => move(pos, -1) }, iconEl("chevron_down", 16)),
				),
			),
		);
		// the arrow points up for "move up"
		for (const b of list.querySelectorAll("button svg")) b.style.transform = "rotate(180deg)";
	}
	function move(pos, by) {
		const j = pos + by;
		[order[pos], order[j]] = [order[j], order[pos]];
		renderList();
		draw();
	}
	function draw() {
		out();
		const pics = order.map((i) => imgs[i]);
		const n = pics.length;
		const ctx = canvas.getContext("2d");
		if (st.layout === "gif") {
			// a still of the first frame stands for the GIF
			const p = pics[0];
			canvas.width = p.naturalWidth;
			canvas.height = p.naturalHeight;
			ctx.drawImage(p, 0, 0);
		} else {
			const cols = st.layout === "row" ? n : st.layout === "column" ? 1 : Math.ceil(Math.sqrt(n));
			const rows = Math.ceil(n / cols);
			const cell = 400;
			const sizes = pics.map((p) => (st.layout === "column" ? [cell, (p.naturalHeight * cell) / p.naturalWidth] : [(p.naturalWidth * cell) / p.naturalHeight, cell]));
			const colW = [...Array(cols)].map((_, c) => Math.max(0, ...sizes.filter((_, i) => i % cols === c).map((s) => s[0])));
			const rowH = [...Array(rows)].map((_, r) => Math.max(0, ...sizes.filter((_, i) => Math.floor(i / cols) === r).map((s) => s[1])));
			const g = st.gap;
			canvas.width = Math.ceil(colW.reduce((a, b) => a + b, 0) + g * (cols + 1));
			canvas.height = Math.ceil(rowH.reduce((a, b) => a + b, 0) + g * (rows + 1));
			ctx.fillStyle = st.bg;
			ctx.fillRect(0, 0, canvas.width, canvas.height);
			pics.forEach((p, i) => {
				const c = i % cols;
				const r = Math.floor(i / cols);
				const x = g + colW.slice(0, c).reduce((a, b) => a + b, 0) + g * c + (colW[c] - sizes[i][0]) / 2;
				const y = g + rowH.slice(0, r).reduce((a, b) => a + b, 0) + g * r + (rowH[r] - sizes[i][1]) / 2;
				ctx.drawImage(p, x, y, sizes[i][0], sizes[i][1]);
			});
		}
		const s = fit(ui.stage, canvas.width, canvas.height);
		canvas.style.width = `${canvas.width * s}px`;
		canvas.style.height = `${canvas.height * s}px`;
	}
	renderList();
	addEventListener("resize", draw);
	draw();
}
