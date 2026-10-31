// Plays the app's part for its tool windows, so the website can run the real windows in a
// frame. Picture results are made right here in the browser; for video, sound and archives
// the frame says exactly what the app would save.
//
// host.html?tool=crop&dark=1&win11=1

(() => {
	const q = new URLSearchParams(location.search);
	const id = q.get("tool") || "crop";
	const theme = { dark: q.get("dark") !== "0", win11: q.get("win11") !== "0", accent: q.get("accent") || "#0078d4", highContrast: false };

	const TOOLS = {
		crop: { tool: "crop", kind: "image", files: ["photo"] },
		"crop-video": { tool: "crop", kind: "video", files: ["clip"] },
		add_background: { tool: "add_background", kind: "image", files: ["plant"] },
		adjust: { tool: "adjust", kind: "image", files: ["photo"] },
		annotate: { tool: "annotate", kind: "image", files: ["photo"] },
		redact: { tool: "redact", kind: "image", files: ["photo"] },
		resize: { tool: "resize", kind: "image", files: ["photo"] },
		compress: { tool: "compress", kind: "image", files: ["photo"] },
		trim: { tool: "trim", kind: "video", files: ["clip"] },
		"trim-audio": { tool: "trim", kind: "audio", files: ["song"] },
		make_gif: { tool: "make_gif", kind: "video", files: ["clip"] },
		metadata: { tool: "metadata", kind: "image", files: ["photo"] },
		tags: { tool: "metadata", kind: "audio", files: ["song"] },
		code_image: { tool: "code_image", kind: "code", files: ["code"] },
		combine: { tool: "combine", kind: "image", files: ["photo", "plant"] },
		protect: { tool: "protect", kind: "folder", files: ["folder"] },
		contents: { tool: "contents", kind: "archive", files: ["album"] },
		rename: { tool: "rename", kind: "image", files: ["photo", "plant"] },
		snapshot: { tool: "snapshot", kind: "video", files: ["clip"] },
		touchup: { tool: "remove_background", kind: "image", files: ["plantphoto"] },
		info: { tool: "info", kind: "image", files: ["photo"] },
	};
	const LABELS = {
		crop: ["Crop", "Cut to a size or shape"],
		add_background: ["Add BG", "Put it on a backdrop"],
		adjust: ["Adjust", "Light, colour and contrast"],
		annotate: ["Annotate", "Draw, point and write"],
		redact: ["Redact", "Hide private details"],
		resize: ["Resize", "Change the size"],
		compress: ["Compress", "Make it smaller"],
		trim: ["Trim", "Keep or cut a part"],
		make_gif: ["Make GIF", "Turn a part into a GIF"],
		metadata: ["Metadata", "See or remove hidden data"],
		code_image: ["Code shot", "A picture of your code"],
		combine: ["Combine", "Lay pictures out together"],
		protect: ["Password", "Pack with a password"],
		contents: ["Contents", "See what's inside"],
		rename: ["Rename", "New names, one or many"],
		snapshot: ["Snapshot", "Save a frame as a picture"],
		remove_background: ["Remove BG", "Cut out the subject"],
		info: ["File info", "Size, real type and details"],
	};
	const setup = TOOLS[id] || TOOLS.crop;
	// samples only these windows use: a photo with its background, and the app's cut-out of it
	const EXTRA = {
		plantphoto: {
			file: { name: "plant.jpg", path: "C:\\Users\\you\\Pictures\\plant.jpg", ext: "jpg", size: 56706 },
			prep: { url: "plant-photo.jpg", cutout: "plant.png" },
		},
	};
	const base = "/try/";
	const saved = new Map();
	let data = null;
	let inputs = [];

	const tell = (msg) => parent !== window && parent.postMessage({ source: "tapwheel-try", ...msg }, location.origin);
	const receive = (msg) => window.TW?.receive(msg);

	// pages hand finished pictures to the app through /save; keep them here instead
	const realFetch = window.fetch.bind(window);
	window.fetch = async (url, opts) => {
		if (String(url).startsWith("/save")) {
			const blob = opts.body instanceof Blob ? opts.body : new Blob([opts.body]);
			const u = URL.createObjectURL(blob);
			saved.set(u, blob);
			return new Response(JSON.stringify({ path: u }), { headers: { "content-type": "application/json" } });
		}
		return realFetch(url, opts);
	};

	async function load() {
		if (!data) data = await (await realFetch(`${base}samples.json`)).json();
		return data;
	}

	async function ready() {
		const d = await load();
		inputs = setup.files.map((k) => (k === "folder" ? { name: "album", path: "C:\\Users\\you\\Pictures\\album", ext: "", size: 0 } : EXTRA[k]?.file || d.files[k]));
		const [label, describe] = LABELS[setup.tool];
		const title = inputs.length === 1 ? `${label} · ${inputs[0].name}` : `${label} · ${inputs.length} files`;
		receive({ type: "init", page: "editor", title, tool: setup.tool, label, describe, kind: setup.kind, files: inputs, defaults: d.defaults, presets: [], theme });
		// the app prepares previews in the background; a short wait shows the loading state honestly
		setTimeout(() => {
			const files = setup.files.map((k, index) => {
				const p = k === "folder" ? {} : { ...(EXTRA[k]?.prep || d.prepared[k]) };
				for (const key of ["url", "filmstrip", "cutout"]) if (p[key]) p[key] = base + p[key];
				if (setup.tool === "info") return { index, info: details(d.files[k], p) };
				return { index, ...p };
			});
			receive({ type: "prepared", files });
		}, 350);
	}

	const stem = (name) => name.replace(/\.[^.]+$/, "");
	const named = (file, suffix, ext) => `${suffix ? `${stem(file.name)}-${suffix}` : stem(file.name)}.${ext}`;
	const size = (n) => (n < 1024 ? `${n} bytes` : n < 1048576 ? `${(n / 1024).toFixed(n < 102400 ? 1 : 0)} KB` : `${(n / 1048576).toFixed(1)} MB`);
	const change = (a, b) => {
		const pct = Math.round((b / a - 1) * 100);
		return `${size(a)} to ${size(b)}${pct <= -1 ? `, ${-pct}% smaller` : pct >= 1 ? `, ${pct}% larger` : ""}`;
	};
	const time = (t) => {
		const m = Math.floor(t / 60);
		return `${m}:${(t - m * 60).toFixed(1).padStart(4, "0")}`;
	};

	async function picture(url) {
		const img = new Image();
		img.src = url;
		await img.decode();
		return img;
	}
	function canvas(w, h) {
		const c = document.createElement("canvas");
		c.width = Math.max(1, Math.round(w));
		c.height = Math.max(1, Math.round(h));
		return c;
	}
	const blobOf = (c, type, quality) => new Promise((r) => c.toBlob(r, type, quality));

	/** Removes EXIF without touching the picture, like the app does for JPGs. */
	async function stripJpeg(url, locationOnly) {
		const bytes = new Uint8Array(await (await realFetch(url)).arrayBuffer());
		const out = [bytes.slice(0, 2)];
		let i = 2;
		while (i + 4 <= bytes.length && bytes[i] === 0xff) {
			const marker = bytes[i + 1];
			const len = (bytes[i + 2] << 8) | bytes[i + 3];
			const seg = bytes.slice(i, i + 2 + len);
			const exif = marker === 0xe1 && String.fromCharCode(...seg.slice(4, 8)) === "Exif";
			if (exif && locationOnly) out.push(dropGps(seg));
			else if (!exif) out.push(seg);
			i += 2 + len;
			if (marker === 0xda) break;
		}
		out.push(bytes.slice(i));
		return new Blob(out, { type: "image/jpeg" });
	}

	/** Blanks the GPS block inside an EXIF segment and unhooks it, leaving everything else. */
	function dropGps(seg) {
		const s = seg.slice();
		const t = 10; // after marker, length and "Exif\0\0"
		const le = s[t] === 0x49;
		const u16 = (o) => (le ? s[t + o] | (s[t + o + 1] << 8) : (s[t + o] << 8) | s[t + o + 1]);
		const u32 = (o) => (le ? (s[t + o] | (s[t + o + 1] << 8) | (s[t + o + 2] << 16) | (s[t + o + 3] << 24)) >>> 0 : ((s[t + o] << 24) | (s[t + o + 1] << 16) | (s[t + o + 2] << 8) | s[t + o + 3]) >>> 0);
		const zero = (o, n) => s.fill(0, t + o, t + o + n);
		const ifd0 = u32(4);
		const n = u16(ifd0);
		const sizes = { 1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8 };
		for (let e = 0; e < n; e++) {
			const at = ifd0 + 2 + e * 12;
			if (u16(at) !== 0x8825) continue;
			const gps = u32(at + 8);
			const count = u16(gps);
			for (let g = 0; g < count; g++) {
				const ent = gps + 2 + g * 12;
				const bytes = (sizes[u16(ent + 2)] || 1) * u32(ent + 4);
				if (bytes > 4) zero(u32(ent + 8), bytes);
			}
			zero(gps, 2 + count * 12);
			zero(at, 12);
			// keep the entry list sorted and counted: move the blank entry out by shrinking the count
			const rest = s.slice(t + at + 12, t + ifd0 + 2 + n * 12 + 4);
			s.set(rest, t + at);
			zero(ifd0 + 2 + (n - 1) * 12 + 4, 12);
			const c = n - 1;
			if (le) {
				s[t + ifd0] = c & 255;
				s[t + ifd0 + 1] = c >> 8;
			} else {
				s[t + ifd0] = c >> 8;
				s[t + ifd0 + 1] = c & 255;
			}
			break;
		}
		return s;
	}

	async function result(recipe, order) {
		const d = await load();
		const file = inputs[0];
		const prep = d.prepared[setup.files[0]];
		const src = base + (prep?.url || "");
		switch (recipe.do) {
			case "rendered": {
				const blob = saved.get(recipe.file);
				const ext = recipe.to || (setup.kind === "code" ? "png" : file.ext === "png" ? "png" : file.ext);
				return { name: named(file, recipe.suffix, ext), blob, detail: size(blob.size) };
			}
			case "crop_image": {
				const img = await picture(src);
				const c = canvas(recipe.w, recipe.h);
				c.getContext("2d").drawImage(img, -recipe.x, -recipe.y);
				const blob = await blobOf(c, "image/jpeg", 0.92);
				return { name: named(file, "cropped", "jpg"), blob, detail: `${c.width} × ${c.height} · ${size(blob.size)}` };
			}
			case "resize_image": {
				const img = await picture(src);
				let w = recipe.width;
				let h = recipe.height;
				if (recipe.percent) {
					w = (img.naturalWidth * recipe.percent) / 100;
					h = (img.naturalHeight * recipe.percent) / 100;
				} else if (w && !h) h = (img.naturalHeight * w) / img.naturalWidth;
				else if (h && !w) w = (img.naturalWidth * h) / img.naturalHeight;
				const scale = Math.min(w / img.naturalWidth, h / img.naturalHeight);
				const c = canvas(img.naturalWidth * scale, img.naturalHeight * scale);
				const x = c.getContext("2d");
				x.imageSmoothingQuality = "high";
				x.drawImage(img, 0, 0, c.width, c.height);
				const blob = await blobOf(c, "image/jpeg", 0.92);
				const tag = recipe.percent ? `${recipe.percent}%` : recipe.width ? String(recipe.width) : "resized";
				return { name: named(file, tag, "jpg"), blob, detail: `${c.width} × ${c.height} · ${size(blob.size)}` };
			}
			case "compress_image": {
				const img = await picture(src);
				const k = recipe.max_edge ? Math.min(1, recipe.max_edge / Math.max(img.naturalWidth, img.naturalHeight)) : 1;
				const c = canvas(img.naturalWidth * k, img.naturalHeight * k);
				c.getContext("2d").drawImage(img, 0, 0, c.width, c.height);
				let blob;
				if (recipe.target_kb) {
					// the closest quality that fits, as the app searches for it
					let lo = 0.05;
					let hi = 0.98;
					for (let i = 0; i < 8; i++) {
						const mid = (lo + hi) / 2;
						const b = await blobOf(c, "image/jpeg", mid);
						if (b.size <= recipe.target_kb * 1024) {
							blob = b;
							lo = mid;
						} else hi = mid;
					}
					blob ||= await blobOf(c, "image/jpeg", 0.05);
				} else blob = await blobOf(c, "image/jpeg", (recipe.quality || 78) / 100);
				return { name: named(file, "small", "jpg"), blob, detail: change(file.size, blob.size) };
			}
			case "strip_metadata": {
				const blob = await stripJpeg(src, recipe.location_only);
				return { name: named(file, "clean", "jpg"), blob, detail: recipe.location_only ? "Location removed, every pixel kept" : "Every detail removed, every pixel kept" };
			}
			case "combine": {
				const imgs = await Promise.all((order || [0, 1]).map((i) => picture(base + d.prepared[setup.files[i]].url)));
				if (recipe.layout === "gif") return { note: "Saves Combined.gif, one picture after another" };
				const gap = recipe.gap ?? 16;
				const h0 = Math.min(...imgs.map((i) => i.naturalHeight), 1200);
				const w0 = Math.min(...imgs.map((i) => i.naturalWidth), 1600);
				const sizes = imgs.map((i) => (recipe.layout === "stacked" ? [w0, (i.naturalHeight * w0) / i.naturalWidth] : [(i.naturalWidth * h0) / i.naturalHeight, h0]));
				const stacked = recipe.layout === "stacked";
				const W = stacked ? w0 + gap * 2 : sizes.reduce((a, s) => a + s[0], 0) + gap * (imgs.length + 1);
				const H = stacked ? sizes.reduce((a, s) => a + s[1], 0) + gap * (imgs.length + 1) : h0 + gap * 2;
				const c = canvas(W, H);
				const x = c.getContext("2d");
				x.fillStyle = recipe.background || "#ffffff";
				x.fillRect(0, 0, W, H);
				let at = gap;
				imgs.forEach((img, i) => {
					const [w, h] = sizes[i];
					if (stacked) x.drawImage(img, gap, at, w, h);
					else x.drawImage(img, at, gap, w, h);
					at += (stacked ? h : w) + gap;
				});
				const blob = await blobOf(c, "image/png");
				return { name: "Combined.png", blob, detail: `${c.width} × ${c.height} · ${size(blob.size)}` };
			}
			case "crop_video":
				return { note: `Saves ${named(file, "cropped", "mp4")} at ${recipe.w} × ${recipe.h}, with the sound kept` };
			case "trim": {
				const parts = recipe.keep.map((k) => `${time(k.start)} to ${time(k.end)}`);
				const ext = file.ext;
				const what = recipe.separate && recipe.keep.length > 1 ? `a folder of ${recipe.keep.length} parts` : named(file, "trimmed", ext);
				return { note: `Saves ${what}, keeping ${parts.join(", ")}${setup.kind === "audio" ? ", cut exactly where you set it" : recipe.exact ? ", cut on the exact frame" : ", cut at the nearest keyframe in an instant"}` };
			}
			case "make_gif":
				return { note: `Saves ${named(file, "", "gif")} from ${time(recipe.span.start)} to ${time(recipe.span.end)}, ${recipe.width} px wide at ${recipe.fps} frames a second` };
			case "tags": {
				const set = Object.entries(recipe.tags || {}).filter(([k, v]) => v && k !== "cover" && k !== "clear_cover");
				return { note: `Saves ${named(file, "tagged", file.ext)} with ${set.map(([k, v]) => `${k} "${v}"`).join(", ") || "its tags cleared"}. The sound isn't touched.` };
			}
			case "protect":
				return { note: `Saves album.${recipe.format === "7z" ? "7z" : "zip"} locked with AES-256${recipe.format === "7z" ? ", file names hidden too" : ""}` };
			case "extract": {
				const n = recipe.only?.length;
				return { note: n ? `Unpacks the ${n === 1 ? "item" : `${n} items`} you ticked into a folder named album` : `Unpacks ${d.prepared.album.items.filter((i) => !i.dir).length} files into a folder named album` };
			}
			case "snapshot":
				return { note: `Saves ${named(file, "frame", "png")}, the frame at ${time(recipe.at.at ?? 0)}, at full size` };
			default:
				return { note: "Done" };
		}
	}

	// what File info shows for a sample, worked out the way the app does
	function details(f, p) {
		const ext = f.ext.toUpperCase() === "JPEG" ? "JPG" : f.ext.toUpperCase();
		const day = Date.UTC(2026, 9, 12, 9, 41) / 1000;
		return { name: f.name, folder: "C:\\Users\\you\\Pictures", size: f.size, kind: setup.kind, named: ext, actual: ext, matches: true, created: day, modified: day + 3600, items: null, unpacked: null, width: p.width || null, height: p.height || null, duration: null, video: null, audio: null, fps: null, lines: null, words: null };
	}

	// Rename's preview, following crates/core/src/files.rs
	const RESERVED = /^(con|prn|aux|nul|com\d|lpt\d)$/i;
	const SMALL = new Set(["a", "an", "and", "as", "at", "but", "by", "for", "in", "of", "on", "or", "the", "to", "vs", "via"]);
	const cased = (s, c) =>
		c === "upper" ? s.toUpperCase()
		: c === "lower" ? s.toLowerCase()
		: c === "title" ? s.split(" ").map((w, i) => (i && SMALL.has(w.toLowerCase()) ? w.toLowerCase() : w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())).join(" ")
		: c === "sentence" ? s.charAt(0).toUpperCase() + s.slice(1).toLowerCase()
		: s;
	function renamePlan(rule) {
		const many = inputs.length > 1;
		const plan = inputs.map((f, i) => {
			const dot = f.name.lastIndexOf(".");
			const ext = dot > 0 ? f.name.slice(dot) : "";
			let st = rule.base || (dot > 0 ? f.name.slice(0, dot) : f.name);
			if (rule.find) st = st.split(rule.find).join(rule.replace || "");
			st = `${rule.prefix || ""}${cased(st, rule.case)}${rule.suffix || ""}`;
			if (rule.number || (many && rule.base)) st += ` ${String((rule.start || 0) + i).padStart(Math.max(1, rule.pad || 1), "0")}`;
			const to = st + ext;
			const bad = /[<>:"/\\|?*\x00-\x1f]/.exec(to);
			const problem = !st.trim() ? "the name is empty" : bad ? `names can't contain ${bad[0]}` : /[. ]$/.test(to) ? "names can't end with a dot or a space" : RESERVED.test(st) ? `Windows keeps ${st} for itself` : null;
			return { from: f.name, to, problem };
		});
		const seen = new Set();
		for (const r of plan) {
			if (r.problem) continue;
			const k = r.to.toLowerCase();
			if (seen.has(k)) r.problem = "another file gets this name too";
			seen.add(k);
		}
		return plan;
	}

	async function apply(msg) {
		try {
			const r = await result(msg.recipe, msg.order);
			const url = r.blob ? URL.createObjectURL(r.blob) : null;
			tell({ type: "result", name: r.name || null, detail: r.detail || null, note: r.note || null, url, image: r.blob ? r.blob.type.startsWith("image/") : false });
		} catch (e) {
			receive({ type: "error", message: `That didn't work: ${e.message || e}` });
		}
	}

	window.ipc = {
		postMessage(text) {
			const msg = JSON.parse(text);
			if (msg.cmd === "ready") ready();
			else if (msg.cmd === "apply") apply(msg);
			else if (msg.cmd === "close") tell({ type: "close" });
			else if (msg.cmd === "renamePreview") receive({ type: "renamePlan", items: renamePlan(msg.rule) });
			else if (msg.cmd === "rename") {
				const done = renamePlan(msg.rule).filter((r) => r.from !== r.to);
				tell({ type: "result", name: done.length === 1 ? `Renamed to ${done[0].to}` : `Renamed ${done.length} items`, note: done.map((r) => `${r.from} to ${r.to}`).join(", ") });
			} else if (msg.cmd === "copyText") {
				navigator.clipboard?.writeText(msg.text).then(
					() => receive({ type: "toast", message: `Copied ${msg.what || "it"}` }),
					() => receive({ type: "error", message: "The browser didn't allow copying here" }),
				);
			}
		},
	};

	// the page around the frame can change the look without reloading
	addEventListener("message", (e) => {
		if (e.origin !== location.origin || e.data?.source !== "tapwheel-site") return;
		if (e.data.type === "theme") {
			Object.assign(theme, e.data.theme);
			receive({ type: "theme", theme });
		}
	});
})();
