// Loads every page at phone to wide widths and fails on layout problems, link order, contrast,
// accent overuse, a hidden header part and search tags.
// node site/check.mjs [base url]   (serves site/public itself when no url is given)
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const { chromium } = await import(process.env.PLAYWRIGHT || "playwright");
const root = fileURLToPath(new URL("./public/", import.meta.url));
// the app's own pages, which the deploy copies to /app/
const appPages = fileURLToPath(new URL("../crates/app/web/", import.meta.url));
const types = { ".html": "text/html", ".css": "text/css", ".js": "text/javascript", ".json": "application/json", ".svg": "image/svg+xml", ".png": "image/png", ".jpg": "image/jpeg", ".mp4": "video/mp4", ".m4a": "audio/mp4", ".woff2": "font/woff2", ".txt": "text/plain", ".xml": "application/xml" };

let base = process.argv[2];
let server;
if (!base) {
	server = createServer(async (req, res) => {
		let path = decodeURIComponent(new URL(req.url, "http://x").pathname);
		if (path.endsWith("/")) path += "index.html";
		let file = path.startsWith("/app/") ? join(appPages, path.slice(5)) : join(root, path);
		try {
			if (!(await stat(file)).isFile()) throw 0;
		} catch {
			try {
				file = join(root, path + ".html");
				await stat(file);
			} catch {
				res.writeHead(404, { "content-type": "text/html" });
				return res.end(await readFile(join(root, "404.html")));
			}
		}
		res.writeHead(200, { "content-type": types[extname(file)] || "application/octet-stream" });
		res.end(await readFile(file));
	});
	await new Promise((r) => server.listen(0, r));
	base = `http://localhost:${server.address().port}`;
}

const pages = ["/", "/privacy", "/support", "/404"];
const widths = [320, 390, 768, 1280, 1440, 1920];
const problems = [];
const browser = await chromium.launch();
const seen = new Set();

for (const page of pages) {
	for (const width of widths) {
		const p = await browser.newPage({ viewport: { width, height: 900 } });
		const errors = [];
		p.on("pageerror", (e) => errors.push(e.message));
		p.on("console", (m) => m.type() === "error" && errors.push(m.text()));
		await p.goto(base + page);
		await p.waitForTimeout(150);
		const found = await p.evaluate(() => {
			const out = [];
			if (document.documentElement.scrollWidth > innerWidth) out.push(`page is ${document.documentElement.scrollWidth}px wide`);
			const h1 = document.querySelectorAll("h1").length;
			if (h1 !== 1) out.push(`${h1} h1 elements`);
			for (const el of document.querySelectorAll("h1, h2, h3, p, li, summary, figcaption")) {
				if (!el.offsetParent || el.closest(".demo")) continue;
				const r = el.getBoundingClientRect();
				if (r.right > innerWidth + 1 || r.left < -1) out.push(`off screen: ${el.tagName} "${el.textContent.trim().slice(0, 40)}"`);
				// orphan: the last line holds one short word
				const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
				const words = [];
				while (walker.nextNode()) {
					const n = walker.currentNode;
					const re = /\S+/g;
					let m;
					while ((m = re.exec(n.data))) {
						const range = document.createRange();
						range.setStart(n, m.index);
						range.setEnd(n, m.index + m[0].length);
						const rects = range.getClientRects();
						if (rects.length) words.push({ w: m[0], top: Math.round(rects[rects.length - 1].top) });
					}
				}
				// words within 10px of each other vertically share a line (inline chips sit a little higher)
				const tops = [...new Set(words.map((w) => w.top))].sort((x, y) => x - y);
				const lines = tops.filter((t, i) => i === 0 || t - tops[i - 1] > 10);
				if (lines.length > 1) {
					const lastTop = lines[lines.length - 1];
					const last = words.filter((w) => w.top >= lastTop - 10 && w.top >= lines[lines.length - 2] + 10);
					if (last.length === 1 && last[0].w.length < 10) out.push(`orphan "${last[0].w}" in ${el.tagName} "${el.textContent.trim().slice(0, 40)}"`);
				}
			}
			for (const a of document.querySelectorAll("a, button, summary")) {
				const r = a.getBoundingClientRect();
				if (!a.offsetParent || a.closest(".footer, .nav")) continue;
				if (r.height < 44 && getComputedStyle(a).display !== "inline") out.push(`small target ${Math.round(r.height)}px: "${a.textContent.trim().slice(0, 30)}"`);
			}
			if (/\.\.\.|…/.test(document.body.innerText)) out.push("ellipsis in text");
			// link lists run shortest first, except a header that follows the page
			for (const list of document.querySelectorAll("nav, ul, ol")) {
				if (list.closest("[data-order='page']")) continue;
				const links = list.matches("nav") ? [...list.querySelectorAll(":scope > a")] : [...list.children].map((li) => (li.children.length === 1 && li.firstElementChild.tagName === "A" && li.textContent.trim() === li.firstElementChild.textContent.trim() ? li.firstElementChild : null));
				if (links.length < 2 || links.some((a) => !a)) continue;
				const lens = links.map((a) => a.textContent.trim().length);
				if (lens.some((n, i) => i && n < lens[i - 1])) out.push(`links not shortest first: ${links.map((a) => a.textContent.trim()).join(", ")}`);
			}
			// colour: parse, find what's behind, measure contrast
			const rgb = (c) => {
				const m = c.match(/[\d.]+/g);
				return m ? m.map(Number) : [0, 0, 0, 0];
			};
			const lum = ([r, g, b]) => {
				const f = (v) => ((v /= 255) <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
				return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
			};
			const behind = (el) => {
				for (let e = el; e; e = e.parentElement) {
					const c = rgb(getComputedStyle(e).backgroundColor);
					if ((c[3] ?? 1) > 0.5) return c;
				}
				return rgb(getComputedStyle(document.body).backgroundColor);
			};
			const skip = (el) => el.closest(".tw-scope, svg, iframe, .try-frame, .skip");
			for (const el of document.querySelectorAll("h1, h2, h3, p, li, a, button, summary, dt, dd, span, strong, figcaption, label")) {
				if (!el.offsetParent || skip(el)) continue;
				if (![...el.childNodes].some((n) => n.nodeType === 3 && n.data.trim())) continue;
				const cs = getComputedStyle(el);
				if (Number(cs.opacity) < 0.5) continue;
				const fg = rgb(cs.color);
				const bg = behind(el);
				const [a, b] = [lum(fg), lum(bg)].sort((x, y) => y - x);
				const ratio = (a + 0.05) / (b + 0.05);
				const size = parseFloat(cs.fontSize);
				const large = size >= 24 || (size >= 18.66 && Number(cs.fontWeight) >= 600);
				if (ratio < (large ? 3 : 4.5)) out.push(`contrast ${ratio.toFixed(2)}: ${el.tagName} "${el.textContent.trim().slice(0, 30)}"`);
			}
			// the accent marks the main action and what's live, never decoration
			const accent = getComputedStyle(document.documentElement).getPropertyValue("--blue").trim();
			if (accent) {
				const probe = document.createElement("i");
				probe.style.color = accent;
				document.body.append(probe);
				const want = getComputedStyle(probe).color;
				probe.remove();
				let uses = 0;
				for (const el of document.querySelectorAll("body *")) {
					if (!el.offsetParent || skip(el)) continue;
					const cs = getComputedStyle(el);
					const after = getComputedStyle(el, "::after");
					if (cs.backgroundColor === want || cs.color === want || cs.borderTopColor === want || (after.content !== "none" && after.backgroundColor === want)) uses++;
				}
				if (uses > 12) out.push(`the accent is used ${uses} times`);
			}
			// the header: logo, links and the button visible, inside it and not covered
			const header = document.querySelector(".header");
			for (const y of [0, Math.round(document.documentElement.scrollHeight / 2)]) {
				scrollTo(0, y);
				const box = header.getBoundingClientRect();
				const parts = [header.querySelector(".brand"), header.querySelector(".button"), ...(innerWidth > 800 ? header.querySelectorAll(".nav a") : [])];
				for (const el of parts) {
					const r = el.getBoundingClientRect();
					const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
					if (!r.width || r.top < box.top - 1 || r.bottom > box.bottom + 1 || !el.contains(hit)) out.push(`header part hidden at ${y}px: "${el.textContent.trim()}"`);
				}
			}
			scrollTo(0, 0);
			// search: description length and title shape
			const desc = document.querySelector('meta[name="description"]')?.content || "";
			if (desc.length < 20 || desc.length > 160) out.push(`description is ${desc.length} characters`);
			if (!/ · muxover$/.test(document.title)) out.push(`title doesn't end with the brand: "${document.title}"`);
			return { out, links: [...document.querySelectorAll("a[href^='/'], link[href^='/'], img[src^='/'], script[src^='/']")].map((e) => e.getAttribute("href") || e.getAttribute("src")) };
		});
		for (const f of found.out) problems.push(`${page} @${width}: ${f}`);
		for (const e of errors) problems.push(`${page} @${width}: console: ${e}`);
		for (const l of found.links) seen.add(l.split("#")[0] || "/");
		await p.close();
	}
}

// a layout class defined twice once collapsed a live header
{
	const css = await readFile(new URL("./public/style.css", import.meta.url), "utf8");
	for (const name of ["header", "nav", "bar", "wrap", "brand"]) {
		const defs = css.match(new RegExp(`(^|\\n)\\.${name}\\s*\\{`, "g")) || [];
		if (defs.length > 1) problems.push(`.${name} is defined ${defs.length} times in style.css`);
	}
}

for (const link of seen) {
	const res = await fetch(base + link);
	if (!res.ok) problems.push(`broken link ${link} (${res.status})`);
}

await browser.close();
server?.close();
const unique = [...new Set(problems)];
if (unique.length) {
	console.error(unique.join("\n"));
	process.exit(1);
}
console.log(`ok: ${pages.length} pages at ${widths.length} widths, ${seen.size} links`);
