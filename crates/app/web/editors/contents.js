// What's inside an archive, without unpacking it.

import { apply, formatSize, group, h, prepared, shell, stageLoading, stem } from "../tw.js";

export default async function contents(init) {
	const file = init.files[0];
	const search = h("input", { type: "search", placeholder: "Find a file", "aria-label": "Find a file" });
	const summary = h("div", { class: "hint" });
	const ui = shell({
		title: "Contents",
		lead: file.name,
		primary: "Extract all",
		panel: [group("Search", search), group("Totals", summary)],
		onSave: async () => apply({ do: "extract" }),
	});
	ui.setOut(`Extracts into a folder named <b>${stem(file.name)}</b> next to it`);
	stageLoading(ui.stage, "Reading the archive");
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const items = info.items || [];
	const filesOnly = items.filter((i) => !i.dir);
	summary.textContent = `${filesOnly.length} files, ${formatSize(filesOnly.reduce((a, i) => a + i.size, 0))} unpacked`;
	const body = h("tbody");
	const table = h("table", { style: { width: "100%", borderCollapse: "collapse", fontVariantNumeric: "tabular-nums" } },
		h("thead", {}, h("tr", { style: { textAlign: "left", color: "var(--text-2)" } }, h("th", { style: { padding: "8px" } }, "Name"), h("th", { style: { padding: "8px", width: "110px" } }, "Size"), h("th", { style: { padding: "8px", width: "170px" } }, "Modified"))),
		body,
	);
	const box = h("div", { style: { position: "absolute", inset: "0", overflow: "auto", padding: "8px 12px", userSelect: "text" } }, table);
	ui.stage.style.placeItems = "stretch";
	ui.stage.replaceChildren(box);
	const fmt = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });
	function show(q) {
		const list = items.filter((i) => !q || i.name.toLowerCase().includes(q)).slice(0, 5000);
		body.replaceChildren(
			...list.map((i) =>
				h("tr", { style: { borderTop: "1px solid var(--stroke)" } },
					h("td", { style: { padding: "6px 8px", wordBreak: "break-all" } }, i.dir ? `${i.name}` : i.name),
					h("td", { style: { padding: "6px 8px", color: "var(--text-2)" } }, i.dir ? "Folder" : formatSize(i.size)),
					h("td", { style: { padding: "6px 8px", color: "var(--text-2)" } }, i.modified ? fmt.format(new Date(i.modified * 1000)) : ""),
				),
			),
		);
	}
	search.addEventListener("input", () => show(search.value.trim().toLowerCase()));
	show("");
}
