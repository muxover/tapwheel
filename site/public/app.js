// The hero demo: drag the photo, tap a key, drop it on the wheel.
(() => {
	const stage = document.getElementById("stage");
	const file = document.getElementById("file");
	const svg = document.getElementById("wheel");
	const toast = document.getElementById("toast");
	const hint = document.getElementById("hint");
	if (!stage || !file || !svg) return;

	const NS = "http://www.w3.org/2000/svg";
	const R_OUT = 140;
	const R_IN = 54;
	const WHEELS = {
		Shift: {
			title: "Convert",
			items: [
				["JPG", "Converted to JPG", "jpg", "JPG"],
				["PNG", "Converted to PNG", "png", "PNG"],
				["WebP", "Converted to WebP", "webp", "WEBP"],
				["AVIF", "Converted to AVIF", "avif", "AVIF"],
				["GIF", "Converted to GIF", "gif", "GIF"],
				["TIFF", "Converted to TIFF", "tiff", "TIFF"],
				["BMP", "Converted to BMP", "bmp", "BMP"],
				["ICO", "Converted to ICO", "ico", "ICO"],
			],
		},
		Control: {
			title: "Tools",
			items: [
				["Compress", "Smaller copy saved", "jpg", "JPG", "-small"],
				["Half size", "Half-size copy saved", "jpg", "JPG", "-half"],
				["Fit 1920", "Resized copy saved", "jpg", "JPG", "-1920"],
				["Rotate", "Rotated copy saved", "jpg", "JPG", "-rotated"],
				["Strip data", "Location and camera data removed", "jpg", "JPG", "-clean"],
				["ZIP", "Packed into a ZIP", "zip", "ZIP"],
			],
		},
		Alt: {
			title: "Actions",
			items: [
				["Copy path", "Path copied"],
				["Copy", "Copied, ready to paste"],
				["Terminal", "A terminal would open here"],
				["Show", "Shown in File Explorer"],
			],
		},
	};
	const touch = window.matchMedia("(pointer: coarse)").matches;
	const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
	if (touch) hint.textContent = "Drag the photo and hold still for a moment";

	let drag = null;
	let wheel = null;
	let held = null;
	let still = null;
	let made = 0;

	const el = (name, attrs) => {
		const n = document.createElementNS(NS, name);
		for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v);
		return n;
	};

	const arc = (r0, r1, a0, a1) => {
		const p = (r, a) => `${150 + r * Math.cos(a)} ${150 + r * Math.sin(a)}`;
		const large = a1 - a0 > Math.PI ? 1 : 0;
		return `M${p(r1, a0)}A${r1} ${r1} 0 ${large} 1 ${p(r1, a1)}L${p(r0, a1)}A${r0} ${r0} 0 ${large} 0 ${p(r0, a0)}Z`;
	};

	function draw() {
		svg.replaceChildren();
		const { key, hover } = wheel;
		const items = WHEELS[key].items;
		const n = items.length;
		const step = (Math.PI * 2) / n;
		svg.append(el("circle", { cx: 150, cy: 150, r: R_OUT + 6, fill: "#1e2128", stroke: "#3a3f47" }));
		items.forEach((item, i) => {
			const mid = -Math.PI / 2 + step * i;
			if (hover === i) {
				svg.append(el("path", { d: arc(R_IN + 4, R_OUT, mid - step / 2 + 0.02, mid + step / 2 - 0.02), fill: "#b6f23a" }));
			}
			const a = mid - step / 2;
			svg.append(
				el("line", {
					x1: 150 + (R_IN + 12) * Math.cos(a),
					y1: 150 + (R_IN + 12) * Math.sin(a),
					x2: 150 + (R_OUT - 12) * Math.cos(a),
					y2: 150 + (R_OUT - 12) * Math.sin(a),
					stroke: "#2c3038",
				}),
			);
			const r = (R_IN + R_OUT) / 2 + 2;
			const t = el("text", {
				x: 150 + r * Math.cos(mid),
				y: 150 + r * Math.sin(mid),
				"text-anchor": "middle",
				"dominant-baseline": "central",
				fill: hover === i ? "#0e0f12" : "#f4f3ef",
				"font-size": 13,
				"font-weight": 600,
				"font-family": "Geist, Segoe UI, sans-serif",
			});
			t.textContent = item[0];
			svg.append(t);
		});
		svg.append(el("circle", { cx: 150, cy: 150, r: R_IN - 6, fill: hover === "center" ? "#2a2e36" : "#262a31" }));
		const center = (text, y, size, color, weight) => {
			const t = el("text", { x: 150, y, "text-anchor": "middle", fill: color, "font-size": size, "font-weight": weight, "font-family": "Geist, Segoe UI, sans-serif" });
			t.textContent = text;
			svg.append(t);
		};
		if (typeof hover === "number") {
			center(items[hover][0], 155, 15, "#f4f3ef", 600);
		} else if (hover === "center") {
			center("Cancel", 155, 15, "#f4f3ef", 600);
		} else {
			center(WHEELS[key].title, 148, 15, "#f4f3ef", 600);
			center("IMG_2041.heic", 166, 10.5, "#8c9099", 400);
		}
	}

	function open(key, x, y) {
		const box = stage.getBoundingClientRect();
		const cx = Math.min(Math.max(x, 150), box.width - 150);
		const cy = Math.min(Math.max(y, 150), box.height - 150);
		wheel = { key, hover: null, cx, cy };
		svg.style.left = `${cx}px`;
		svg.style.top = `${cy}px`;
		draw();
		svg.classList.add("open");
		hint.textContent = touch ? "Slide onto a slice and let go" : "Drop on a slice, or in the middle to cancel";
	}

	function close() {
		wheel = null;
		svg.classList.remove("open");
	}

	function hoverAt(x, y) {
		if (!wheel) return;
		const dx = x - wheel.cx;
		const dy = y - wheel.cy;
		const d = Math.hypot(dx, dy);
		const n = WHEELS[wheel.key].items.length;
		let h = null;
		if (d < R_IN) h = "center";
		else if (d < R_OUT + 16) {
			const step = (Math.PI * 2) / n;
			const a = (Math.atan2(dy, dx) + Math.PI / 2 + step / 2 + Math.PI * 4) % (Math.PI * 2);
			h = Math.min(n - 1, Math.floor(a / step));
		}
		if (h !== wheel.hover) {
			wheel.hover = h;
			draw();
		}
	}

	function say(title, body) {
		document.getElementById("toast-title").textContent = title;
		document.getElementById("toast-body").textContent = body;
		toast.classList.add("show");
		clearTimeout(say.t);
		say.t = setTimeout(() => toast.classList.remove("show"), 2600);
	}

	function result(item) {
		const [, done, ext, label, suffix] = item;
		if (!ext) {
			say(done, "IMG_2041.heic");
			return;
		}
		const name = ext === "zip" ? "IMG_2041.zip" : `IMG_2041${suffix || ""}.${ext}`;
		const tile = document.createElement("div");
		tile.className = "tile new" + (reduced ? "" : " appear");
		const slot = made % 3;
		made += 1;
		tile.style.left = `${32 + (slot + 1) * 124}px`;
		tile.style.top = "32px";
		tile.innerHTML = '<div class="icon"></div><span></span>';
		tile.querySelector(".icon").dataset.ext = label;
		tile.querySelector("span").textContent = name;
		const old = stage.querySelector(`.tile.new[data-slot="${slot}"]`);
		if (old) old.remove();
		tile.dataset.slot = slot;
		stage.append(tile);
		say(done, `${name}, next to the original`);
	}

	function finish() {
		const choice = wheel && typeof wheel.hover === "number" ? WHEELS[wheel.key].items[wheel.hover] : null;
		close();
		file.style.left = "32px";
		file.style.top = "32px";
		file.classList.remove("dragging");
		if (choice) result(choice);
		hint.textContent = choice ? "Try another one, or tap Ctrl for tools" : touch ? "Drag the photo and hold still for a moment" : "Drag the photo, then tap Shift";
	}

	const local = (e) => {
		const b = stage.getBoundingClientRect();
		return [e.clientX - b.left, e.clientY - b.top];
	};

	file.addEventListener("pointerdown", (e) => {
		if (e.button !== 0) return;
		e.preventDefault();
		file.setPointerCapture(e.pointerId);
		const [x, y] = local(e);
		drag = { dx: x - file.offsetLeft, dy: y - file.offsetTop, moved: false };
	});

	file.addEventListener("pointermove", (e) => {
		if (!drag) return;
		const [x, y] = local(e);
		if (!drag.moved && Math.hypot(x - drag.dx - 32, y - drag.dy - 32) < 4) return;
		drag.moved = true;
		file.classList.add("dragging");
		file.style.left = `${x - drag.dx}px`;
		file.style.top = `${y - drag.dy}px`;
		if (wheel) hoverAt(x, y);
		else if (touch) {
			clearTimeout(still);
			still = setTimeout(() => drag && !wheel && open("Shift", x, y), 450);
		}
		drag.x = x;
		drag.y = y;
		if (!wheel && !touch) hint.innerHTML = "Now tap <kbd>Shift</kbd>, <kbd>Ctrl</kbd> or <kbd>Alt</kbd>";
	});

	const end = () => {
		clearTimeout(still);
		if (!drag) return;
		drag = null;
		finish();
	};
	file.addEventListener("pointerup", end);
	file.addEventListener("pointercancel", end);

	window.addEventListener("keydown", (e) => {
		if (!(e.key in WHEELS)) {
			held = null;
			if (e.key === "Escape" && wheel) {
				wheel.hover = null;
				finish();
			}
			return;
		}
		if ((drag && drag.moved) || wheel) e.preventDefault();
		if (!e.repeat) held = { key: e.key, at: performance.now() };
	});

	window.addEventListener("keyup", (e) => {
		if (!held || held.key !== e.key || performance.now() - held.at > 400) return;
		held = null;
		if (wheel) {
			wheel.key = e.key;
			wheel.hover = null;
			draw();
		} else if (drag && drag.moved) {
			open(e.key, drag.x, drag.y);
		}
	});

	// keyboard path: Enter on the photo opens the wheel, arrows pick, Enter drops
	file.addEventListener("keydown", (e) => {
		if (e.key === "Enter" && !wheel) {
			e.preventDefault();
			open("Shift", file.offsetLeft + 56, file.offsetTop + 120);
			wheel.hover = 0;
			draw();
			return;
		}
		if (!wheel) return;
		const n = WHEELS[wheel.key].items.length;
		const cur = typeof wheel.hover === "number" ? wheel.hover : 0;
		if (e.key === "ArrowRight" || e.key === "ArrowDown") wheel.hover = (cur + 1) % n;
		else if (e.key === "ArrowLeft" || e.key === "ArrowUp") wheel.hover = (cur + n - 1) % n;
		else if (e.key === "Enter") return finish();
		else return;
		e.preventDefault();
		draw();
	});
})();
