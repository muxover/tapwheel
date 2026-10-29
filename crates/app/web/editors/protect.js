// Pack with a password: AES-256 ZIP, or 7Z which also hides the file names.

import { apply, group, h, iconEl, savesAs, segmented, shell, stem } from "../tw.js";

export default function protect(init) {
	const files = init.files;
	let format = "zip";
	const pw = h("input", { type: "password", autocomplete: "new-password", "aria-label": "Password" });
	const again = h("input", { type: "password", autocomplete: "new-password", "aria-label": "Password again" });
	const show = h("input", { type: "checkbox", class: "switch", "aria-label": "Show password" });
	show.addEventListener("change", () => {
		pw.type = again.type = show.checked ? "text" : "password";
	});
	const strength = h("div", { class: "hint" });
	pw.addEventListener("input", () => {
		const v = pw.value;
		const kinds = [/[a-z]/, /[A-Z]/, /\d/, /[^\w]/].filter((r) => r.test(v)).length;
		strength.textContent = !v ? "" : v.length < 8 ? "Too short: use at least 8 characters" : v.length >= 14 || kinds >= 3 ? "Strong" : "Okay; longer is stronger";
	});
	const name = () => {
		const base = files.length === 1 ? stem(files[0].name) : "Archive";
		return `${base}.${format}`;
	};
	const ui = shell({
		title: "Password",
		lead: "Anyone opening it will need this password. Keep it somewhere safe: it can't be recovered.",
		primary: "Pack",
		panel: [
			group("Format", segmented([{ label: "ZIP (opens anywhere)", value: "zip" }, { label: "7Z (hides names too)", value: "7z" }], "zip", (f) => {
				format = f;
				ui.setOut(savesAs(name()));
			})),
			group("Password", pw, strength, h("div", { style: { height: "8px" } }), again, h("label", { class: "toggle-row" }, h("span", {}, "Show password"), show)),
		],
		onSave: async () => {
			if (pw.value.length < 4) throw new Error("Use a longer password");
			if (pw.value !== again.value) throw new Error("The two passwords don't match");
			apply({ do: "protect", password: pw.value, format });
		},
	});
	ui.stage.replaceChildren(h("div", { class: "loading" }, iconEl("lock_closed", 56), h("div", {}, files.length === 1 ? files[0].name : `${files.length} items`)));
	ui.setOut(savesAs(name()));
	pw.focus();
}
