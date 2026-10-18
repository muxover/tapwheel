// Backdrops shared by Add background, Code shot and Combine. Same names as the app's own.

export const GRADIENTS = {
	sunset: ["#ff6b6b", "#ffa94d", "#845ef7"],
	ocean: ["#22d3ee", "#6366f1"],
	lime: ["#b6f23a", "#10b981"],
	grape: ["#a855f7", "#ec4899"],
	dusk: ["#1e293b", "#4c1d95"],
	sand: ["#fde68a", "#fb923c"],
	mist: ["#e2e8f0", "#cbd5e1"],
	ink: ["#18181b", "#27272a"],
};

export const SOLIDS = ["#ffffff", "#f5f5f4", "#facc15", "#84cc16", "#14b8a6", "#38bdf8", "#6366f1", "#18181b"];

export function cssFor(name) {
	const g = GRADIENTS[name];
	if (g) return `linear-gradient(135deg, ${g.join(", ")})`;
	if (name === "none") return "repeating-conic-gradient(#ccc 0 25%, #fff 0 50%) 0 0 / 12px 12px";
	if (name === "blur") return "linear-gradient(135deg, #9ca3af, #4b5563)";
	return name;
}

/** Fills a canvas area with a named backdrop. */
export function paintBackdrop(ctx, name, w, h, source) {
	const g = GRADIENTS[name];
	if (g) {
		const grad = ctx.createLinearGradient(0, 0, w, h);
		g.forEach((c, i) => grad.addColorStop(i / (g.length - 1), c));
		ctx.fillStyle = grad;
		ctx.fillRect(0, 0, w, h);
	} else if (name === "blur" && source) {
		// the picture itself, blown up and blurred, behind the picture
		const s = Math.max(w / source.width, h / source.height) * 1.15;
		ctx.save();
		ctx.filter = `blur(${Math.round(Math.max(w, h) / 30)}px) saturate(1.2)`;
		ctx.drawImage(source, (w - source.width * s) / 2, (h - source.height * s) / 2, source.width * s, source.height * s);
		ctx.restore();
		ctx.fillStyle = "rgba(0,0,0,0.08)";
		ctx.fillRect(0, 0, w, h);
	} else if (name && name !== "none") {
		ctx.fillStyle = name;
		ctx.fillRect(0, 0, w, h);
	}
}

/** Rounded rectangle path. */
export function roundRect(ctx, x, y, w, h, r) {
	r = Math.min(r, w / 2, h / 2);
	ctx.beginPath();
	ctx.moveTo(x + r, y);
	ctx.arcTo(x + w, y, x + w, y + h, r);
	ctx.arcTo(x + w, y + h, x, y + h, r);
	ctx.arcTo(x, y + h, x, y, r);
	ctx.arcTo(x, y, x + w, y, r);
	ctx.closePath();
}

import { h } from "../tw.js";

/** The picker: gradients, a blur of the picture, plain colours, a custom colour, or nothing. */
export function backdropPicker(value, onchange, { blur = true } = {}) {
	const names = [...Object.keys(GRADIENTS), ...(blur ? ["blur"] : []), "none", ...SOLIDS];
	const buttons = [];
	const pick = (n) => {
		for (const b of buttons) b.setAttribute("aria-pressed", String(b.dataset.v === n));
		onchange(n);
	};
	for (const n of names) {
		const label = n === "blur" ? "Blur of the picture" : n === "none" ? "No background" : n;
		const b = h("button", { class: "swatch", type: "button", title: label, "aria-label": label, "data-v": n, "aria-pressed": String(n === value), style: { background: cssFor(n) } });
		b.addEventListener("click", () => pick(n));
		buttons.push(b);
	}
	const custom = h("input", { type: "color", value: "#3b82f6", "aria-label": "Custom colour", style: { width: "100%", height: "32px", padding: "0", border: "0", background: "transparent" } });
	custom.addEventListener("input", () => pick(custom.value));
	return h("div", {}, h("div", { class: "swatches" }, buttons), h("div", { class: "row", style: { marginTop: "8px" } }, h("span", { class: "hint", style: { margin: 0, flex: 1 } }, "Any colour"), h("div", { style: { width: "64px" } }, custom)));
}
