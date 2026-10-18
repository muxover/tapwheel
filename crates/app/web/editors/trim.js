// Trim and Make GIF. A player and a timeline (frames for video, the sound wave for audio)
// with in and out points. Keep a part, cut a part out, or split at markers into files.
// Space plays, I and O set the points, M adds a split marker, arrows step.

import { apply, formatTime, group, h, iconEl, keepMediaExt, outName, prepared, savesAs, segmented, shell, stageLoading, stem, toggle } from "../tw.js";

export default async function trim(init) {
	const gif = init.tool === "make_gif";
	const file = init.files[0];
	const st = { mode: "keep", start: 0, end: 0, markers: [], exact: init.defaults.trim_exact, fps: init.defaults.gif_fps, width: init.defaults.gif_width };
	const startOut = h("b");
	const endOut = h("b");
	const lenOut = h("b");
	const modeSeg = gif
		? null
		: segmented(
				[
					{ label: "Keep part", value: "keep", icon: "checkmark" },
					{ label: "Cut out", value: "cut", icon: "cut" },
					{ label: "Split", value: "split", icon: "split_vertical" },
				],
				"keep",
				(m) => {
					st.mode = m;
					markersBox.style.display = m === "split" ? "" : "none";
					draw();
					name();
				},
			);
	const markersList = h("div", { class: "hint" });
	const markersBox = group(
		"Split points",
		h("div", { class: "row" }, h("button", { class: "btn", type: "button", onclick: () => addMarker() }, "Add at playhead"), h("button", { class: "btn subtle", type: "button", onclick: () => { st.markers = []; draw(); name(); } }, "Clear")),
		markersList,
	);
	markersBox.style.display = "none";
	const panel = [
		modeSeg ? group("Do", modeSeg) : null,
		group(
			"Selection",
			h("div", { class: "row", style: { justifyContent: "space-between" } }, h("span", {}, "Start ", startOut), h("span", {}, "End ", endOut)),
			h("div", { class: "hint" }, "Length ", lenOut),
			h("div", { class: "row", style: { marginTop: "8px" } },
				h("button", { class: "btn", type: "button", onclick: () => setIn() }, "Set start ", h("kbd", {}, "I")),
				h("button", { class: "btn", type: "button", onclick: () => setOut() }, "Set end ", h("kbd", {}, "O")),
			),
		),
		markersBox,
		gif
			? group(
					"GIF",
					h("div", { class: "label" }, "Frames per second"),
					segmented([10, 12, 15, 20, 25].map((v) => ({ label: String(v), value: v })), st.fps, (v) => (st.fps = v)),
					h("div", { class: "label", style: { marginTop: "10px" } }, "Width"),
					segmented([320, 480, 640, 800, 1080].map((v) => ({ label: String(v), value: v })), st.width, (v) => (st.width = v)),
					h("div", { class: "hint" }, "Shorter clips and smaller widths make lighter GIFs."),
				)
			: group(null, toggle("Exact cuts", st.exact, (v) => (st.exact = v), "Off cuts at the nearest keyframe instantly, without re-encoding.")),
	];
	const ui = shell({
		title: gif ? "Make GIF" : "Trim",
		lead: "Drag the handles on the timeline, or play and press I and O.",
		panel,
		primary: gif ? "Make GIF" : "Save",
		onSave: async () => {
			const s = { start: st.start, end: st.end };
			if (gif) return apply({ do: "make_gif", span: s, fps: st.fps, width: st.width });
			let keep;
			let separate = false;
			if (st.mode === "keep") keep = [s];
			else if (st.mode === "cut") keep = [{ start: 0, end: st.start }, { start: st.end, end: dur }].filter((p) => p.end - p.start > 0.05);
			else {
				const cuts = [0, ...st.markers.filter((m) => m > 0.05 && m < dur - 0.05).sort((a, b) => a - b), dur];
				keep = cuts.slice(1).map((e, i) => ({ start: cuts[i], end: e }));
				separate = true;
				if (keep.length < 2) throw new Error("Add at least one split point");
			}
			if (!keep.length) throw new Error("Nothing would be left");
			apply({ do: "trim", keep, separate, exact: st.exact });
		},
	});
	stageLoading(ui.stage, "Getting the timeline ready");
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const dur = info.duration || 0;
	st.end = dur;
	const video = !!info.hasVideo;
	const ext = keepMediaExt(file.ext, video);
	function name() {
		if (gif) return ui.setOut(savesAs(`${stem(file.name)}.gif`));
		if (st.mode === "split") return ui.setOut(`Saves the parts in a folder named <b>${stem(file.name)} parts</b>`);
		ui.setOut(savesAs(outName(file, "trimmed", ext)));
	}
	name();

	const player = video ? h("video", { src: info.url, preload: "auto", style: { maxWidth: "100%", maxHeight: "100%", borderRadius: "var(--radius-s)", background: "#000" } }) : h("audio", { src: info.url, preload: "auto" });
	const playBtn = h("button", { class: "btn icon", type: "button", "aria-label": "Play", onclick: () => toggleplay() }, iconEl("play"));
	const now = h("span", { style: { fontVariantNumeric: "tabular-nums", minWidth: "120px" } });
	const track = h("div", { style: { position: "relative", height: video ? "72px" : "96px", borderRadius: "var(--radius-s)", overflow: "hidden", background: "var(--surface)", cursor: "pointer", touchAction: "none" } });
	const wave = h("canvas", { style: { position: "absolute", inset: "0", width: "100%", height: "100%" } });
	if (info.filmstrip) track.append(h("div", { style: { position: "absolute", inset: "0", backgroundImage: `url("${info.filmstrip}")`, backgroundSize: "100% 100%", opacity: "0.9" } }));
	if (info.wave) track.append(wave);
	const shadeL = h("div", { style: { position: "absolute", top: "0", bottom: "0", left: "0", background: "rgba(0,0,0,.55)" } });
	const shadeR = h("div", { style: { position: "absolute", top: "0", bottom: "0", right: "0", background: "rgba(0,0,0,.55)" } });
	const sel = h("div", { style: { position: "absolute", top: "0", bottom: "0", border: "2px solid var(--accent)", borderRadius: "var(--radius-s)", boxSizing: "border-box" } });
	const grip = (side) =>
		h("div", {
			"data-grip": side,
			style: { position: "absolute", top: "0", bottom: "0", width: "14px", [side === "in" ? "left" : "right"]: "-8px", background: "var(--accent)", borderRadius: "4px", cursor: "ew-resize" },
		});
	sel.append(grip("in"), grip("out"));
	const head = h("div", { style: { position: "absolute", top: "-4px", bottom: "-4px", width: "2px", marginLeft: "-1px", background: "#fff", boxShadow: "0 0 0 1px rgba(0,0,0,.4)", pointerEvents: "none" } });
	const marks = h("div", { style: { position: "absolute", inset: "0", pointerEvents: "none" } });
	track.append(shadeL, shadeR, sel, marks, head);
	const controls = h("div", { class: "row", style: { width: "100%" } }, playBtn, now, h("div", { style: { flex: "1" } }), h("span", { class: "hint", style: { margin: 0 } }, "Space plays, arrows step, M marks a split"));
	const visual = video ? player : h("div", { class: "loading" }, iconEl("music_note_2", 48), h("div", {}, file.name), player);
	ui.stage.style.placeItems = "stretch";
	ui.stage.replaceChildren(
		h("div", { style: { display: "grid", gridTemplateRows: "1fr auto auto", gap: "12px", padding: "16px", height: "100%", minHeight: "0" } },
			h("div", { style: { display: "grid", placeItems: "center", minHeight: "0", overflow: "hidden" } }, visual),
			controls,
			track,
		),
	);

	function paintWave() {
		if (!info.wave) return;
		const r = track.getBoundingClientRect();
		wave.width = Math.max(1, Math.round(r.width * devicePixelRatio));
		wave.height = Math.max(1, Math.round(r.height * devicePixelRatio));
		const ctx = wave.getContext("2d");
		ctx.clearRect(0, 0, wave.width, wave.height);
		ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue("--accent");
		ctx.globalAlpha = video ? 0.55 : 0.9;
		const n = info.wave.length;
		const mid = wave.height / 2;
		const band = video ? wave.height * 0.35 : wave.height * 0.46;
		for (let i = 0; i < n; i++) {
			const x = (i / n) * wave.width;
			const v = Math.max(0.02, info.wave[i]) * band;
			ctx.fillRect(x, video ? wave.height - v * 2 : mid - v, Math.max(1, wave.width / n - 0.5), video ? v * 2 : v * 2);
		}
	}

	const pct = (t) => `${(t / dur) * 100}%`;
	function draw() {
		const cut = st.mode === "cut";
		sel.style.left = pct(st.start);
		sel.style.width = pct(st.end - st.start);
		shadeL.style.width = cut ? "0" : pct(st.start);
		shadeR.style.width = cut ? "0" : pct(dur - st.end);
		sel.style.background = cut ? "rgba(0,0,0,.6)" : "transparent";
		sel.style.display = st.mode === "split" ? "none" : "";
		shadeL.style.display = shadeR.style.display = st.mode === "split" ? "none" : "";
		marks.replaceChildren(
			...(st.mode === "split" ? st.markers : []).map((m) => h("div", { style: { position: "absolute", top: "0", bottom: "0", left: pct(m), width: "2px", background: "var(--accent)" } })),
		);
		startOut.textContent = formatTime(st.start);
		endOut.textContent = formatTime(st.end);
		const kept = st.mode === "cut" ? dur - (st.end - st.start) : st.end - st.start;
		lenOut.textContent = formatTime(st.mode === "split" ? dur : kept);
		markersList.textContent = st.markers.length ? `${st.markers.length + 1} parts: ${[0, ...st.markers.sort((a, b) => a - b)].map(formatTime).join(", ")}` : "Play to a spot and add a point, or press M.";
	}
	function tick() {
		head.style.left = pct(player.currentTime);
		now.textContent = `${formatTime(player.currentTime)} / ${formatTime(dur)}`;
		if (!player.paused) {
			// playback follows the edit: keep plays the part, cut skips it
			if (st.mode === "keep" && player.currentTime >= st.end) {
				player.pause();
				player.currentTime = st.end;
			}
			if (st.mode === "cut" && player.currentTime >= st.start && player.currentTime < st.end) player.currentTime = st.end;
			requestAnimationFrame(tick);
		}
	}
	function toggleplay() {
		if (player.paused) {
			if (st.mode === "keep" && (player.currentTime < st.start || player.currentTime >= st.end - 0.05)) player.currentTime = st.start;
			player.play();
		} else player.pause();
	}
	player.addEventListener("play", () => {
		playBtn.replaceChildren(iconEl("pause"));
		tick();
	});
	player.addEventListener("pause", () => playBtn.replaceChildren(iconEl("play")));
	player.addEventListener("timeupdate", () => player.paused && tick());
	function setIn() {
		st.start = Math.min(player.currentTime, st.end - 0.1);
		draw();
	}
	function setOut() {
		st.end = Math.max(player.currentTime, st.start + 0.1);
		draw();
	}
	function addMarker() {
		const t = player.currentTime;
		if (t > 0.05 && t < dur - 0.05 && !st.markers.some((m) => Math.abs(m - t) < 0.2)) st.markers.push(t);
		draw();
		name();
	}
	const timeAt = (e) => {
		const r = track.getBoundingClientRect();
		return Math.min(dur, Math.max(0, ((e.clientX - r.left) / r.width) * dur));
	};
	let drag = null;
	track.addEventListener("pointerdown", (e) => {
		track.setPointerCapture(e.pointerId);
		const g = e.target.dataset?.grip;
		const t = timeAt(e);
		if (g) drag = { what: g };
		else if (st.mode !== "split" && e.target === sel) drag = { what: "move", from: t, s: st.start, e: st.end };
		else {
			drag = { what: "seek" };
			player.currentTime = t;
			tick();
		}
	});
	track.addEventListener("pointermove", (e) => {
		if (!drag) return;
		const t = timeAt(e);
		if (drag.what === "in") {
			st.start = Math.min(t, st.end - 0.1);
			player.currentTime = st.start;
		} else if (drag.what === "out") {
			st.end = Math.max(t, st.start + 0.1);
			player.currentTime = st.end;
		} else if (drag.what === "move") {
			const len = drag.e - drag.s;
			st.start = Math.min(Math.max(0, drag.s + t - drag.from), dur - len);
			st.end = st.start + len;
		} else player.currentTime = t;
		draw();
		tick();
	});
	track.addEventListener("pointerup", () => (drag = null));
	addEventListener("keydown", (e) => {
		if (/INPUT|TEXTAREA/.test(document.activeElement?.tagName || "")) return;
		const step = video && info.fps ? 1 / info.fps : 0.1;
		if (e.key === " ") {
			e.preventDefault();
			toggleplay();
		} else if (e.key === "i" || e.key === "I") setIn();
		else if (e.key === "o" || e.key === "O") setOut();
		else if ((e.key === "m" || e.key === "M") && !gif) {
			modeSeg.select("split");
			st.mode = "split";
			markersBox.style.display = "";
			addMarker();
		} else if (e.key === "ArrowLeft") player.currentTime = Math.max(0, player.currentTime - (e.shiftKey ? 1 : step));
		else if (e.key === "ArrowRight") player.currentTime = Math.min(dur, player.currentTime + (e.shiftKey ? 1 : step));
		else return;
		tick();
	});
	addEventListener("resize", paintWave);
	requestAnimationFrame(() => {
		paintWave();
		draw();
		tick();
	});
}
