import { demo } from "./demo.js";
import { group, h, iconEl, on, segmented, send, slider, toast, toggle } from "./tw.js";
import { wheelsPage } from "./wheels.js";

const KEYS = [
	{ label: "Shift", value: "shift" },
	{ label: "Ctrl", value: "ctrl" },
	{ label: "Alt", value: "alt" },
	{ label: "Off", value: "off" },
];
const NO_DRAG = [
	{ label: "Double tap", value: "double_tap" },
	{ label: "Ctrl+Shift+Space", value: "shortcut" },
	{ label: "Off", value: "off" },
];
const NO_DRAG_HINT = {
	double_tap: "Tap a wheel's key twice, quickly, to open that wheel.",
	shortcut: "Opens the Convert wheel. Other apps can't use this shortcut while it's on.",
	off: "Wheels open only while you drag.",
};
const WHEELS = [
	["convert", "Convert", "Turns a file into another format"],
	["tools", "Tools", "Crop, trim, compress, remove backgrounds and more"],
	["actions", "Actions", "Copy, move, share, float and checksums"],
];

export default function settings(init) {
	const s = structuredClone(init.settings);
	const d = s.defaults;
	let saveTimer;
	const save = () => {
		clearTimeout(saveTimer);
		saveTimer = setTimeout(() => send("settings", { settings: s }), 250);
	};
	const setD = (k) => (v) => {
		d[k] = v;
		save();
	};

	const pages = {
		home: { label: "Home", icon: "app_generic", build: home },
		keys: { label: "Keys", icon: "keyboard", build: keys },
		wheels: { label: "Wheels", icon: "apps", build: wheels },
		tools: { label: "Tools", icon: "options", build: tools },
		general: { label: "General", icon: "settings", build: general },
		about: { label: "About", icon: "info", build: about },
	};
	const content = h("div", { class: "settings", style: { overflowY: "auto", padding: "8px 40px 48px", minHeight: "0" } });
	const nav = h("nav", { style: { display: "grid", alignContent: "start", gap: "4px", padding: "20px 12px" } });
	const navButtons = Object.entries(pages).map(([id, p]) => {
		const b = h("button", { class: "btn subtle", type: "button", style: { justifyContent: "flex-start", gap: "12px", padding: "8px 12px", position: "relative" } }, iconEl(p.icon), p.label);
		b.addEventListener("click", () => show(id));
		b.dataset.id = id;
		return b;
	});
	nav.append(h("div", { style: { display: "flex", alignItems: "center", gap: "10px", padding: "4px 12px 16px", font: "600 15px var(--font-display)" } }, logo(28), "TapWheel"), ...navButtons);
	const app = document.getElementById("app");
	app.className = "";
	app.replaceChildren(h("div", { style: { display: "grid", gridTemplateColumns: "220px 1fr", height: "100%" } }, nav, content));

	let shown = "home";
	// a tool window kept a ring choice while this window was open
	on("ownChoices", (m) => {
		s.own_choices = m.list;
		if (shown === "wheels") show("wheels");
	});

	function show(id) {
		shown = id;
		for (const b of navButtons) {
			const on = b.dataset.id === id;
			b.style.background = on ? "var(--hover)" : "";
			b.style.fontWeight = on ? "600" : "";
			b.style.boxShadow = on ? "inset 3px 0 0 var(--accent)" : "";
		}
		content.replaceChildren(h("div", { class: "rise", style: { maxWidth: "760px" } }, pages[id].build()));
		content.scrollTop = 0;
	}

	function title(text, lead) {
		return [h("h1", { style: { font: "600 28px/36px var(--font-display)", margin: "24px 0 6px" } }, text), h("p", { style: { color: "var(--text-2)", margin: "0 0 28px" } }, lead || "")];
	}

	function home() {
		return h("div", {},
			title("Drag, tap, drop", "Start dragging any file, tap a key, and drop it on the wheel."),
			demo(),
			h("div", { style: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(190px, 1fr))", gap: "12px", margin: "20px 0" } },
				WHEELS.map(([k, name, what]) =>
					h("section", { class: "group", style: { margin: 0 } },
						h("div", { class: "row" }, h("kbd", {}, label(s.keys[k])), h("b", {}, name)),
						h("div", { class: "hint" }, what),
					),
				),
			),
			group("Good to know",
				h("ul", { style: { margin: "0", paddingLeft: "18px", color: "var(--text-2)", lineHeight: "24px" } },
					h("li", {}, "Rest on a slice to see its choices on the outer ring, then drop on one."),
					h("li", {}, "More than eight slices? Rest on More, press Tab, or roll the mouse wheel."),
					h("li", {}, "Tap the same key twice to repeat your last choice."),
					h("li", {}, "Drop in the middle, or press Esc, to cancel."),
					h("li", {}, "Select files in File Explorer and tap a key twice to open its wheel without dragging."),
					h("li", {}, "Right-click a file and choose TapWheel to use the keyboard instead."),
				),
			),
			group("Practise",
				h("div", { class: "row" },
					h("span", { style: { flex: "1", color: "var(--text-2)" } }, "A folder in Documents with a photo, a sound and subtitles to try things on."),
					h("button", { class: "btn primary", type: "button", onclick: () => send("samples") }, "Open sample files"),
				),
			),
		);
	}

	function keys() {
		return h("div", {},
			title("Keys", "Each wheel opens with a quick tap of its key while you drag."),
			...WHEELS.map(([k, name, what]) => {
				const seg = segmented(KEYS, s.keys[k], (v) => assign(k, v));
				seg.dataset.wheel = k;
				return group(name, h("div", { class: "hint", style: { marginTop: 0, marginBottom: "8px" } }, what), seg);
			}),
			h("p", { class: "hint" }, "A key can only open one wheel; picking one that's in use swaps them."),
			noDrag(),
		);
	}

	function noDrag() {
		const hint = h("div", { class: "hint", style: { marginTop: "8px" } }, NO_DRAG_HINT[s.no_drag]);
		const seg = segmented(NO_DRAG, s.no_drag, (v) => {
			s.no_drag = v;
			hint.textContent = NO_DRAG_HINT[v];
			save();
		});
		return [
			h("h2", { style: section }, "Without dragging"),
			group(null, h("div", { class: "hint", style: { marginTop: 0, marginBottom: "8px" } }, "Select files in File Explorer or on the desktop, then open a wheel on them."), seg, hint),
		];
	}

	function assign(wheel, key) {
		const old = s.keys[wheel];
		if (key !== "off") {
			for (const w of Object.keys(s.keys)) if (w !== wheel && s.keys[w] === key) s.keys[w] = old;
		}
		s.keys[wheel] = key;
		save();
		show("keys");
	}

	function wheels() {
		return h("div", {}, title("Wheels", "Make each wheel your own: what's on it, in what order, how big and in what colour."), wheelsPage(s, init.wheels || [], save));
	}

	/** Next to the original, a subfolder beside it, or one fixed folder. */
	function whereResultsGo() {
		d.output ||= { put: "next_to_source" };
		const name = h("input", { type: "text", maxlength: "60", "aria-label": "Subfolder name", value: d.output.name || "TapWheel", style: { width: "220px" } });
		const path = h("span", { class: "path" }, d.output.path || "No folder chosen yet");
		const choose = h("button", { class: "btn", type: "button", onclick: () => send("pickFolder") }, "Choose folder");
		const sub = h("div", { class: "row", style: { marginTop: "10px" } }, h("span", { style: { flex: "1", color: "var(--text-2)" } }, "Folder name"), name);
		const fixed = h("div", { class: "row", style: { marginTop: "10px" } }, path, choose);
		// .row sets display, which outranks the hidden attribute
		const showRow = (el, on) => (el.style.display = on ? "" : "none");
		const show = () => {
			showRow(sub, d.output.put === "subfolder");
			showRow(fixed, d.output.put === "folder");
		};
		name.addEventListener("change", () => {
			d.output = { put: "subfolder", name: name.value.trim() || "TapWheel" };
			name.value = d.output.name;
			save();
		});
		on("folderPicked", (m) => {
			d.output = { put: "folder", path: m.path };
			path.textContent = m.path;
			show();
			save();
		});
		const seg = segmented(
			[
				{ label: "Next to the original", value: "next_to_source" },
				{ label: "In a subfolder", value: "subfolder" },
				{ label: "In one folder", value: "folder" },
			],
			d.output.put,
			(v) => {
				if (v === "subfolder") d.output = { put: v, name: name.value.trim() || "TapWheel" };
				else if (v === "folder") {
					if (!d.output.path) {
						// nothing is saved until a folder is chosen with the button, so the arrow
						// keys can pass over this choice without a dialog opening
						showRow(sub, false);
						showRow(fixed, true);
						return;
					}
					d.output = { put: v, path: d.output.path };
				} else d.output = { put: v };
				show();
				save();
			},
		);
		show();
		return group("Where results go", seg, sub, fixed, h("div", { class: "hint", style: { marginTop: "8px" } }, "A duplicate always stays next to its original. A file already there is never replaced."));
	}

	function tools() {
		const voices = init.voices || [];
		return h("div", {},
			title("Tools", "What a plain drop does. Each tool's window can still change these for one file."),
			whereResultsGo(),
			h("h2", { style: section }, "Pictures"),
			slider({ label: "Quality when converting", min: 50, max: 100, value: d.image_quality, format: (v) => `${v}%`, oninput: setD("image_quality") }),
			slider({ label: "Compress: Balanced", min: 40, max: 95, value: d.image_quality_balanced, format: (v) => `${v}%`, oninput: setD("image_quality_balanced") }),
			slider({ label: "Compress: Smaller", min: 25, max: 90, value: d.image_quality_small, format: (v) => `${v}%`, oninput: setD("image_quality_small") }),
			group(null, toggle("Remove location and camera details when converting", d.strip_on_convert, setD("strip_on_convert"), "Off keeps the date and camera so photos sort correctly.")),
			group("Add background style", segmented(["sunset", "ocean", "lime", "grape", "dusk", "mist"].map((v) => ({ label: v[0].toUpperCase() + v.slice(1), value: v })), d.background_style, setD("background_style"))),
			h("h2", { style: section }, "Video"),
			slider({ label: "Quality when converting", min: 16, max: 32, value: 48 - d.video_crf, format: (v) => quality(48 - v), oninput: (v) => setD("video_crf")(48 - v) }),
			slider({ label: "Compress: Balanced", min: 16, max: 32, value: 58 - d.video_crf_balanced, format: (v) => quality(58 - v), oninput: (v) => setD("video_crf_balanced")(58 - v) }),
			group(null, toggle("Exact cuts when trimming", d.trim_exact, setD("trim_exact"), "Off cuts instantly at the nearest keyframe.")),
			group(null, toggle("Use the graphics card for video", d.hardware_video, setD("hardware_video"), init.encoder ? `Compress on Balanced uses ${init.encoder}: much faster, files a little bigger. Smaller and size targets stay on the processor.` : "No video encoder was found on this PC's graphics card, so the processor does the work.")),
			group("GIFs", h("div", { class: "label" }, "Frames per second"), segmented([10, 12, 15, 20, 25].map((v) => ({ label: String(v), value: v })), d.gif_fps, setD("gif_fps")), h("div", { class: "label", style: { marginTop: "10px" } }, "Width"), segmented([320, 480, 640, 800, 1080].map((v) => ({ label: String(v), value: v })), d.gif_width, setD("gif_width"))),
			h("h2", { style: section }, "Audio"),
			group("Bitrate when converting", segmented([128, 160, 192, 256, 320].map((v) => ({ label: `${v}k`, value: v })), d.audio_kbps, setD("audio_kbps"))),
			slider({ label: "Fade length", min: 0.5, max: 8, step: 0.5, value: d.fade_seconds, format: (v) => `${v} s`, oninput: setD("fade_seconds") }),
			slider({ label: "Silence is quieter than", min: -60, max: -25, value: d.silence_db, format: (v) => `${v} dB`, oninput: setD("silence_db") }),
			h("h2", { style: section }, "Text and code"),
			group("Read aloud voice", voices.length
				? h("select", { "aria-label": "Voice", onchange: (e) => setD("voice")(e.target.value || null) }, h("option", { value: "" }, "Windows default"), voices.map((v) => h("option", { value: v.id, selected: v.id === d.voice }, v.name)))
				: h("div", { class: "hint" }, "Windows' own voice is used.")),
			slider({ label: "Reading speed", min: 0.5, max: 2, step: 0.1, value: d.voice_rate, format: (v) => `${v.toFixed(1)}×`, oninput: setD("voice_rate") }),
			group("Code shot", segmented([{ label: "Dark", value: "dark" }, { label: "Light", value: "light" }], d.code_theme, setD("code_theme")), h("div", { style: { height: "8px" } }), segmented(["sunset", "ocean", "lime", "grape", "dusk", "ink"].map((v) => ({ label: v[0].toUpperCase() + v.slice(1), value: v })), d.code_background, setD("code_background"))),
		);
	}

	function general() {
		return h("div", {},
			title("General", "How TapWheel starts, what it remembers and what it knows about this PC."),
			group(null,
				toggle("Start with Windows", s.start_with_windows, (v) => { s.start_with_windows = v; save(); }, "TapWheel waits quietly in the tray."),
				toggle("Favourites first", s.learn_order, (v) => { s.learn_order = v; save(); }, "Slices you use most move to the top of each wheel."),
				toggle("Pause TapWheel", init.paused, (v) => send("pause", { on: v }), "Ignores key taps until you turn this off or restart."),
			),
			thisPc(),
			group("History",
				h("div", { class: "row" },
					h("span", { style: { flex: "1", color: "var(--text-2)" } }, "Favourite counts and recent Move to folders."),
					h("button", { class: "btn", type: "button", onclick: () => { send("clearUsage"); toast("Cleared"); } }, "Clear"),
				),
			),
		);
	}

	/** What TapWheel read about this PC when it started, and what it does with it. */
	function thisPc() {
		const d = init.device;
		if (!d) return null;
		const gb = (b) => `${Math.round(b / 1024 ** 3)} GB`;
		const row = (name, value) => h("div", { class: "row" }, h("span", { style: { flex: "1", color: "var(--text-2)" } }, name), h("span", {}, value));
		const power = !d.power
			? "Plugged in, no battery"
			: `${d.power.on_battery ? "On battery" : "Plugged in"}${d.power.percent != null ? `, ${d.power.percent}%` : ""}${d.power.saver ? ", Battery saver on" : ""}`;
		const drive = d.ssd === true ? "SSD" : d.ssd === false ? "Hard drive" : "Not known";
		return group("This PC",
			row("Processor", `${d.cores} cores, ${d.arch}${d.emulated ? " (emulated)" : ""}`),
			row("Memory", gb(d.memory)),
			row("Graphics", d.gpu || "Not known"),
			row("Drive for work in progress", drive),
			row("Power", power),
			row("Screen", d.refresh_hz ? `${d.refresh_hz} Hz` : "Not known"),
			row("Video encoder", init.encoder || "Processor only"),
			h("div", { class: "hint", style: { marginTop: "8px" } },
				`TapWheel runs up to ${init.slots} ${init.slots === 1 ? "batch" : "batches"} at once on this PC. Video and background removal wait while Battery saver is on.`),
		);
	}

	function about() {
		const link = (text, url) => h("button", { class: "btn subtle", type: "button", style: { justifyContent: "flex-start" }, onclick: () => send("open", { url }) }, iconEl("globe", 16), text);
		return h("div", {},
			title("About", `Version ${init.version}`),
			group(null, h("p", { style: { margin: "0 0 8px" } }, "Free and open source under GPL-3.0. Everything runs on your PC; nothing is uploaded and there's no account."), h("p", { class: "hint", style: { margin: 0 } }, "Inspired by Tangerine for Mac. Made by Jax (muxover).")),
			group(null, link("Website", "https://tapwheel.app"), link("Privacy", "https://tapwheel.app/privacy"), link("Source code", "https://github.com/muxover/tapwheel")),
			group("Report a problem",
				h("p", { class: "hint", style: { margin: "0 0 10px" } }, "Opens a new GitHub issue in your browser, filled in with TapWheel's version, your Windows version, your screens and the end of the log, with your user name taken out. Nothing is sent until you read it and post it."),
				h("button", { class: "btn", type: "button", onclick: () => send("report") }, iconEl("error_circle", 16), "Report a problem"),
			),
		);
	}

	show("home");
}

const section = { font: "600 16px var(--font-display)", margin: "22px 0 10px" };

function label(k) {
	return { shift: "Shift", ctrl: "Ctrl", alt: "Alt", off: "Off" }[k] || k;
}

function quality(crf) {
	return crf <= 18 ? "Very high" : crf <= 22 ? "High" : crf <= 26 ? "Balanced" : crf <= 30 ? "Small" : "Smallest";
}

/** The TapWheel mark, the same drawing as the app icon (crates/ui/src/icon.rs). */
function logo(size) {
	const el = h("span", { style: { display: "inline-flex" } });
	el.innerHTML = `<svg width="${size}" height="${size}" viewBox="0 0 64 64" aria-hidden="true"><circle cx="32" cy="32" r="32" fill="#15171b"/><path d="M33.54 6.45A25.6 25.6 0 0 1 53.36 17.89L43.75 24.24A14.08 14.08 0 0 0 32.84 17.95Z" fill="#60cdff"/><path d="M54.90 20.55A25.6 25.6 0 0 1 54.90 43.45L44.59 38.30A14.08 14.08 0 0 0 44.59 25.70Z" fill="#3d424b"/><path d="M53.36 46.11A25.6 25.6 0 0 1 33.54 57.55L32.84 46.05A14.08 14.08 0 0 0 43.75 39.76Z" fill="#3d424b"/><path d="M30.46 57.55A25.6 25.6 0 0 1 10.64 46.11L20.25 39.76A14.08 14.08 0 0 0 31.16 46.05Z" fill="#3d424b"/><path d="M9.10 43.45A25.6 25.6 0 0 1 9.10 20.55L19.41 25.70A14.08 14.08 0 0 0 19.41 38.30Z" fill="#3d424b"/><path d="M10.64 17.89A25.6 25.6 0 0 1 30.46 6.45L31.16 17.95A14.08 14.08 0 0 0 20.25 24.24Z" fill="#3d424b"/><circle cx="32" cy="32" r="6.4" fill="#f4f3ef"/></svg>`;
	return el;
}
