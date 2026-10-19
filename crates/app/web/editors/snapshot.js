// Snapshot: pick the exact frame and save it as a PNG.
// Space plays, arrows step one frame, Shift+arrows step a second.

import { apply, formatTime, group, h, mediaFor, outName, prepared, savesAs, shell, stageLoading } from "../tw.js";

export default async function snapshot(init) {
	const file = init.files[0];
	const time = h("div", { class: "value-big" }, "0:00.0");
	const back = h("button", { class: "btn", type: "button" }, "Previous frame");
	const next = h("button", { class: "btn", type: "button" }, "Next frame");
	const ui = shell({
		title: "Snapshot",
		lead: "Find the frame, then save it as a picture at full size.",
		primary: "Save frame",
		panel: [group("Frame at", time, h("div", { class: "row" }, back, next), h("p", { class: "hint" }, "Arrows step one frame, Shift and arrows one second, Space plays."))],
		onSave: async () => apply({ do: "snapshot", at: { at: Math.round(video.currentTime * 1000) / 1000 } }),
	});
	ui.setOut(savesAs(outName(file, "frame", "png")));
	stageLoading(ui.stage, "Getting the video ready");
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const step = 1 / (info.fps > 0 ? info.fps : 30);
	const video = mediaFor(info, { preload: "auto", muted: true, playsinline: true });
	video.muted = true;
	const scrub = h("input", { type: "range", min: 0, max: info.duration || 0, step: "any", value: 0, "aria-label": "Position" });
	const wrap = h("div", { class: "snap" }, video, scrub);
	ui.stage.replaceChildren(wrap);
	await new Promise((ok, fail) => {
		video.addEventListener("loadeddata", ok, { once: true });
		video.addEventListener("error", () => fail(new Error("This video can't be shown here")), { once: true });
		setTimeout(() => fail(new Error("The video took too long to load")), 20000);
	});
	const show = () => {
		time.textContent = formatTime(video.currentTime);
		scrub.value = video.currentTime;
		scrub.style.setProperty("--fill", `${(video.currentTime / (info.duration || video.duration || 1)) * 100}%`);
	};
	const seek = (t) => {
		video.pause();
		video.currentTime = Math.min(Math.max(0, t), Math.max(0, (info.duration || video.duration) - step));
	};
	video.addEventListener("timeupdate", show);
	video.addEventListener("seeked", show);
	scrub.addEventListener("input", () => seek(Number(scrub.value)));
	back.addEventListener("click", () => seek(video.currentTime - step));
	next.addEventListener("click", () => seek(video.currentTime + step));
	addEventListener("keydown", (e) => {
		if (/INPUT|TEXTAREA/.test(document.activeElement?.tagName || "") && document.activeElement !== scrub) return;
		const d = e.key === "ArrowLeft" ? -1 : e.key === "ArrowRight" ? 1 : 0;
		if (d) {
			e.preventDefault();
			seek(video.currentTime + d * (e.shiftKey ? 1 : step));
		} else if (e.key === " ") {
			e.preventDefault();
			if (video.paused) video.play();
			else video.pause();
		}
	});
	seek((info.duration || 0) / 2);
}
