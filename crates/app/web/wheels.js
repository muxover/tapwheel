// The Wheels page in Settings: arrange each wheel for each kind of file (pin, hide, move), and
// pick the wheel's size and colour. Layouts are kept per wheel and kind, as the app reads them.

import { group, h, iconEl, segmented } from "./tw.js";

const WHEEL_NAMES = { convert: "Convert", tools: "Tools", actions: "Actions" };
const KIND_NAMES = { image: "Pictures", video: "Video", audio: "Audio", icon: "Icons", text: "Text", code: "Code", archive: "Archives" };
const SIZES = [
	{ label: "Small", value: "small" },
	{ label: "Normal", value: "normal" },
	{ label: "Large", value: "large" },
];
// Windows' own accent swatches, so a picked colour still looks at home
const COLOURS = ["#0078d4", "#0099bc", "#00b294", "#10893e", "#ca5010", "#e81123", "#c30052", "#8764b8", "#6b69d6", "#515c6b"];

/**
 * @param s the settings being edited
 * @param catalog every slice each wheel can show, by kind: [{ key, wheel, kind, slices: [{ id, label, icon }] }]
 * @param save stores the settings
 */
export function wheelsPage(s, catalog, save) {
	s.wheels ||= {};
	let wheel = "convert";
	let kind = "image";
	const list = h("div", { class: "slices", role: "list" });
	const resetOne = h("button", { class: "btn", type: "button" }, "Reset this wheel");

	const layout = () => s.wheels[`${wheel}/${kind}`] || { order: [], pinned: [], hidden: [] };
	const entry = () => catalog.find((c) => c.wheel === wheel && c.kind === kind);

	/** The slices as the wheel shows them: pinned first, then the hand-set order or the app's. */
	function arranged() {
		const l = layout();
		const all = entry()?.slices || [];
		const place = (ids, id) => {
			const i = ids.indexOf(id);
			return i < 0 ? Infinity : i;
		};
		const byOrder = l.order.length ? [...all].sort((a, b) => place(l.order, a.id) - place(l.order, b.id)) : [...all];
		return byOrder.sort((a, b) => place(l.pinned, a.id) - place(l.pinned, b.id));
	}

	function change(fn) {
		const l = structuredClone(layout());
		fn(l);
		const empty = !l.order.length && !l.pinned.length && !l.hidden.length;
		if (empty) delete s.wheels[`${wheel}/${kind}`];
		else s.wheels[`${wheel}/${kind}`] = l;
		save();
		draw();
	}

	function move(id, by) {
		change((l) => {
			const ids = arranged().map((x) => x.id);
			const i = ids.indexOf(id);
			const j = i + by;
			if (j < 0 || j >= ids.length) return;
			[ids[i], ids[j]] = [ids[j], ids[i]];
			l.order = ids;
			// a pinned slice stays among the pinned ones, in the new order
			l.pinned = ids.filter((x) => l.pinned.includes(x));
		});
	}

	function draw() {
		const l = layout();
		const items = arranged();
		resetOne.disabled = !s.wheels[`${wheel}/${kind}`];
		list.replaceChildren(
			...items.map((x, i) => {
				const pinned = l.pinned.includes(x.id);
				const hidden = l.hidden.includes(x.id);
				const flag = (icon, label, on, fn) =>
					h("button", { class: "btn icon subtle", type: "button", title: label, "aria-label": `${label}: ${x.label}`, "aria-pressed": String(on), onclick: fn }, iconEl(icon, 16));
				return h("div", { class: `slice${hidden ? " off" : ""}`, role: "listitem" },
					h("span", { class: "slice-icon" }, iconEl(x.icon || "document", 16)),
					h("span", { class: "slice-name" }, x.label, hidden ? h("span", { class: "hint" }, " hidden") : null),
					flag("pin", "Pin", pinned, () => change((c) => {
						c.pinned = pinned ? c.pinned.filter((p) => p !== x.id) : [...c.pinned, x.id];
						c.hidden = c.hidden.filter((p) => p !== x.id);
					})),
					flag("eye_off", "Hide", hidden, () => change((c) => {
						c.hidden = hidden ? c.hidden.filter((p) => p !== x.id) : [...c.hidden, x.id];
						c.pinned = c.pinned.filter((p) => p !== x.id);
					})),
					h("button", { class: "btn icon subtle up", type: "button", title: "Move up", "aria-label": `Move up: ${x.label}`, disabled: i === 0, onclick: () => move(x.id, -1) }, iconEl("chevron_down", 16)),
					h("button", { class: "btn icon subtle", type: "button", title: "Move down", "aria-label": `Move down: ${x.label}`, disabled: i === items.length - 1, onclick: () => move(x.id, 1) }, iconEl("chevron_down", 16)),
				);
			}),
		);
	}

	resetOne.addEventListener("click", () => {
		delete s.wheels[`${wheel}/${kind}`];
		save();
		draw();
	});
	const resetAll = h("button", { class: "btn", type: "button", onclick: () => {
		s.wheels = {};
		save();
		draw();
	} }, "Reset all wheels");

	const kinds = [...new Set(catalog.map((c) => c.kind))];
	draw();

	const custom = h("input", { type: "color", "aria-label": "Your own colour", value: s.accent || "#0078d4" });
	const swatches = [
		h("button", { class: "swatch windows", type: "button", "aria-label": "Windows accent", title: "Windows accent", "aria-pressed": String(!s.accent) }),
		...COLOURS.map((c) => h("button", { class: "swatch", type: "button", "aria-label": c, title: c, "aria-pressed": String(s.accent === c), style: { background: c } })),
	];
	const pickColour = (c) => {
		s.accent = c;
		swatches.forEach((b, i) => b.setAttribute("aria-pressed", String(i === 0 ? !c : COLOURS[i - 1] === c)));
		save();
	};
	swatches.forEach((b, i) => b.addEventListener("click", () => pickColour(i === 0 ? null : COLOURS[i - 1])));
	custom.addEventListener("change", () => pickColour(custom.value));

	const own = h("div", { class: "slices" });
	function drawOwn() {
		const list = s.own_choices || [];
		own.replaceChildren(
			...(list.length
				? list.map((c, i) =>
						h("div", { class: "slice" },
							h("span", { class: "slice-name" }, c.label, h("span", { class: "hint" }, ` ${c.op.replace(/_/g, " ")} · ${KIND_NAMES[c.kind] || "any file"}`)),
							h("button", { class: "btn icon subtle", type: "button", title: "Remove", "aria-label": `Remove: ${c.label}`, onclick: () => {
								s.own_choices = list.filter((_, j) => j !== i);
								save();
								drawOwn();
							} }, iconEl("delete", 16)),
						),
					)
				: [h("p", { class: "hint", style: { margin: "4px 0" } }, "None yet. In a tool's window, tick Keep as a ring choice before saving.")]),
		);
	}
	drawOwn();

	return h("div", {},
		group("Size", segmented(SIZES, s.wheel_size || "normal", (v) => {
			s.wheel_size = v;
			save();
		})),
		group("Colour", h("div", { class: "swatches compact" }, swatches), h("div", { class: "row", style: { marginTop: "10px" } }, h("span", { style: { flex: "1", color: "var(--text-2)" } }, "Your own colour"), custom), h("div", { class: "hint", style: { marginTop: "8px" } }, "Windows accent follows the colour set in Windows. High contrast always uses the system's colours.")),
		h("h3", { class: "section" }, "Arrange"),
		h("p", { class: "hint" }, "Pinned slices come first, in the order you set. Hidden ones leave the wheel. Each kind of file keeps its own arrangement."),
		group(null,
			segmented(Object.entries(WHEEL_NAMES).map(([value, label]) => ({ label, value })), wheel, (v) => {
				wheel = v;
				draw();
			}),
			h("div", { style: { height: "8px" } }),
			segmented(kinds.map((k) => ({ label: KIND_NAMES[k] || k, value: k })), kind, (v) => {
				kind = v;
				draw();
			}),
			h("div", { style: { height: "12px" } }),
			list,
			h("div", { class: "row", style: { marginTop: "12px", gap: "8px", justifyContent: "flex-end" } }, resetOne, resetAll),
		),
		h("h3", { class: "section" }, "Your ring choices"),
		h("p", { class: "hint" }, "Tool settings you kept, shown on the slice's ring before Custom. Each slice keeps up to three for each kind of file."),
		group(null, own),
	);
}
