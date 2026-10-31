use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::Error;
use crate::format::Format;
use crate::kind::ext_of;
use crate::recipe::{Case, Recipe, SortBy};

const MB: u64 = 1024 * 1024;

/// The largest file a tool takes: a fixed ceiling, and what fits in the memory this PC has free
/// when the job starts, given how many times the file's size the tool holds while it works.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limit {
	pub ceiling: u64,
	pub copies: u64,
}

// measured copies: about 2× for counting, 4× for the line tools, 30× for data conversions and
// 50× for CSV; the ceilings keep a 4 GB PC from running out of memory halfway
pub const READ_LIMIT: Limit = Limit {
	ceiling: 700 * MB,
	copies: 2,
};
pub const LINE_LIMIT: Limit = Limit {
	ceiling: 400 * MB,
	copies: 4,
};
pub const DATA_LIMIT: Limit = Limit {
	ceiling: 64 * MB,
	copies: 30,
};
pub const CSV_LIMIT: Limit = Limit {
	ceiling: 32 * MB,
	copies: 50,
};
/// SQL formatting slows down with the square of the size.
pub const SQL_LIMIT: Limit = Limit {
	ceiling: 2 * MB,
	copies: 30,
};
/// Read aloud keeps the whole recording in memory, about 3 KB per character.
pub const SPEECH_LIMIT: Limit = Limit {
	ceiling: 300 * 1024,
	copies: 3 * 1024,
};

/// Refuses a file before reading it when it's over the limit's ceiling, or when this PC hasn't
/// the memory free for it now.
pub fn check_size(path: &Path, limit: Limit) -> Result<(), Error> {
	check_len(fs::metadata(path)?.len(), limit, crate::budget::free_memory())
}

fn check_len(len: u64, limit: Limit, free: Option<u64>) -> Result<(), Error> {
	if len > limit.ceiling {
		let limit = limit.ceiling;
		let mb = |b: u64| (b as f64 / MB as f64).ceil() as u64;
		let size = |b: u64| {
			if b < MB {
				format!("{} KB", b.div_ceil(1024))
			} else {
				format!("{} MB", mb(b))
			}
		};
		return Err(Error::Engine(format!(
			"this file is too big for this tool ({}; it works up to {})",
			size(len),
			size(limit)
		)));
	}
	crate::budget::fits(len.saturating_mul(limit.copies), free)
		.map_err(|e| Error::Engine(format!("this file is too big for this tool right now: {e}")))
}

/// Reads a text file in whatever encoding it uses.
pub fn read(path: &Path) -> Result<String, Error> {
	read_within(path, READ_LIMIT)
}

/// Like [`read`], refusing files over `limit` bytes before reading them.
pub fn read_within(path: &Path, limit: Limit) -> Result<String, Error> {
	check_size(path, limit)?;
	let bytes = fs::read(path)?;
	// valid UTF-8 becomes the String without a second copy
	match String::from_utf8(bytes) {
		Ok(s) if !s.starts_with('\u{feff}') => Ok(s),
		Ok(s) => Ok(decode(s.as_bytes())),
		Err(e) => Ok(decode(e.as_bytes())),
	}
}

/// How a text file was stored, so a tool can write its result back the same way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stored {
	pub encoding: &'static encoding_rs::Encoding,
	pub bom: bool,
}

/// How much of a file the formatter for `ext` can take.
pub fn format_limit(ext: &str) -> Limit {
	match ext {
		"sql" => SQL_LIMIT,
		"csv" | "tsv" => CSV_LIMIT,
		_ => DATA_LIMIT,
	}
}

pub fn decode(bytes: &[u8]) -> String {
	decode_stored(bytes).0
}

/// Text in the encoding it was saved in. A file that is UTF-8 apart from a few broken bytes stays
/// UTF-8, with those bytes replaced, instead of being read as an older code page.
pub fn decode_stored(bytes: &[u8]) -> (String, Stored) {
	if let Some((enc, bom)) = encoding_rs::Encoding::for_bom(bytes) {
		let (text, _) = enc.decode_without_bom_handling(&bytes[bom..]);
		return (
			text.into_owned(),
			Stored {
				encoding: enc,
				bom: true,
			},
		);
	}
	let utf8 = Stored {
		encoding: encoding_rs::UTF_8,
		bom: false,
	};
	if let Ok(s) = std::str::from_utf8(bytes) {
		return (s.to_string(), utf8);
	}
	let (mut broken, mut accented) = (0usize, 0usize);
	for chunk in bytes.utf8_chunks() {
		accented += chunk.valid().chars().filter(|c| !c.is_ascii()).count();
		broken += !chunk.invalid().is_empty() as usize;
	}
	if accented > 0 && broken <= accented {
		return (String::from_utf8_lossy(bytes).into_owned(), utf8);
	}
	let mut det = chardetng::EncodingDetector::new();
	det.feed(bytes, true);
	let enc = det.guess(None, true);
	let (text, _, _) = enc.decode(bytes);
	(
		text.into_owned(),
		Stored {
			encoding: enc,
			bom: false,
		},
	)
}

/// `text` saved the way the original was: same encoding, same byte-order mark.
pub fn encode_stored(text: &str, s: Stored) -> Vec<u8> {
	let mut out = Vec::with_capacity(text.len() + 3);
	let enc = s.encoding;
	if s.bom {
		out.extend_from_slice(if enc == encoding_rs::UTF_16LE {
			&[0xff, 0xfe][..]
		} else if enc == encoding_rs::UTF_16BE {
			&[0xfe, 0xff][..]
		} else {
			&[0xef, 0xbb, 0xbf][..]
		});
	}
	if enc == encoding_rs::UTF_16LE {
		out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
	} else if enc == encoding_rs::UTF_16BE {
		out.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
	} else {
		out.extend_from_slice(&enc.encode(text).0);
	}
	out
}

/// The encoding the file seems to use, for the card after a fix.
pub fn encoding_name(bytes: &[u8]) -> &'static str {
	if let Some((enc, _)) = encoding_rs::Encoding::for_bom(bytes) {
		return enc.name();
	}
	if std::str::from_utf8(bytes).is_ok() {
		return "UTF-8";
	}
	let mut det = chardetng::EncodingDetector::new();
	det.feed(bytes, true);
	det.guess(None, true).name()
}

fn engine(e: impl std::fmt::Display) -> Error {
	Error::Engine(e.to_string())
}

/// How deep nesting may go before a file is refused. Real files stay far below it; deeper ones
/// are broken or hostile, and the parsers and formatters would run out of stack on them.
pub const MAX_DEPTH: usize = 512;

pub fn too_deep() -> Error {
	Error::Engine("this file is nested too deeply to work on".into())
}

/// The deepest `[`, `{` or `(` nesting in the text. Brackets inside strings count too, which
/// only matters for files that are refused anyway.
pub fn bracket_depth(text: &str) -> usize {
	let (mut depth, mut max) = (0usize, 0usize);
	for b in text.bytes() {
		match b {
			b'[' | b'{' | b'(' => {
				depth += 1;
				max = max.max(depth);
			}
			b']' | b'}' | b')' => depth = depth.saturating_sub(1),
			_ => {}
		}
	}
	max
}

/// The deepest element nesting in HTML, leaving out elements that close themselves.
pub fn tag_depth(html: &str) -> usize {
	const OPEN_ENDED: &[&str] = &[
		"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr", "p",
		"li", "dt", "dd", "tr", "td", "th", "option", "thead", "tbody", "tfoot", "colgroup", "rt", "rp", "optgroup",
	];
	let b = html.as_bytes();
	let (mut depth, mut max) = (0usize, 0usize);
	let mut i = 0;
	while let Some(at) = html[i..].find('<') {
		i += at + 1;
		match b.get(i) {
			Some(b'/') => depth = depth.saturating_sub(1),
			Some(c) if c.is_ascii_alphabetic() => {
				let end = html[i..]
					.find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
					.map_or(html.len(), |e| i + e);
				let name = html[i..end].to_ascii_lowercase();
				let close = html[end..].find('>').map_or(html.len(), |e| end + e);
				let self_closing = close > 0 && b.get(close - 1) == Some(&b'/');
				if !OPEN_ENDED.contains(&name.as_str()) && !self_closing {
					depth += 1;
					max = max.max(depth);
				}
			}
			_ => {}
		}
	}
	max
}

/// Line ending the file mostly uses.
fn crlf(text: &str) -> bool {
	text.matches("\r\n").count() * 2 > text.matches('\n').count()
}

fn lines(text: &str) -> Vec<&str> {
	text.lines().collect()
}

fn join(lines: &[impl AsRef<str>], crlf: bool) -> String {
	let nl = if crlf { "\r\n" } else { "\n" };
	let mut s = lines.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(nl);
	if !s.is_empty() {
		s.push_str(nl);
	}
	s
}

fn delimiter(ext: &str) -> u8 {
	match ext {
		"tsv" => b'\t',
		"psv" => b'|',
		_ => b',',
	}
}

/// A cell turned into a number or boolean only when it reads back the same, so "007" stays text.
fn cell(s: &str) -> Value {
	if s.is_empty() {
		return Value::String(String::new());
	}
	if let Ok(i) = s.parse::<i64>()
		&& i.to_string() == s
	{
		return Value::from(i);
	}
	if let Ok(f) = s.parse::<f64>()
		&& f.is_finite()
		&& (f.to_string() == s || format!("{f:?}") == s)
	{
		return Value::from(f);
	}
	match s {
		"true" => Value::Bool(true),
		"false" => Value::Bool(false),
		_ => Value::String(s.to_string()),
	}
}

/// Puts `value` at a dotted path like `address.city`, building objects on the way.
fn set_path(obj: &mut Map<String, Value>, path: &str, value: Value) {
	let mut parts = path.split('.').peekable();
	let mut cur = obj;
	while let Some(p) = parts.next() {
		if parts.peek().is_none() {
			cur.insert(p.to_string(), value);
			return;
		}
		let next = cur.entry(p.to_string()).or_insert_with(|| Value::Object(Map::new()));
		if !next.is_object() {
			*next = Value::Object(Map::new());
		}
		cur = next.as_object_mut().expect("just made an object");
	}
}

fn parse_csv(text: &str, delim: u8) -> Result<Value, Error> {
	let mut r = csv::ReaderBuilder::new()
		.delimiter(delim)
		.flexible(true)
		.from_reader(text.as_bytes());
	let mut headers: Vec<String> = Vec::new();
	for h in r.headers().map_err(engine)?.iter() {
		// a repeated column keeps its values under name_2, name_3 and so on
		let mut name = h.to_string();
		let mut n = 1;
		while headers.contains(&name) {
			n += 1;
			name = format!("{h}_{n}");
		}
		headers.push(name);
	}
	// "a.b" columns nest as {"a": {"b": ...}}, unless "a" is a column of its own as well
	let dotted = headers.iter().any(|h| h.contains('.'))
		&& !headers
			.iter()
			.any(|h| headers.iter().any(|other| other.starts_with(&format!("{h}."))));
	let mut rows = Vec::new();
	for rec in r.records() {
		let rec = rec.map_err(engine)?;
		let mut obj = Map::new();
		for (i, v) in rec.iter().enumerate() {
			let key = headers.get(i).cloned().unwrap_or_else(|| format!("column{}", i + 1));
			if dotted {
				set_path(&mut obj, &key, cell(v));
			} else {
				obj.insert(key, cell(v));
			}
		}
		rows.push(Value::Object(obj));
	}
	Ok(Value::Array(rows))
}

/// XML into nested objects: attributes as `@name`, text as `#text`, repeated children as lists.
fn parse_xml(text: &str) -> Result<Value, Error> {
	use quick_xml::events::Event;
	let mut reader = quick_xml::Reader::from_str(text);
	reader.config_mut().trim_text(true);
	// stack of (name, object, text gathered so far)
	let mut stack: Vec<(String, Map<String, Value>, String)> = vec![("#document".into(), Map::new(), String::new())];
	let attrs = |e: &quick_xml::events::BytesStart| -> Map<String, Value> {
		let mut m = Map::new();
		for a in e.attributes().flatten() {
			let k = format!("@{}", String::from_utf8_lossy(a.key.as_ref()));
			let v = a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
			m.insert(k, cell(&v));
		}
		m
	};
	let push_child = |parent: &mut Map<String, Value>, name: String, v: Value| match parent.get_mut(&name) {
		Some(Value::Array(a)) => a.push(v),
		Some(existing) => {
			let old = existing.take();
			*existing = Value::Array(vec![old, v]);
		}
		None => {
			parent.insert(name, v);
		}
	};
	let finish = |m: Map<String, Value>| -> Value {
		if m.len() == 1
			&& let Some(t) = m.get("#text")
		{
			return t.clone();
		}
		if m.is_empty() {
			return Value::String(String::new());
		}
		Value::Object(m)
	};
	loop {
		match reader.read_event().map_err(engine)? {
			Event::Start(e) => {
				if stack.len() > MAX_DEPTH {
					return Err(too_deep());
				}
				let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
				stack.push((name, attrs(&e), String::new()));
			}
			Event::Empty(e) => {
				let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
				let v = finish(attrs(&e));
				if let Some((_, parent, _)) = stack.last_mut() {
					push_child(parent, name, v);
				}
			}
			Event::Text(t) => {
				if let Some((_, _, text)) = stack.last_mut() {
					text.push_str(&t.decode().map_err(engine)?);
				}
			}
			Event::GeneralRef(r) => {
				let resolved = match r.resolve_char_ref().map_err(engine)? {
					Some(ch) => ch.to_string(),
					None => {
						let name = r.decode().map_err(engine)?;
						quick_xml::escape::resolve_predefined_entity(&name)
							.unwrap_or("")
							.to_string()
					}
				};
				if let Some((_, _, text)) = stack.last_mut() {
					text.push_str(&resolved);
				}
			}
			Event::CData(t) => {
				if let Some((_, _, text)) = stack.last_mut() {
					text.push_str(&String::from_utf8_lossy(&t));
				}
			}
			Event::End(_) => {
				if stack.len() > 1 {
					let (name, mut m, text) = stack.pop().expect("checked");
					if !text.trim().is_empty() {
						m.insert("#text".into(), cell(text.trim()));
					}
					let v = finish(m);
					if let Some((_, parent, _)) = stack.last_mut() {
						push_child(parent, name, v);
					}
				}
			}
			Event::Eof => break,
			_ => {}
		}
	}
	let (_, doc, _) = stack.swap_remove(0);
	Ok(Value::Object(doc))
}

/// INI, properties and .env files into objects, with `[sections]` as nested objects.
fn parse_ini(text: &str) -> Value {
	let mut root = Map::new();
	let mut section: Option<String> = None;
	for line in text.lines() {
		let l = line.trim();
		if l.is_empty() || l.starts_with(['#', ';', '!']) {
			continue;
		}
		if l.starts_with('[') && l.ends_with(']') {
			section = Some(l[1..l.len() - 1].trim().to_string());
			continue;
		}
		let l = l.strip_prefix("export ").unwrap_or(l);
		let Some((k, v)) = l.split_once(['=', ':']) else {
			continue;
		};
		let v = v.trim().trim_matches('"').trim_matches('\'');
		let target = match &section {
			Some(s) => {
				let entry = root.entry(s.clone()).or_insert_with(|| Value::Object(Map::new()));
				// a key named like a later section gives way to the section
				if !entry.is_object() {
					*entry = Value::Object(Map::new());
				}
				entry.as_object_mut().expect("just made an object")
			}
			None => &mut root,
		};
		target.insert(k.trim().to_string(), cell(v));
	}
	Value::Object(root)
}

pub fn parse_data(text: &str, ext: &str) -> Result<Value, Error> {
	Ok(match ext {
		"json" | "geojson" | "json5" => serde_json::from_str(text).map_err(engine)?,
		"jsonl" | "ndjson" => Value::Array(
			text.lines()
				.filter(|l| !l.trim().is_empty())
				.map(serde_json::from_str)
				.collect::<Result<_, _>>()
				.map_err(engine)?,
		),
		"yaml" | "yml" => serde_yaml::from_str(text).map_err(engine)?,
		"toml" => {
			if bracket_depth(text) > MAX_DEPTH {
				return Err(too_deep());
			}
			let v: toml::Value = toml::from_str(text).map_err(engine)?;
			serde_json::to_value(v).map_err(engine)?
		}
		"xml" | "plist" => parse_xml(text)?,
		"csv" | "tsv" | "psv" => parse_csv(text, delimiter(ext))?,
		"ini" | "cfg" | "conf" | "properties" | "env" => parse_ini(text),
		other => return Err(Error::Engine(format!(".{other} isn't a data file"))),
	})
}

/// Nested objects into `a.b` keys, so they fit in table columns.
fn flatten(v: &Value, prefix: &str, out: &mut Vec<(String, String)>) {
	let key = |k: &str| {
		if prefix.is_empty() {
			k.to_string()
		} else {
			format!("{prefix}.{k}")
		}
	};
	match v {
		Value::Object(m) => {
			for (k, v) in m {
				flatten(v, &key(k), out);
			}
		}
		Value::Array(a) if a.iter().all(|x| !x.is_object() && !x.is_array()) => {
			let joined = a.iter().map(scalar).collect::<Vec<_>>().join("; ");
			out.push((prefix.to_string(), joined));
		}
		Value::Array(a) => {
			for (i, x) in a.iter().enumerate() {
				flatten(x, &key(&i.to_string()), out);
			}
		}
		other => out.push((prefix.to_string(), scalar(other))),
	}
}

fn scalar(v: &Value) -> String {
	match v {
		Value::Null => String::new(),
		Value::String(s) => s.clone(),
		other => other.to_string(),
	}
}

/// The rows a table should hold: a list stays a list, a single object is one row, and an object
/// holding exactly one list of rows (like `{"users": [...]}`) uses that list.
fn rows(v: &Value) -> Vec<&Value> {
	match v {
		Value::Array(a) => a.iter().collect(),
		Value::Object(m) => {
			let lists: Vec<&Value> = m.values().filter(|x| x.is_array()).collect();
			if m.len() == 1
				&& let [Value::Array(a)] = lists[..]
			{
				return a.iter().collect();
			}
			vec![v]
		}
		other => vec![other],
	}
}

fn write_csv(v: &Value, delim: u8) -> Result<String, Error> {
	let mut cols: Vec<String> = Vec::new();
	let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
	// each row as (column, value) pairs, columns numbered in the order they first appear
	let mut table: Vec<Vec<(usize, String)>> = Vec::new();
	for row in rows(v) {
		let mut cells = Vec::new();
		if row.is_object() {
			flatten(row, "", &mut cells);
		} else {
			cells.push(("value".into(), scalar(row)));
		}
		let numbered = cells
			.into_iter()
			.map(|(k, v)| {
				let n = match index.get(&k) {
					Some(&n) => n,
					None => {
						index.insert(k.clone(), cols.len());
						cols.push(k);
						cols.len() - 1
					}
				};
				(n, v)
			})
			.collect();
		table.push(numbered);
	}
	drop(index);
	let mut w = csv::WriterBuilder::new().delimiter(delim).from_writer(Vec::new());
	w.write_record(&cols).map_err(engine)?;
	let mut rec = vec![String::new(); cols.len()];
	for cells in table {
		rec.iter_mut().for_each(String::clear);
		for (n, v) in cells {
			rec[n] = v;
		}
		w.write_record(&rec).map_err(engine)?;
	}
	String::from_utf8(w.into_inner().map_err(engine)?).map_err(engine)
}

fn xml_name(k: &str) -> String {
	let mut s: String = k
		.chars()
		.map(|c| {
			if c.is_alphanumeric() || matches!(c, '_' | '-' | '.') {
				c
			} else {
				'_'
			}
		})
		.collect();
	if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.') {
		s.insert(0, '_');
	}
	s
}

fn xml_escape(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
}

fn write_xml_node(name: &str, v: &Value, depth: usize, out: &mut String) {
	let pad = "  ".repeat(depth);
	let tag = xml_name(name);
	match v {
		Value::Array(a) => {
			for x in a {
				write_xml_node(name, x, depth, out);
			}
		}
		Value::Object(m) => {
			out.push_str(&format!("{pad}<{tag}"));
			for (k, v) in m.iter().filter(|(k, _)| k.starts_with('@')) {
				out.push_str(&format!(" {}=\"{}\"", xml_name(&k[1..]), xml_escape(&scalar(v))));
			}
			let children: Vec<_> = m.iter().filter(|(k, _)| !k.starts_with('@')).collect();
			if children.is_empty() {
				out.push_str("/>\n");
				return;
			}
			if let [(k, t)] = children[..]
				&& k == "#text"
			{
				out.push_str(&format!(">{}</{tag}>\n", xml_escape(&scalar(t))));
				return;
			}
			out.push_str(">\n");
			for (k, v) in children {
				if k == "#text" {
					out.push_str(&format!("{pad}  {}\n", xml_escape(&scalar(v))));
				} else {
					write_xml_node(k, v, depth + 1, out);
				}
			}
			out.push_str(&format!("{pad}</{tag}>\n"));
		}
		other => out.push_str(&format!("{pad}<{tag}>{}</{tag}>\n", xml_escape(&scalar(other)))),
	}
}

fn write_xml(v: &Value) -> String {
	let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
	match v {
		Value::Object(m) if m.len() == 1 && !m.values().next().is_some_and(Value::is_array) => {
			let (k, v) = m.iter().next().expect("one entry");
			write_xml_node(k, v, 0, &mut out);
		}
		Value::Array(a) => {
			out.push_str("<items>\n");
			for x in a {
				write_xml_node("item", x, 1, &mut out);
			}
			out.push_str("</items>\n");
		}
		other => write_xml_node("root", other, 0, &mut out),
	}
	out
}

/// TOML has no null and needs a table at the top.
fn toml_ready(v: &Value) -> Value {
	match v {
		Value::Object(m) => Value::Object(
			m.iter()
				.filter(|(_, v)| !v.is_null())
				.map(|(k, v)| (k.clone(), toml_ready(v)))
				.collect(),
		),
		Value::Array(a) => Value::Array(a.iter().filter(|v| !v.is_null()).map(toml_ready).collect()),
		other => other.clone(),
	}
}

pub fn write_data(v: &Value, to: Format) -> Result<String, Error> {
	Ok(match to {
		Format::Json => serde_json::to_string_pretty(v).map_err(engine)? + "\n",
		Format::Jsonl => {
			let mut s = String::new();
			for row in rows(v) {
				s.push_str(&serde_json::to_string(row).map_err(engine)?);
				s.push('\n');
			}
			s
		}
		Format::Yaml => serde_yaml::to_string(v).map_err(engine)?,
		Format::Toml => {
			let ready = match toml_ready(v) {
				obj @ Value::Object(_) => obj,
				other => serde_json::json!({ "items": other }),
			};
			let t: toml::Value = serde_json::from_value(ready).map_err(engine)?;
			toml::to_string_pretty(&t).map_err(engine)?
		}
		Format::Xml => write_xml(v),
		Format::Csv => write_csv(v, b',')?,
		Format::Tsv => write_csv(v, b'\t')?,
		other => return Err(Error::Engine(format!("can't write {} from data", other.label()))),
	})
}

const PAGE_CSS: &str = "body{max-width:46rem;margin:3rem auto;padding:0 1.25rem;font:16px/1.65 'Segoe UI Variable Text','Segoe UI',system-ui,sans-serif;color:#1f2328;background:#fff}h1,h2,h3{line-height:1.25}h1,h2{border-bottom:1px solid #d1d9e0;padding-bottom:.3em}a{color:#0969da}code,pre{font-family:'Cascadia Code',Consolas,monospace;font-size:.9em}code{background:#eff1f3;padding:.15em .35em;border-radius:6px}pre{background:#f6f8fa;padding:1rem;border-radius:8px;overflow:auto}pre code{background:none;padding:0}blockquote{margin:0;padding:0 1em;color:#59636e;border-left:.25em solid #d1d9e0}table{border-collapse:collapse}th,td{border:1px solid #d1d9e0;padding:.4em .8em}img{max-width:100%}hr{border:0;border-top:1px solid #d1d9e0}@media (prefers-color-scheme:dark){body{color:#e6edf3;background:#0d1117}h1,h2{border-color:#3d444d}a{color:#4493f8}code{background:#262c36}pre{background:#151b23}blockquote{color:#9198a1;border-color:#3d444d}th,td,hr{border-color:#3d444d}}";

pub fn page(title: &str, body: &str) -> String {
	format!(
		"<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{PAGE_CSS}</style>\n</head>\n<body>\n{body}</body>\n</html>\n",
		xml_escape(title)
	)
}

fn markdown_to_html(md: &str, title: &str) -> String {
	use pulldown_cmark::{Options, Parser, html};
	let opts = Options::ENABLE_TABLES
		| Options::ENABLE_STRIKETHROUGH
		| Options::ENABLE_TASKLISTS
		| Options::ENABLE_FOOTNOTES
		| Options::ENABLE_HEADING_ATTRIBUTES;
	let mut body = String::new();
	html::push_html(&mut body, Parser::new_ext(md, opts));
	let heading = md
		.lines()
		.find_map(|l| l.strip_prefix("# "))
		.map(str::trim)
		.unwrap_or(title);
	page(heading, &body)
}

fn markdown_to_text(md: &str) -> String {
	use pulldown_cmark::{Event, Parser, Tag, TagEnd};
	let mut out = String::new();
	for ev in Parser::new(md) {
		match ev {
			Event::Text(t) | Event::Code(t) => out.push_str(&t),
			Event::SoftBreak => out.push(' '),
			Event::HardBreak => out.push('\n'),
			Event::Start(Tag::Item) => out.push_str("• "),
			Event::End(TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::CodeBlock) => out.push('\n'),
			Event::End(TagEnd::BlockQuote(_) | TagEnd::List(_)) => out.push('\n'),
			_ => {}
		}
	}
	let mut s = out.replace("\n\n\n", "\n\n");
	if !s.ends_with('\n') {
		s.push('\n');
	}
	s
}

fn html_to_text(html: &str) -> String {
	html2text::from_read(html.as_bytes(), 100).unwrap_or_default()
}

fn html_to_markdown(html: &str) -> String {
	htmd::convert(html).unwrap_or_default() + "\n"
}

/// The words of an RTF file, paragraph by paragraph.
pub fn rtf_to_text(rtf: &str) -> String {
	let skip_groups = [
		"fonttbl",
		"colortbl",
		"stylesheet",
		"info",
		"pict",
		"object",
		"header",
		"footer",
		"themedata",
		"datastore",
		"latentstyles",
		"listtable",
		"listoverridetable",
		"rsidtbl",
		"generator",
		"xmlnstbl",
		"mmathPr",
		"*",
	];
	let b = rtf.as_bytes();
	let mut out = String::new();
	// \'xx bytes are in the document's code page, and a character can take two of them
	let codepage = rtf
		.find("\\ansicpg")
		.map(|at| &rtf[at + 8..])
		.and_then(|r| {
			r[..r.find(|c: char| !c.is_ascii_digit()).unwrap_or(r.len())]
				.parse::<u32>()
				.ok()
		})
		.and_then(|n| {
			let label = match n {
				932 => "shift_jis".to_string(),
				936 => "gbk".to_string(),
				949 => "euc-kr".to_string(),
				950 => "big5".to_string(),
				65001 => "utf-8".to_string(),
				n => format!("windows-{n}"),
			};
			encoding_rs::Encoding::for_label(label.as_bytes())
		})
		.unwrap_or(encoding_rs::WINDOWS_1252);
	let mut pending: Vec<u8> = Vec::new();
	let flush = |pending: &mut Vec<u8>, out: &mut String| {
		if !pending.is_empty() {
			out.push_str(&codepage.decode_without_bom_handling(pending).0);
			pending.clear();
		}
	};
	// per group: whether its text is hidden
	let mut stack: Vec<bool> = vec![false];
	let mut skip_next_chars = 0usize;
	let mut uc = 1usize;
	let mut i = 0;
	while i < b.len() {
		let hidden = *stack.last().unwrap_or(&false);
		if b[i] != b'\\' || b.get(i + 1) != Some(&b'\'') {
			flush(&mut pending, &mut out);
		}
		match b[i] {
			b'{' => {
				stack.push(hidden);
				i += 1;
			}
			b'}' => {
				stack.pop();
				if stack.is_empty() {
					stack.push(false);
				}
				i += 1;
			}
			b'\\' => {
				i += 1;
				if i >= b.len() {
					break;
				}
				let c = b[i];
				if c == b'\'' {
					// two hex digits; anything else is a stray quote and its bytes stay text
					let hex = b.get(i + 1..i + 3).filter(|h| h.iter().all(u8::is_ascii_hexdigit));
					let Some(hex) = hex else {
						i += 1;
						continue;
					};
					let v = u8::from_str_radix(std::str::from_utf8(hex).unwrap_or("0"), 16).unwrap_or(0);
					if !hidden && skip_next_chars == 0 {
						pending.push(v);
					}
					skip_next_chars = skip_next_chars.saturating_sub(1);
					i += 3;
				} else if c.is_ascii_alphabetic() {
					let start = i;
					while i < b.len() && b[i].is_ascii_alphabetic() {
						i += 1;
					}
					let word = &rtf[start..i];
					let nstart = i;
					if i < b.len() && (b[i] == b'-' || b[i].is_ascii_digit()) {
						i += 1;
						while i < b.len() && b[i].is_ascii_digit() {
							i += 1;
						}
					}
					let num: Option<i32> = rtf[nstart..i].parse().ok();
					if i < b.len() && b[i] == b' ' {
						i += 1;
					}
					if skip_groups.contains(&word)
						&& let Some(top) = stack.last_mut()
					{
						*top = true;
					}
					if hidden {
						continue;
					}
					match word {
						"par" | "line" | "sect" | "page" => out.push('\n'),
						"tab" | "cell" => out.push('\t'),
						"row" => out.push('\n'),
						"uc" => uc = num.unwrap_or(1).max(0) as usize,
						"u" => {
							if let Some(n) = num {
								let code = if n < 0 { (n + 65536) as u32 } else { n as u32 };
								if let Some(ch) = char::from_u32(code) {
									out.push(ch);
								}
								skip_next_chars = uc;
							}
						}
						"emdash" => out.push('—'),
						"endash" => out.push('–'),
						"bullet" => out.push('•'),
						"lquote" => out.push('‘'),
						"rquote" => out.push('’'),
						"ldblquote" => out.push('“'),
						"rdblquote" => out.push('”'),
						_ => {}
					}
				} else {
					if c == b'*'
						&& let Some(top) = stack.last_mut()
					{
						*top = true;
					} else if !hidden {
						match c {
							b'\\' | b'{' | b'}' => out.push(c as char),
							b'~' => out.push('\u{a0}'),
							b'\n' | b'\r' => out.push('\n'),
							_ => {}
						}
					}
					// a control symbol can be any character, so step over all of it
					i += rtf[i..].chars().next().map_or(1, char::len_utf8);
				}
			}
			b'\r' | b'\n' => i += 1,
			_ => {
				let start = i;
				while i < b.len() && !matches!(b[i], b'\\' | b'{' | b'}' | b'\r' | b'\n') {
					i += 1;
				}
				if !hidden {
					let chunk = &rtf[start..i];
					let skip = skip_next_chars.min(chunk.chars().count());
					out.extend(chunk.chars().skip(skip));
					skip_next_chars -= skip;
				}
			}
		}
	}
	flush(&mut pending, &mut out);
	let cleaned: Vec<&str> = out.lines().map(str::trim_end).collect();
	cleaned.join("\n").trim().to_string() + "\n"
}

fn text_to_html(text: &str, title: &str) -> String {
	let body: String = text
		.split("\n\n")
		.filter(|p| !p.trim().is_empty())
		.map(|p| format!("<p>{}</p>\n", xml_escape(p.trim()).replace('\n', "<br>\n")))
		.collect();
	page(title, &body)
}

pub fn convert(input: &Path, to: Format) -> Result<String, Error> {
	let ext = ext_of(input);
	let limit = match ext.as_str() {
		"csv" | "tsv" | "psv" => CSV_LIMIT,
		"json" | "jsonl" | "ndjson" | "geojson" | "json5" | "yaml" | "yml" | "toml" | "xml" | "plist" | "html"
		| "htm" | "xhtml" => DATA_LIMIT,
		_ => READ_LIMIT,
	};
	let text = read_within(input, limit)?;
	let title = crate::plan::stem_of(input);
	let data_in = matches!(
		ext.as_str(),
		"json"
			| "jsonl" | "ndjson"
			| "geojson"
			| "json5" | "yaml"
			| "yml" | "toml"
			| "xml" | "plist"
			| "csv" | "tsv"
			| "psv" | "ini"
			| "cfg" | "conf"
			| "properties"
			| "env"
	);
	if data_in && to.is_data() {
		return write_data(&parse_data(&text, &ext)?, to);
	}
	if matches!(ext.as_str(), "srt" | "vtt" | "ass" | "ssa" | "sub") {
		return Ok(crate::subs::convert(&text, to));
	}
	let md = matches!(ext.as_str(), "md" | "markdown" | "mdown" | "mkd");
	let html = matches!(ext.as_str(), "html" | "htm" | "xhtml");
	if html && tag_depth(&text) > MAX_DEPTH {
		return Err(too_deep());
	}
	Ok(match to {
		Format::Html if md => markdown_to_html(&text, &title),
		Format::Txt if md => markdown_to_text(&text),
		Format::Md if html => html_to_markdown(&text),
		Format::Txt if html => html_to_text(&text),
		Format::Txt if ext == "rtf" => rtf_to_text(&text),
		Format::Md if ext == "rtf" => rtf_to_text(&text).replace('\n', "\n\n"),
		Format::Html if ext == "rtf" => text_to_html(&rtf_to_text(&text), &title),
		Format::Html => text_to_html(&text, &title),
		Format::Md | Format::Txt => text,
		other => return Err(Error::Engine(format!("can't make {} from this file", other.label()))),
	})
}

fn natural_key(s: &str) -> (Option<f64>, String) {
	let t = s.trim();
	let lead: String = t
		.chars()
		.take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | ','))
		.filter(|c| *c != ',')
		.collect();
	(lead.parse().ok(), t.to_lowercase())
}

pub(crate) fn title_case(s: &str) -> String {
	const SMALL: &[&str] = &[
		"a", "an", "and", "as", "at", "but", "by", "for", "in", "of", "on", "or", "the", "to", "vs", "via",
	];
	let mut first = true;
	s.split(' ')
		.map(|w| {
			let lower = w.to_lowercase();
			let out = if !first && SMALL.contains(&lower.as_str()) {
				lower
			} else {
				let mut c = lower.chars();
				match c.next() {
					Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
					None => String::new(),
				}
			};
			if !w.is_empty() {
				first = false;
			}
			out
		})
		.collect::<Vec<_>>()
		.join(" ")
}

pub(crate) fn sentence_case(s: &str) -> String {
	let mut out = String::with_capacity(s.len());
	let mut start = true;
	for ch in s.chars() {
		if start && ch.is_alphabetic() {
			out.extend(ch.to_uppercase());
			start = false;
		} else {
			out.extend(ch.to_lowercase());
		}
		if matches!(ch, '.' | '!' | '?' | '\n') {
			start = true;
		}
	}
	out
}

/// The indentation unit a file uses: 0 for tabs, otherwise the smallest step of spaces.
fn indent_unit(lines: &[&str]) -> usize {
	if lines.iter().filter(|l| l.starts_with('\t')).count() > lines.iter().filter(|l| l.starts_with("  ")).count() {
		return 0;
	}
	// the step the indentation most often goes up by; block-comment lines like " * text" sit one
	// space in and say nothing about it
	let mut steps: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
	let mut prev = 0;
	for l in lines {
		let body = l.trim_start_matches(' ');
		if body.is_empty() || body.starts_with('*') {
			continue;
		}
		let width = l.len() - body.len();
		if width > prev {
			*steps.entry(width - prev).or_default() += 1;
		}
		prev = width;
	}
	// the most common step wins, the smaller one on a tie
	steps
		.iter()
		.max_by_key(|(step, n)| (**n, std::cmp::Reverse(**step)))
		.map(|(step, _)| *step)
		.unwrap_or(4)
		.clamp(1, 8)
}

fn reindent(text: &str, spaces: u8) -> String {
	let ls = lines(text);
	let unit = indent_unit(&ls);
	let out: Vec<String> = ls
		.iter()
		.map(|l| {
			let body = l.trim_start_matches([' ', '\t']);
			let lead = &l[..l.len() - body.len()];
			let width: usize = lead.chars().map(|c| if c == '\t' { unit.max(1) } else { 1 }).sum();
			// a unit of 0 means the file indents with tabs
			let level = width
				.checked_div(unit)
				.unwrap_or_else(|| lead.chars().filter(|c| *c == '\t').count());
			let rest = width.checked_rem(unit).unwrap_or(0);
			let new = if spaces == 0 {
				"\t".repeat(level) + " ".repeat(rest).as_str()
			} else {
				" ".repeat(level * spaces as usize + rest)
			};
			new + body
		})
		.collect();
	join(&out, crlf(text))
}

/// Line and text tools that read one file and write one file.
pub fn tool(input: &Path, r: &Recipe) -> Result<Vec<u8>, Error> {
	let ext = ext_of(input);
	let limit = match r {
		Recipe::Pretty | Recipe::Minify => format_limit(&ext),
		_ => LINE_LIMIT,
	};
	check_size(input, limit)?;
	let (text, stored) = decode_stored(&fs::read(input)?);
	let nl = crlf(&text);
	let s = match r {
		Recipe::ToUtf8 => return Ok(text.into_bytes()),
		Recipe::LineEndings { crlf } => join(&lines(&text), *crlf),
		Recipe::CleanUp => {
			let mut out: Vec<&str> = Vec::new();
			let mut blank = 0;
			for l in text.lines().map(str::trim_end) {
				if l.is_empty() {
					blank += 1;
					if blank > 1 || out.is_empty() {
						continue;
					}
				} else {
					blank = 0;
				}
				out.push(l);
			}
			while out.last().is_some_and(|l| l.is_empty()) {
				out.pop();
			}
			join(&out, nl)
		}
		Recipe::SortLines { by } => {
			let header = matches!(ext.as_str(), "csv" | "tsv");
			let mut ls = lines(&text);
			let head: Vec<&str> = if header && !ls.is_empty() {
				vec![ls.remove(0)]
			} else {
				Vec::new()
			};
			match by {
				SortBy::Az => ls.sort_by_key(|l| natural_key(l).1),
				SortBy::Za => {
					ls.sort_by_key(|l| natural_key(l).1);
					ls.reverse();
				}
				SortBy::Number => ls.sort_by(|a, b| {
					let (ka, kb) = (natural_key(a), natural_key(b));
					match (ka.0, kb.0) {
						(Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
						(Some(_), None) => std::cmp::Ordering::Less,
						(None, Some(_)) => std::cmp::Ordering::Greater,
						_ => ka.1.cmp(&kb.1),
					}
				}),
				SortBy::Length => ls.sort_by_key(|l| l.chars().count()),
			}
			join(&[head, ls].concat(), nl)
		}
		Recipe::Dedupe => {
			let mut seen = std::collections::HashSet::new();
			let out: Vec<&str> = text
				.lines()
				.filter(|l| l.trim().is_empty() || seen.insert(*l))
				.collect();
			join(&out, nl)
		}
		Recipe::ChangeCase { to } => match to {
			Case::Upper => text.to_uppercase(),
			Case::Lower => text.to_lowercase(),
			Case::Title => {
				text.lines()
					.map(title_case)
					.collect::<Vec<_>>()
					.join(if nl { "\r\n" } else { "\n" })
					+ if text.ends_with('\n') {
						if nl { "\r\n" } else { "\n" }
					} else {
						""
					}
			}
			Case::Sentence => sentence_case(&text),
		},
		Recipe::Indent { spaces } => reindent(&text, *spaces),
		Recipe::Pretty => crate::code::pretty(&text, &ext)?,
		Recipe::Minify => crate::code::minify(&text, &ext)?,
		_ => return Err(Error::Engine("this tool doesn't fit text files".into())),
	};
	// everything but Fix encoding writes the file back the way it was stored
	Ok(encode_stored(&s, stored))
}

/// Joins text files. CSV and TSV keep one header; JSON lists become one list.
pub fn merge(inputs: &[PathBuf]) -> Result<String, Error> {
	let ext = ext_of(&inputs[0]);
	let total: u64 = inputs
		.iter()
		.map(|i| fs::metadata(i).map(|m| m.len()).unwrap_or(0))
		.sum();
	let limit = if ext == "json" { DATA_LIMIT } else { READ_LIMIT };
	if total > limit.ceiling {
		let limit = limit.ceiling;
		return Err(Error::Engine(format!(
			"these files are too big to merge ({} MB; it works up to {} MB)",
			total.div_ceil(MB),
			limit / MB
		)));
	}
	crate::budget::fits_now(total.saturating_mul(limit.copies))
		.map_err(|e| Error::Engine(format!("these files are too big to merge right now: {e}")))?;
	if matches!(ext.as_str(), "json") {
		let mut all = Vec::new();
		for i in inputs {
			match parse_data(&read(i)?, "json")? {
				Value::Array(a) => all.extend(a),
				other => all.push(other),
			}
		}
		return write_data(&Value::Array(all), Format::Json);
	}
	let mut out = String::new();
	for (n, i) in inputs.iter().enumerate() {
		let text = read(i)?;
		let body = if n > 0 && matches!(ext.as_str(), "csv" | "tsv") {
			text.split_once('\n')
				.map(|(_, rest)| rest.to_string())
				.unwrap_or_default()
		} else {
			text
		};
		out.push_str(&body);
		if !out.ends_with('\n') {
			out.push('\n');
		}
	}
	Ok(out)
}

/// Cuts a file into parts of `per` lines, repeating a CSV header in every part.
pub fn split(input: &Path, per: usize, dir: &Path) -> Result<usize, Error> {
	let text = read(input)?;
	let ext = ext_of(input);
	let stem = crate::plan::stem_of(input);
	let mut ls = lines(&text);
	let header = if matches!(ext.as_str(), "csv" | "tsv") && !ls.is_empty() {
		Some(ls.remove(0))
	} else {
		None
	};
	fs::create_dir_all(dir)?;
	let nl = crlf(&text);
	let mut n = 0;
	for (i, chunk) in ls.chunks(per.max(1)).enumerate() {
		let mut part: Vec<&str> = header.into_iter().collect();
		part.extend(chunk);
		let name = if ext.is_empty() {
			format!("{stem} {}", i + 1)
		} else {
			format!("{stem} {}.{ext}", i + 1)
		};
		fs::write(dir.join(name), join(&part, nl))?;
		n += 1;
	}
	Ok(n)
}

/// Lines, words, characters and reading time.
pub fn stats(input: &Path) -> Result<String, Error> {
	let text = read(input)?;
	let lines = text.lines().count();
	let words = text.split_whitespace().count();
	let chars = text.chars().filter(|c| *c != '\r' && *c != '\n').count();
	let minutes = (words as f32 / 230.0).ceil() as usize;
	let fmt = |n: usize| {
		let s = n.to_string();
		let mut out = String::new();
		for (i, ch) in s.chars().enumerate() {
			if i > 0 && (s.len() - i).is_multiple_of(3) {
				out.push(',');
			}
			out.push(ch);
		}
		out
	};
	Ok(format!(
		"{} lines, {} words, {} characters, about {} min to read",
		fmt(lines),
		fmt(words),
		fmt(chars),
		minutes.max(1)
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn csv_to_json_keeps_types_and_rebuilds_nesting() {
		let v = parse_data("id,name,zip,address.city,ok\n1,Ann,007,Cairo,true\n", "csv").unwrap();
		let row = &v[0];
		assert_eq!(row["id"], 1);
		assert_eq!(row["zip"], "007");
		assert_eq!(row["address"]["city"], "Cairo");
		assert_eq!(row["ok"], true);
	}

	#[test]
	fn json_to_csv_flattens() {
		let v: Value = serde_json::from_str(r#"{"users":[{"n":"a","t":{"x":1}},{"n":"b","tags":["p","q"]}]}"#).unwrap();
		let csv = write_data(&v, Format::Csv).unwrap();
		assert_eq!(csv, "n,t.x,tags\na,1,\nb,,p; q\n");
	}

	#[test]
	fn xml_round_trip_through_json() {
		let v = parse_xml(r#"<shop id="7"><item>a</item><item>b</item><name>Corner</name></shop>"#).unwrap();
		assert_eq!(v["shop"]["@id"], 7);
		assert_eq!(v["shop"]["item"][1], "b");
		let xml = write_xml(&v);
		assert!(xml.contains("<shop id=\"7\">"));
		assert!(xml.contains("<item>b</item>"));
	}

	#[test]
	fn toml_drops_nulls_and_wraps_lists() {
		let v: Value = serde_json::from_str(r#"[{"a":1,"b":null}]"#).unwrap();
		let t = write_data(&v, Format::Toml).unwrap();
		assert!(t.contains("[[items]]") && t.contains("a = 1") && !t.contains("b ="));
	}

	#[test]
	fn ini_sections_nest() {
		let v = parse_data("top=1\n[db]\nhost = \"x\"\n; note\nport: 5432\n", "ini").unwrap();
		assert_eq!(v["top"], 1);
		assert_eq!(v["db"]["host"], "x");
		assert_eq!(v["db"]["port"], 5432);
	}

	#[test]
	fn rtf_with_non_ascii_after_a_backslash() {
		assert_eq!(rtf_to_text("{\\rtf1\\ansi Hello \\'1é world\\par}"), "Hello 1é world\n");
		assert_eq!(rtf_to_text("{\\rtf1\\ansi Hello \\é world\\par}"), "Hello  world\n");
		assert_eq!(rtf_to_text("C:\\Élèves\\notes"), "C:lèves\n");
	}

	#[test]
	fn files_past_a_tool_limit_are_refused_before_reading() {
		let dir = std::env::temp_dir().join(format!("tw-size-{}", std::process::id()));
		fs::create_dir_all(&dir).unwrap();
		// sparse files: the size is there without the disk use
		let log = dir.join("big.log");
		fs::File::create(&log).unwrap().set_len(LINE_LIMIT.ceiling + 1).unwrap();
		let err = tool(&log, &Recipe::SortLines { by: SortBy::Az })
			.unwrap_err()
			.to_string();
		assert!(err.starts_with("this file is too big for this tool"), "{err}");
		let json = dir.join("big.json");
		fs::File::create(&json)
			.unwrap()
			.set_len(DATA_LIMIT.ceiling + 1)
			.unwrap();
		let err = convert(&json, Format::Yaml).unwrap_err().to_string();
		assert!(err.starts_with("this file is too big"), "{err}");
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn a_file_under_the_ceiling_still_waits_for_free_memory() {
		// 100 MB through the line tools holds about 400 MB
		assert!(check_len(100 * MB, LINE_LIMIT, Some(4096 * MB)).is_ok());
		let err = check_len(100 * MB, LINE_LIMIT, Some(600 * MB)).unwrap_err().to_string();
		assert!(
			err.starts_with("this file is too big for this tool right now: this needs about 400 MB of memory"),
			"{err}"
		);
		assert!(check_len(500 * MB, LINE_LIMIT, Some(64 * 1024 * MB)).is_err());
	}

	#[test]
	fn csv_from_uneven_rows_keeps_first_seen_column_order() {
		let v: Value = serde_json::from_str(r#"[{"b":1,"a":2},{"c":3,"b":4}]"#).unwrap();
		assert_eq!(write_csv(&v, b',').unwrap(), "b,a,c\n1,2,\n4,,3\n");
	}

	#[test]
	fn csv_columns_survive_a_round_trip() {
		let v = parse_data("name,name,a,a.b,zeta,alpha\nfirst,second,x,y,1,2\n", "csv").unwrap();
		assert_eq!(
			serde_json::to_string(&v).unwrap(),
			r#"[{"name":"first","name_2":"second","a":"x","a.b":"y","zeta":1,"alpha":2}]"#
		);
		assert_eq!(
			write_data(&v, Format::Csv).unwrap(),
			"name,name_2,a,a.b,zeta,alpha\nfirst,second,x,y,1,2\n"
		);
	}

	#[test]
	fn nesting_past_the_limit_is_refused() {
		let xml = "<a>".repeat(20_000) + &"</a>".repeat(20_000)[..];
		assert_eq!(parse_data(&xml, "xml").unwrap_err().to_string(), too_deep().to_string());
		let toml = format!("x = {}{}", "[".repeat(50_000), "]".repeat(50_000));
		assert!(parse_data(&toml, "toml").is_err());
		let dir = std::env::temp_dir().join(format!("tw-deep-{}", std::process::id()));
		fs::create_dir_all(&dir).unwrap();
		let page = dir.join("deep.html");
		fs::write(&page, "<div>x".repeat(6_000)).unwrap();
		assert_eq!(
			convert(&page, Format::Md).unwrap_err().to_string(),
			too_deep().to_string()
		);
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn ordinary_html_isnt_deep() {
		let list = format!("<ul>{}</ul>", "<li>item<p>text".repeat(2_000));
		assert!(tag_depth(&list) < 5);
		assert_eq!(tag_depth("<div><span>a</span><br><img src=x/></div>"), 2);
	}

	#[test]
	fn one_bad_byte_doesnt_change_the_encoding() {
		let text = decode(b"Caf\xc3\xa9 cr\xc3\xa8me\nna\xc3\xafve\nstray byte \xff\n");
		assert!(text.starts_with("Café crème\nnaïve\n"), "{text}");
		assert_eq!(decode(b"Caf\xe9 cr\xe8me\n"), "Café crème\n");
	}

	#[test]
	fn tools_write_back_in_the_files_own_encoding() {
		let dir = std::env::temp_dir().join(format!("tw-enc-{}", std::process::id()));
		fs::create_dir_all(&dir).unwrap();
		let cases: [(&str, Vec<u8>, Vec<u8>); 3] = [
			(
				"bom.csv",
				b"\xef\xbb\xbfb\n\xc3\xa9\n".to_vec(),
				b"\xef\xbb\xbfb\n\xc3\xa9\n".to_vec(),
			),
			(
				"old.bat",
				b"echo Caf\xe9\r\necho B\r\n".to_vec(),
				b"echo B\r\necho Caf\xe9\r\n".to_vec(),
			),
			(
				"wide.reg",
				[
					&[0xffu8, 0xfe][..],
					&"z\r\na\r\n"
						.encode_utf16()
						.flat_map(u16::to_le_bytes)
						.collect::<Vec<u8>>(),
				]
				.concat(),
				[
					&[0xffu8, 0xfe][..],
					&"a\r\nz\r\n"
						.encode_utf16()
						.flat_map(u16::to_le_bytes)
						.collect::<Vec<u8>>(),
				]
				.concat(),
			),
		];
		for (name, input, want) in cases {
			let p = dir.join(name);
			fs::write(&p, input).unwrap();
			let by = if name == "bom.csv" { SortBy::Za } else { SortBy::Az };
			assert_eq!(tool(&p, &Recipe::SortLines { by }).unwrap(), want, "{name}");
		}
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn rtf_reads_its_code_page() {
		assert_eq!(
			rtf_to_text("{\\rtf1\\ansi\\ansicpg1251 \\'cf\\'f0\\'e8\\'e2\\'e5\\'f2}"),
			"Привет\n"
		);
		assert_eq!(rtf_to_text("{\\rtf1\\ansi\\ansicpg932 \\'93\\'fa\\'96\\'7b}"), "日本\n");
		assert_eq!(rtf_to_text("{\\rtf1\\ansi Caf\\'e9}"), "Café\n");
	}

	#[test]
	fn indent_isnt_fooled_by_comment_lines() {
		let js = "/**\n * doc\n */\nfunction f() {\n    if (x) {\n        y();\n    }\n}\n";
		assert_eq!(
			reindent(js, 2),
			"/**\n * doc\n */\nfunction f() {\n  if (x) {\n    y();\n  }\n}\n"
		);
	}

	#[test]
	fn ini_key_named_like_a_later_section() {
		let v = parse_ini("db=1\n[db]\nhost=x\n");
		assert_eq!(v["db"]["host"], "x");
	}

	#[test]
	fn rtf_text_comes_out() {
		let rtf = r"{\rtf1\ansi{\fonttbl{\f0 Arial;}}\f0 Hello {\b bold} world\par Caf\'e9 \u8364?5\par}";
		assert_eq!(rtf_to_text(rtf), "Hello bold world\nCafé €5\n");
	}

	#[test]
	fn markdown_becomes_a_page() {
		let h = markdown_to_html("# Notes\n\n| a | b |\n|---|---|\n| 1 | 2 |\n", "x");
		assert!(h.contains("<title>Notes</title>") && h.contains("<table>"));
		assert_eq!(markdown_to_text("# Hi\n\nsome *text*\n"), "Hi\nsome text\n");
	}

	#[test]
	fn encodings_are_detected() {
		let latin = encoding_rs::WINDOWS_1252.encode("Café crème brûlée à la française").0;
		assert_eq!(decode(&latin), "Café crème brûlée à la française");
		assert_eq!(decode(b"\xef\xbb\xbfhi"), "hi");
	}

	#[test]
	fn indentation_converts() {
		assert_eq!(reindent("a\n    b\n        c\n", 0), "a\n\tb\n\t\tc\n");
		assert_eq!(reindent("a\n\tb\n", 2), "a\n  b\n");
	}

	#[test]
	fn cases_and_sorting() {
		assert_eq!(title_case("the lord of the rings"), "The Lord of the Rings");
		assert_eq!(sentence_case("HELLO THERE. how ARE you?"), "Hello there. How are you?");
		assert_eq!(natural_key("10 apples").0, Some(10.0));
	}
}
