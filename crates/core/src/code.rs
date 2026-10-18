//! Code and markup: formatting, minifying, highlighting.

use std::borrow::Cow;

use crate::Error;

fn engine(e: impl std::fmt::Display) -> Error {
	Error::Engine(e.to_string())
}

fn ends_nl(mut s: String) -> String {
	if !s.ends_with('\n') {
		s.push('\n');
	}
	s
}

fn format_script(text: &str, ext: &str) -> Result<String, Error> {
	use dprint_plugin_typescript::configuration::ConfigurationBuilder;
	let config = ConfigurationBuilder::new().indent_width(2).line_width(100).build();
	let path = std::path::PathBuf::from(format!("file.{ext}"));
	let out = dprint_plugin_typescript::format_text(dprint_plugin_typescript::FormatTextOptions {
		path: &path,
		extension: None,
		text: text.to_string(),
		config: &config,
		external_formatter: None,
	})
	.map_err(engine)?;
	Ok(out.unwrap_or_else(|| text.to_string()))
}

fn format_css(text: &str, ext: &str) -> Result<String, Error> {
	let syntax = match ext {
		"scss" => malva::Syntax::Scss,
		"sass" => malva::Syntax::Sass,
		"less" => malva::Syntax::Less,
		_ => malva::Syntax::Css,
	};
	malva::format_text(text, syntax, &malva::config::FormatOptions::default()).map_err(engine)
}

fn format_markup(text: &str, ext: &str) -> Result<String, Error> {
	let lang = match ext {
		"vue" => markup_fmt::Language::Vue,
		"svelte" => markup_fmt::Language::Svelte,
		"astro" => markup_fmt::Language::Astro,
		_ => markup_fmt::Language::Html,
	};
	markup_fmt::format_text(
		text,
		lang,
		&markup_fmt::config::FormatOptions::default(),
		|code, hints| {
			let ext = hints.ext;
			let formatted = match ext {
				"css" | "scss" | "sass" | "less" => format_css(code, ext).ok(),
				"js" | "ts" | "mjs" | "cjs" | "jsx" | "tsx" | "mts" | "cts" => format_script(code, ext).ok(),
				"json" => pretty_json(code).ok(),
				_ => None,
			};
			Ok::<_, std::convert::Infallible>(formatted.map(Cow::Owned).unwrap_or(Cow::Borrowed(code)))
		},
	)
	.map_err(|e| Error::Engine(format!("{e:?}")))
}

fn pretty_json(text: &str) -> Result<String, Error> {
	let v: serde_json::Value = serde_json::from_str(text).map_err(engine)?;
	Ok(serde_json::to_string_pretty(&v).map_err(engine)? + "\n")
}

/// Re-indents XML two spaces per level, keeping text, comments and declarations.
fn pretty_xml(text: &str) -> Result<String, Error> {
	use quick_xml::events::Event;
	let mut r = quick_xml::Reader::from_str(text);
	r.config_mut().trim_text(true);
	let mut w = quick_xml::Writer::new_with_indent(Vec::new(), b' ', 2);
	loop {
		match r.read_event().map_err(engine)? {
			Event::Eof => break,
			ev => w.write_event(ev).map_err(engine)?,
		}
	}
	Ok(ends_nl(String::from_utf8(w.into_inner()).map_err(engine)?))
}

fn minify_xml(text: &str) -> Result<String, Error> {
	use quick_xml::events::Event;
	let mut r = quick_xml::Reader::from_str(text);
	r.config_mut().trim_text(true);
	let mut w = quick_xml::Writer::new(Vec::new());
	loop {
		match r.read_event().map_err(engine)? {
			Event::Eof => break,
			Event::Comment(_) => {}
			ev => w.write_event(ev).map_err(engine)?,
		}
	}
	String::from_utf8(w.into_inner()).map_err(engine)
}

/// SVG minify: XML minify plus editor leftovers that browsers never use.
fn minify_svg(text: &str) -> Result<String, Error> {
	let small = minify_xml(text)?;
	let mut out = String::with_capacity(small.len());
	let mut rest = small.as_str();
	// drop <metadata>…</metadata> and editor-namespaced elements' blocks
	while let Some(i) = rest.find("<metadata") {
		out.push_str(&rest[..i]);
		match rest[i..].find("</metadata>") {
			Some(j) => rest = &rest[i + j + "</metadata>".len()..],
			None => {
				rest = &rest[i..];
				break;
			}
		}
	}
	out.push_str(rest);
	Ok(out)
}

fn minify_css(text: &str) -> Result<String, Error> {
	use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
	let mut sheet = StyleSheet::parse(text, ParserOptions::default()).map_err(|e| engine(e.to_string()))?;
	sheet
		.minify(MinifyOptions::default())
		.map_err(|e| engine(e.to_string()))?;
	let out = sheet
		.to_css(PrinterOptions {
			minify: true,
			..PrinterOptions::default()
		})
		.map_err(|e| engine(e.to_string()))?;
	Ok(out.code)
}

fn minify_js(text: &str) -> Result<String, Error> {
	let session = minify_js::Session::new();
	let mut out = Vec::new();
	minify_js::minify(&session, minify_js::TopLevelMode::Global, text.as_bytes(), &mut out)
		.map_err(|e| engine(format!("{e:?}")))?;
	String::from_utf8(out).map_err(engine)
}

fn minify_html(text: &str) -> String {
	let cfg = minify_html::Cfg {
		minify_css: true,
		minify_js: true,
		keep_comments: false,
		..minify_html::Cfg::default()
	};
	String::from_utf8_lossy(&minify_html::minify(text.as_bytes(), &cfg)).into_owned()
}

/// Tidies the layout of a file in a language we can format.
pub fn pretty(text: &str, ext: &str) -> Result<String, Error> {
	Ok(match ext {
		"json" | "geojson" => pretty_json(text)?,
		"jsonl" | "ndjson" => {
			text.lines()
				.filter(|l| !l.trim().is_empty())
				.map(|l| serde_json::from_str::<serde_json::Value>(l).map(|v| v.to_string()))
				.collect::<Result<Vec<_>, _>>()
				.map_err(engine)?
				.join("\n") + "\n"
		}
		"xml" | "plist" | "svg" => pretty_xml(text)?,
		"yaml" | "yml" => pretty_yaml::format_text(text, &pretty_yaml::config::FormatOptions::default())
			.map_err(|e| engine(format!("{e:?}")))?,
		"toml" => taplo::formatter::format(text, taplo::formatter::Options::default()),
		"css" | "scss" | "sass" | "less" => format_css(text, ext)?,
		"html" | "htm" | "xhtml" | "vue" | "svelte" | "astro" => format_markup(text, ext)?,
		"js" | "mjs" | "cjs" | "jsx" | "ts" | "mts" | "cts" | "tsx" => format_script(text, ext)?,
		"sql" => ends_nl(sqlformat::format(
			text,
			&sqlformat::QueryParams::None,
			&sqlformat::FormatOptions {
				uppercase: Some(true),
				lines_between_queries: 2,
				..sqlformat::FormatOptions::default()
			},
		)),
		"csv" | "tsv" => {
			let v = crate::text::parse_data(text, ext)?;
			crate::text::write_data(
				&v,
				if ext == "tsv" {
					crate::Format::Tsv
				} else {
					crate::Format::Csv
				},
			)?
		}
		other => return Err(Error::Engine(format!(".{other} files can't be formatted yet"))),
	})
}

/// Squeezes out spaces and comments.
pub fn minify(text: &str, ext: &str) -> Result<String, Error> {
	Ok(match ext {
		"json" | "geojson" => serde_json::from_str::<serde_json::Value>(text)
			.map_err(engine)?
			.to_string(),
		"xml" | "plist" => minify_xml(text)?,
		"svg" | "svgz" => minify_svg(text)?,
		"css" => minify_css(text)?,
		"js" | "mjs" | "cjs" => minify_js(text)?,
		"html" | "htm" | "xhtml" => minify_html(text),
		"yaml" | "yml" => {
			let v: serde_json::Value = serde_yaml::from_str(text).map_err(engine)?;
			v.to_string()
		}
		"toml" => {
			text.lines()
				.map(str::trim)
				.filter(|l| !l.is_empty() && !l.starts_with('#'))
				.collect::<Vec<_>>()
				.join("\n") + "\n"
		}
		"csv" | "tsv" => text.lines().map(str::trim_end).collect::<Vec<_>>().join("\n") + "\n",
		other => return Err(Error::Engine(format!(".{other} files can't be minified yet"))),
	})
}

/// The name Markdown and highlighters use for a file's language.
pub fn language(ext: &str) -> &'static str {
	match ext {
		"rs" => "rust",
		"py" | "pyw" | "pyi" => "python",
		"js" | "mjs" | "cjs" => "javascript",
		"jsx" => "jsx",
		"ts" | "mts" | "cts" => "typescript",
		"tsx" => "tsx",
		"go" => "go",
		"c" | "h" => "c",
		"cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
		"cs" | "csx" => "csharp",
		"vb" => "vbnet",
		"fs" => "fsharp",
		"java" => "java",
		"kt" | "kts" => "kotlin",
		"scala" => "scala",
		"groovy" | "gradle" => "groovy",
		"swift" => "swift",
		"m" | "mm" => "objectivec",
		"rb" => "ruby",
		"php" => "php",
		"pl" | "pm" => "perl",
		"lua" => "lua",
		"r" => "r",
		"jl" => "julia",
		"dart" => "dart",
		"zig" => "zig",
		"nim" => "nim",
		"ex" | "exs" => "elixir",
		"erl" => "erlang",
		"hs" => "haskell",
		"clj" => "clojure",
		"elm" => "elm",
		"ml" => "ocaml",
		"sh" | "bash" | "zsh" => "bash",
		"fish" => "fish",
		"ps1" | "psm1" | "psd1" => "powershell",
		"bat" | "cmd" => "batch",
		"sql" => "sql",
		"css" => "css",
		"scss" | "sass" => "scss",
		"less" => "less",
		"html" | "htm" | "xhtml" => "html",
		"vue" => "vue",
		"svelte" => "svelte",
		"astro" => "astro",
		"graphql" | "gql" => "graphql",
		"proto" => "protobuf",
		"dockerfile" => "dockerfile",
		"makefile" => "makefile",
		"cmake" => "cmake",
		"tf" | "hcl" => "hcl",
		"nix" => "nix",
		"glsl" => "glsl",
		"hlsl" => "hlsl",
		"wgsl" => "wgsl",
		"asm" | "s" => "asm",
		"v" | "sv" => "verilog",
		"vhd" => "vhdl",
		"sol" => "solidity",
		"gd" => "gdscript",
		"ahk" => "autohotkey",
		"json" => "json",
		"yaml" | "yml" => "yaml",
		"toml" => "toml",
		"xml" => "xml",
		"md" => "markdown",
		"reg" => "ini",
		_ => "",
	}
}

/// A fenced Markdown block, with a fence longer than any run of backticks inside.
pub fn markdown_block(text: &str, ext: &str) -> String {
	let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
	let fence = "`".repeat(longest.max(2) + 1);
	let body = text.trim_end_matches(['\n', '\r']);
	format!("{fence}{}\n{body}\n{fence}\n", language(ext))
}

fn syntaxes() -> &'static syntect::parsing::SyntaxSet {
	static SET: std::sync::OnceLock<syntect::parsing::SyntaxSet> = std::sync::OnceLock::new();
	SET.get_or_init(two_face::syntax::extra_newlines)
}

fn themes() -> &'static two_face::theme::EmbeddedLazyThemeSet {
	static SET: std::sync::OnceLock<two_face::theme::EmbeddedLazyThemeSet> = std::sync::OnceLock::new();
	SET.get_or_init(two_face::theme::extra)
}

pub fn syntax_for(ext: &str) -> &'static syntect::parsing::SyntaxReference {
	let set = syntaxes();
	set.find_syntax_by_extension(ext)
		.or_else(|| set.find_syntax_by_token(language(ext)))
		.unwrap_or_else(|| set.find_syntax_plain_text())
}

/// Coloured spans per line, for the code picture.
pub fn highlight(text: &str, ext: &str, dark: bool) -> Vec<Vec<(syntect::highlighting::Style, String)>> {
	use syntect::easy::HighlightLines;
	let theme = themes().get(if dark {
		two_face::theme::EmbeddedThemeName::OneHalfDark
	} else {
		two_face::theme::EmbeddedThemeName::OneHalfLight
	});
	let mut h = HighlightLines::new(syntax_for(ext), theme);
	syntect::util::LinesWithEndings::from(text)
		.map(|line| {
			h.highlight_line(line, syntaxes())
				.unwrap_or_default()
				.into_iter()
				.map(|(s, t)| (s, t.trim_end_matches(['\n', '\r']).to_string()))
				.collect()
		})
		.collect()
}

/// A self-contained page showing the code highlighted, light or dark with the system.
pub fn to_html(text: &str, ext: &str, title: &str) -> Result<String, Error> {
	use syntect::html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style};
	let mut gen_ = ClassedHTMLGenerator::new_with_class_style(
		syntax_for(ext),
		syntaxes(),
		ClassStyle::SpacedPrefixed { prefix: "c-" },
	);
	for line in syntect::util::LinesWithEndings::from(text) {
		gen_.parse_html_for_line_which_includes_newline(line).map_err(engine)?;
	}
	let body = gen_.finalize();
	let light = css_for_theme_with_class_style(
		themes().get(two_face::theme::EmbeddedThemeName::OneHalfLight),
		ClassStyle::SpacedPrefixed { prefix: "c-" },
	)
	.map_err(engine)?;
	let dark = css_for_theme_with_class_style(
		themes().get(two_face::theme::EmbeddedThemeName::OneHalfDark),
		ClassStyle::SpacedPrefixed { prefix: "c-" },
	)
	.map_err(engine)?;
	let css = format!(
		"body{{margin:0;padding:2rem;background:#fafafa;color:#383a42;font:14px/1.6 'Cascadia Code','JetBrains Mono',Consolas,monospace}}h1{{font:600 15px 'Segoe UI',system-ui,sans-serif;margin:0 0 1rem;color:#6a737d}}pre{{margin:0;white-space:pre;overflow:auto}}{light}@media (prefers-color-scheme:dark){{body{{background:#282c34;color:#abb2bf}}{dark}}}"
	);
	Ok(format!(
		"<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>{css}</style>\n</head>\n<body>\n<h1>{title}</h1>\n<pre class=\"c-code\">{body}</pre>\n</body>\n</html>\n"
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn json_and_xml_tidy() {
		assert_eq!(
			pretty("{\"a\":[1,2]}", "json").unwrap(),
			"{\n  \"a\": [\n    1,\n    2\n  ]\n}\n"
		);
		assert_eq!(minify("{\n \"a\": 1\n}", "json").unwrap(), "{\"a\":1}");
		assert_eq!(pretty("<a><b>x</b></a>", "xml").unwrap(), "<a>\n  <b>x</b>\n</a>\n");
		assert_eq!(
			minify("<a>\n  <!-- c -->\n  <b>x</b>\n</a>", "xml").unwrap(),
			"<a><b>x</b></a>"
		);
	}

	#[test]
	fn css_and_js_shrink() {
		assert_eq!(minify("a {  color: red;  }\n", "css").unwrap(), "a{color:red}");
		let js = minify("function add(first, second) {\n  return first + second;\n}\n", "js").unwrap();
		assert!(js.len() < 40, "{js}");
		assert!(pretty("a{color:red}", "css").unwrap().contains("color: red;"));
		assert!(pretty("const a={b:1}", "js").unwrap().contains("const a = { b: 1 };"));
		assert!(
			pretty("select a,b from t where x=1", "sql")
				.unwrap()
				.starts_with("SELECT")
		);
	}

	#[test]
	fn markdown_fence_outgrows_backticks() {
		assert_eq!(markdown_block("let a = 1;\n", "rs"), "```rust\nlet a = 1;\n```\n");
		assert!(markdown_block("use ``` inside", "md").starts_with("````markdown"));
	}

	#[test]
	fn html_page_highlights() {
		let page = to_html("fn main() {}\n", "rs", "main.rs").unwrap();
		assert!(page.contains("c-code") && page.contains("prefers-color-scheme"));
	}
}
