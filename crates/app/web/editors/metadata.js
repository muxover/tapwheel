// Metadata: every detail a photo carries, with location and the rest removable without
// re-encoding; or a song's title, artist, album and cover.

import { apply, escapeHtml, group, h, iconEl, keepImageExt, keepMediaExt, outName, prepared, saveBlob, savesAs, segmented, shell, stageLoading, toggle } from "../tw.js";

export default async function metadata(init) {
	const file = init.files[0];
	return init.kind === "audio" ? tags(init, file) : photo(init, file);
}

async function photo(init, file) {
	let what = "all";
	const ui = shell({
		title: "Metadata",
		lead: "Photos can carry where they were taken, the camera and more.",
		primary: "Remove",
		panel: [
			group("Remove", segmented([{ label: "Everything", value: "all" }, { label: "Location only", value: "location" }], "all", (v) => (what = v))),
			h("div", { class: "hint" }, "JPG, PNG and WebP keep every pixel exactly as they are. The picture stays upright."),
		],
		onSave: async () => apply({ do: "strip_metadata", location_only: what === "location" }),
	});
	ui.setOut(savesAs(outName(file, "clean", keepImageExt(file.ext))));
	stageLoading(ui.stage, "Reading the details");
	const [info] = await prepared();
	if (info.error) throw new Error(info.error);
	const fields = info.fields || [];
	const sections = {};
	for (const [section, name, value] of fields) (sections[section] ||= []).push([name, value]);
	const order = ["Location", "Camera", "Picture"];
	const box = h("div", { style: { position: "absolute", inset: "0", overflow: "auto", padding: "16px 20px", userSelect: "text" } });
	if (!fields.length) box.append(h("div", { class: "loading", style: { height: "100%" } }, iconEl("shield_checkmark", 40), h("div", {}, "No hidden details in this picture")));
	if (info.location) box.append(h("div", { class: "group", style: { borderColor: "var(--accent)" } }, h("div", { class: "label" }, "This photo says where it was taken"), h("div", { class: "hint" }, "Anyone you send it to can see the place. Remove the location before sharing it.")));
	for (const s of order.filter((o) => sections[o])) {
		const rows = sections[s].map(([n, v]) => h("tr", {}, h("td", { style: { color: "var(--text-2)", padding: "4px 12px 4px 0", whiteSpace: "nowrap", verticalAlign: "top" } }, n), h("td", { style: { padding: "4px 0", wordBreak: "break-word" }, html: escapeHtml(v) })));
		box.append(h("section", { class: "group" }, h("div", { class: "label" }, s), h("table", { style: { borderCollapse: "collapse", width: "100%" } }, h("tbody", {}, rows))));
	}
	ui.stage.style.placeItems = "stretch";
	ui.stage.replaceChildren(box);
}

const FIELDS = [
	["title", "Title"],
	["artist", "Artist"],
	["album", "Album"],
	["album_artist", "Album artist"],
	["date", "Year"],
	["genre", "Genre"],
	["track", "Track"],
	["comment", "Comment"],
];

async function tags(init, file) {
	let cover = null;
	let clear = false;
	const inputs = {};
	const coverImg = h("div", { style: { width: "220px", height: "220px", borderRadius: "var(--radius)", background: "var(--surface)", display: "grid", placeItems: "center", overflow: "hidden", boxShadow: "var(--shadow)" } }, iconEl("music_note_2", 56));
	const pick = h("input", { type: "file", accept: "image/jpeg,image/png", style: { display: "none" } });
	pick.addEventListener("change", async () => {
		const f = pick.files[0];
		if (!f) return;
		cover = await saveBlob(f, f.type === "image/png" ? "png" : "jpg");
		coverImg.replaceChildren(h("img", { src: URL.createObjectURL(f), style: { width: "100%", height: "100%", objectFit: "cover" } }));
	});
	const ui = shell({
		title: "Song details",
		lead: "What music players show for this file.",
		panel: [
			group("Cover", h("button", { class: "btn", type: "button", onclick: () => pick.click() }, iconEl("image", 16), "Choose a picture"), pick),
			group(null, toggle("Remove everything else", false, (v) => (clear = v), "Drops tags not listed here, like lyrics or encoder notes.")),
		],
		onSave: async () => {
			const t = {};
			for (const [k] of FIELDS) {
				const v = inputs[k].value.trim();
				t[k === "date" ? "year" : k] = v || null;
			}
			apply({ do: "tags", tags: { ...t, cover, clear } });
		},
	});
	ui.setOut(savesAs(outName(file, "tagged", keepMediaExt(file.ext, false))));
	stageLoading(ui.stage, "Reading the tags");
	const [info] = await prepared();
	const now = info.tags || {};
	const form = h("div", { style: { display: "grid", gridTemplateColumns: "120px 1fr", gap: "8px 12px", alignItems: "center", width: "min(520px, 90%)" } });
	for (const [k, label] of FIELDS) {
		inputs[k] = h("input", { type: "text", value: now[k] || "", "aria-label": label });
		form.append(h("label", { style: { color: "var(--text-2)" } }, label), inputs[k]);
	}
	if (info.hasCover) coverImg.append(h("div", { class: "hint" }, "Has a cover"));
	ui.stage.replaceChildren(h("div", { style: { display: "grid", justifyItems: "center", gap: "20px" } }, coverImg, form));
}
