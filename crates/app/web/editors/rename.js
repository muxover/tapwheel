// Rename one file or many, with every new name shown before anything changes.

import { group, h, on, segmented, send, shell, stem, toggle, numberInput } from "../tw.js";

export default function rename(init) {
	const files = init.files;
	const many = files.length > 1;
	const rule = { base: many ? "" : stem(files[0].name), find: "", replace: "", case: null, prefix: "", suffix: "", number: false, start: 1, pad: 2 };
	const field = (key, label, placeholder = "") => {
		const el = h("input", { type: "text", value: rule[key], placeholder, "aria-label": label, spellcheck: "false" });
		el.addEventListener("input", () => {
			rule[key] = el.value;
			ask();
		});
		return el;
	};
	const base = field("base", "New name", many ? "Keep each name" : "");
	const numbering = h("div", { class: "row" },
		h("label", { class: "field" }, h("span", { class: "hint" }, "Start at"), numberInput(rule.start, { min: 0, max: 99999, label: "Start at", onchange: (v) => ((rule.start = v), ask()) })),
		h("label", { class: "field" }, h("span", { class: "hint" }, "Digits"), numberInput(rule.pad, { min: 1, max: 6, label: "Digits", onchange: (v) => ((rule.pad = v), ask()) })),
	);
	const body = h("tbody");
	const ui = shell({
		title: "Rename",
		lead: many ? `${files.length} items. Extensions stay as they are.` : "The extension stays as it is.",
		primary: "Rename",
		panel: [
			group("Name", base),
			group("Find and replace", field("find", "Find", "Find"), field("replace", "Replace with", "Replace with")),
			group("Add", h("div", { class: "row" }, field("prefix", "Before", "Before"), field("suffix", "After", "After"))),
			group("Case", segmented([
				{ label: "Keep", value: "" },
				{ label: "Title", value: "title" },
				{ label: "lower", value: "lower" },
				{ label: "UPPER", value: "upper" },
			], "", (c) => ((rule.case = c || null), ask()))),
			group("Numbers", toggle("Number them", rule.number, (on) => ((rule.number = on), ask()), many ? "Added on its own when every file gets the same name" : ""), numbering),
		],
		onSave: async () => {
			if (!ok) throw new Error(problem || "Nothing to rename");
			send("rename", { rule: clean() });
		},
	});
	ui.stage.style.placeItems = "stretch";
	ui.stage.replaceChildren(h("div", { class: "sheet" }, h("table", {}, h("thead", {}, h("tr", {}, h("th", {}, "Now"), h("th", {}, "Becomes"))), body)));
	let ok = false;
	let problem = "";
	const clean = () => ({ ...rule, base: rule.base.trim() ? rule.base : null });
	let timer = 0;
	function ask() {
		clearTimeout(timer);
		timer = setTimeout(() => send("renamePreview", { rule: clean() }), 60);
	}
	on("renamePlan", (msg) => {
		const items = msg.items;
		const changed = items.filter((i) => i.from !== i.to).length;
		const bad = items.find((i) => i.problem);
		problem = bad ? `${bad.from}: ${bad.problem}` : "";
		ok = !bad && changed > 0;
		body.replaceChildren(...items.slice(0, 2000).map((i) =>
			h("tr", {},
				h("td", { class: "dim" }, i.from),
				h("td", { class: i.problem ? "bad" : i.from === i.to ? "same" : "" }, i.problem ? `${i.to} (${i.problem})` : i.to),
			),
		));
		ui.setOut(bad ? "Fix the names in red first" : changed ? `Renames <b>${changed}</b> of ${items.length}` : "Nothing changes yet");
		ui.save.disabled = !ok;
	});
	ask();
	base.focus();
	base.select();
}
