// Loads every page at phone to wide widths and fails on layout problems.
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
			return { out, links: [...document.querySelectorAll("a[href^='/'], link[href^='/'], img[src^='/'], script[src^='/']")].map((e) => e.getAttribute("href") || e.getAttribute("src")) };
		});
		for (const f of found.out) problems.push(`${page} @${width}: ${f}`);
		for (const e of errors) problems.push(`${page} @${width}: console: ${e}`);
		for (const l of found.links) seen.add(l.split("#")[0] || "/");
		await p.close();
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
