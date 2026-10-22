// Compress with full control: a quality, an exact target size, and a size limit.

import { apply, fit, formatSize, group, h, keepImageExt, loadImage, mediaFor, numberInput, outName, prepared, savesAs, segmented, shell, slider, stageLoading } from "../tw.js";

/** The words for a video's CRF: lower keeps more. The slider runs the other way, so it shows 58 minus the CRF. */
const quality = (crf) => (crf <= 22 ? "Very high" : crf <= 26 ? "High" : crf <= 30 ? "Balanced" : crf <= 34 ? "Small" : "Smallest");

export default async function compress(init) {
	const file = init.files[0];
	const video = init.kind === "video";
	const d = init.defaults;
	let mode = "quality";
	const st = video
		? { crf: d.video_crf_balanced, mb: 25, height: 0 }
		: { quality: d.image_quality_balanced, kb: Math.max(50, Math.round(file.size / 1024 / 3)), edge: 0 };
	const qual = video
		? slider({ label: "Quality", min: 18, max: 40, value: st.crf, format: (v) => quality(58 - v), oninput: (v) => (st.crf = 58 - v) })
		: slider({ label: "Quality", min: 30, max: 95, value: st.quality, format: (v) => `${v}%`, oninput: (v) => (st.quality = v) });
	if (video) qual.set(58 - st.crf);
	const target = numberInput(video ? st.mb : st.kb, { label: "Target size", min: 1, onchange: (v) => (video ? (st.mb = v) : (st.kb = v)) });
	const unit = h("span", {}, video ? "MB" : "KB");
	const quick = video
		? h("div", { class: "seg" }, [8, 10, 25, 50, 100].map((v) => h("button", { type: "button", onclick: () => { st.mb = v; target.value = v; } }, `${v} MB`)))
		: null;
	const targetBox = group("Aim for", h("div", { class: "row" }, target, unit), quick, h("div", { class: "hint" }, video ? "Two passes get close to the size; long videos may be scaled down to fit." : "The best quality that fits, shrinking the picture only if it must."));
	targetBox.style.display = "none";
	const limit = video
		? group("Largest size", segmented([{ label: "Original", value: 0 }, { label: "1080p", value: 1080 }, { label: "720p", value: 720 }, { label: "480p", value: 480 }], 0, (v) => (st.height = v)))
		: group("Longest side", segmented([{ label: "Original", value: 0 }, { label: "3840", value: 3840 }, { label: "2560", value: 2560 }, { label: "1920", value: 1920 }, { label: "1280", value: 1280 }], 0, (v) => (st.edge = v)));
	const ui = shell({
		title: "Compress",
		lead: `${file.name} is ${formatSize(file.size)} now.`,
		panel: [
			group("Make it smaller by", segmented([{ label: "Quality", value: "quality" }, { label: "Target size", value: "size" }], "quality", (m) => {
				mode = m;
				qual.style.display = m === "quality" ? "" : "none";
				targetBox.style.display = m === "size" ? "" : "none";
			})),
			qual,
			targetBox,
			limit,
		],
		onSave: async () => {
			if (video) {
				apply({ do: "compress_video", crf: st.crf, max_height: st.height || null, target_mb: mode === "size" ? st.mb : null });
			} else {
				apply({ do: "compress_image", quality: st.quality, max_edge: st.edge || null, target_kb: mode === "size" ? st.kb : null });
			}
		},
	});
	const ext = video ? "mp4" : (() => {
		const k = keepImageExt(file.ext);
		return ["tiff", "bmp", "tga", "qoi"].includes(k) ? "jpg" : k;
	})();
	ui.setOut(savesAs(outName(file, "small", ext)));
	stageLoading(ui.stage);
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const media = video ? mediaFor(info, { controls: true, muted: true }) : await loadImage(info.url);
	const s = fit(ui.stage, info.width || 800, info.height || 450);
	media.style.width = `${(info.width || 800) * s}px`;
	ui.stage.replaceChildren(h("div", { class: "canvas-wrap" }, media));
}
