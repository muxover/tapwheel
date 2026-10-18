// What a file really is: its size, dates, what's inside, and whether its name tells the truth.

import { formatSize, formatTime, group, h, iconEl, prepared, send, shell, stageLoading } from "../tw.js";

export default async function info(init) {
	let text = "";
	const ui = shell({
		title: "File info",
		lead: init.files.length === 1 ? init.files[0].name : `${init.files.length} items`,
		primary: "Copy",
		onSave: async () => {
			send("copyText", { text, what: "the details" });
			ui.busy(false);
		},
	});
	stageLoading(ui.stage, "Reading the file");
	const all = await prepared();
	const fmt = new Intl.DateTimeFormat(undefined, { dateStyle: "long", timeStyle: "short" });
	const body = h("tbody");
	const lines = [];
	const row = (k, v) => {
		if (v === null || v === undefined || v === "") return;
		lines.push(`${k}: ${v}`);
		body.append(h("tr", {}, h("th", { class: "key", scope: "row" }, k), h("td", {}, String(v))));
	};
	for (const f of all) {
		const i = f.info;
		if (all.length > 1) {
			lines.push("");
			body.append(h("tr", { class: "section" }, h("th", { colspan: 2 }, init.files[f.index].name)));
		}
		if (f.error || !i) {
			row("Problem", f.error || "Couldn't read it");
			continue;
		}
		row("Name", i.name);
		row("Folder", i.folder);
		row("Size", i.size >= 1024 ? `${formatSize(i.size)} (${i.size.toLocaleString()} bytes)` : formatSize(i.size));
		row("Type", i.matches ? i.actual || i.named : `${i.actual}, though the name says ${i.named}`);
		if (i.width) row("Dimensions", `${i.width} × ${i.height}`);
		if (i.duration) row("Length", formatTime(i.duration).replace(/\.\d$/, ""));
		row("Video", i.video);
		if (i.fps) row("Frame rate", `${Math.round(i.fps * 100) / 100} fps`);
		row("Audio", i.audio);
		if (i.items !== null) row("Items", i.items.toLocaleString());
		if (i.unpacked) row("Unpacked", formatSize(i.unpacked));
		if (i.lines !== null) row("Lines", i.lines.toLocaleString());
		if (i.words !== null) row("Words", i.words.toLocaleString());
		if (i.created) row("Created", fmt.format(new Date(i.created * 1000)));
		if (i.modified) row("Modified", fmt.format(new Date(i.modified * 1000)));
	}
	text = lines.join("\n").trim();
	const odd = all.filter((f) => f.info && !f.info.matches);
	ui.side.append(odd.length
		? group("Check the name", h("p", { class: "hint" }, odd.length === 1 && all.length === 1
			? `It's really a ${odd[0].info.actual} file. Some apps open it anyway; Convert can make it a real ${odd[0].info.named}.`
			: `${odd.length} of these aren't what their names say. Convert can make them match.`))
		: h("div", { class: "info-icon" }, iconEl("document_search", 64)));
	ui.stage.style.placeItems = "stretch";
	ui.stage.replaceChildren(h("div", { class: "sheet" }, h("table", {}, body)));
	ui.setOut("Copies these details as text");
}
