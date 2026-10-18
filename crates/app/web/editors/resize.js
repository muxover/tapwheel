// Resize to an exact box, a width, a height or a percentage. Pictures are never enlarged.

import { apply, group, h, keepImageExt, numberInput, outName, prepared, savesAs, segmented, shell, stageLoading, loadImage, fit } from "../tw.js";

export default async function resize(init) {
	const file = init.files[0];
	let mode = "box";
	let W = 0;
	let H = 0;
	const st = { width: 1920, height: 1080, percent: 50, lock: true };
	const wIn = numberInput(st.width, { label: "Width", min: 1, onchange: (v) => upd("width", v) });
	const hIn = numberInput(st.height, { label: "Height", min: 1, onchange: (v) => upd("height", v) });
	const pIn = numberInput(st.percent, { label: "Percent", min: 1, max: 100, onchange: (v) => upd("percent", v) });
	const result = h("div", { class: "hint" });
	const boxRow = h("div", { class: "row" }, wIn, h("span", {}, "×"), hIn);
	const pctRow = h("div", { class: "row", style: { display: "none" } }, pIn, h("span", {}, "%"));
	const presets = h("div", { class: "seg" },
		[["3840", 3840], ["2560", 2560], ["1920", 1920], ["1280", 1280], ["1080", 1080], ["800", 800]].map(([l, v]) =>
			h("button", { type: "button", onclick: () => { st.width = v; st.height = v; wIn.value = v; hIn.value = v; show(); } }, l),
		),
	);
	const ui = shell({
		title: "Resize",
		lead: "The picture keeps its proportions and is never made larger.",
		panel: [
			group("Size by", segmented([{ label: "Longest side", value: "box" }, { label: "Percent", value: "pct" }], "box", (m) => {
				mode = m;
				boxRow.style.display = m === "box" ? "" : "none";
				presets.style.display = m === "box" ? "" : "none";
				pctRow.style.display = m === "pct" ? "" : "none";
				show();
			})),
			group("Fit inside", boxRow, pctRow, result),
			group("Quick sizes", presets),
		],
		onSave: async () => {
			if (mode === "pct") apply({ do: "resize_image", width: null, height: null, percent: st.percent });
			else apply({ do: "resize_image", width: st.width, height: st.height, percent: null });
		},
	});
	ui.setOut(savesAs(outName(file, "resized", keepImageExt(file.ext))));
	function upd(k, v) {
		st[k] = v;
		show();
	}
	function show() {
		if (!W) return;
		let s = mode === "pct" ? st.percent / 100 : Math.min(1, st.width / W, st.height / H);
		s = Math.min(1, s);
		result.textContent = `${W} × ${H} becomes ${Math.round(W * s)} × ${Math.round(H * s)}`;
		const tag = mode === "pct" ? `${st.percent}%` : String(st.width);
		ui.setOut(savesAs(outName(file, tag, keepImageExt(file.ext))));
	}
	stageLoading(ui.stage);
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	W = info.width;
	H = info.height;
	const img = await loadImage(info.url);
	const s = fit(ui.stage, W, H);
	img.style.width = `${W * s}px`;
	ui.stage.replaceChildren(h("div", { class: "canvas-wrap" }, img));
	show();
}
