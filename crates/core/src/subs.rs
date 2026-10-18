use crate::ops::Format;

#[derive(Debug, PartialEq)]
struct Cue {
	start: String,
	end: String,
	lines: Vec<String>,
}

fn is_timing(line: &str) -> bool {
	line.contains("-->")
}

/// Reads SRT or WebVTT. Both use the same `start --> end` line, so one parser covers them.
fn parse(text: &str) -> Vec<Cue> {
	let text = text
		.trim_start_matches('\u{feff}')
		.replace("\r\n", "\n")
		.replace('\r', "\n");
	let mut cues = Vec::new();
	for block in text.split("\n\n") {
		let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
		let Some(at) = lines.iter().position(|l| is_timing(l)) else {
			continue;
		};
		let mut parts = lines[at].split("-->");
		let start = parts.next().unwrap_or("").trim().to_string();
		// VTT cue settings ("align:start") follow the end time
		let end = parts
			.next()
			.unwrap_or("")
			.split_whitespace()
			.next()
			.unwrap_or("")
			.to_string();
		cues.push(Cue {
			start,
			end,
			lines: lines[at + 1..].iter().map(|l| l.to_string()).collect(),
		});
	}
	cues
}

/// `00:01:02,500`, `00:01:02.500` or VTT's short `01:02.500`, as SRT or VTT wants it.
fn time(t: &str, sep: char) -> String {
	let t = t.replace([',', '.'], &sep.to_string());
	let colons = t.matches(':').count();
	let t = if colons == 1 { format!("00:{t}") } else { t };
	if t.contains(sep) { t } else { format!("{t}{sep}000") }
}

fn strip_tags(line: &str) -> String {
	let mut out = String::with_capacity(line.len());
	let mut depth = 0;
	for c in line.chars() {
		match c {
			'<' | '{' => depth += 1,
			'>' | '}' if depth > 0 => depth -= 1,
			_ if depth == 0 => out.push(c),
			_ => {}
		}
	}
	out
}

pub fn convert(text: &str, to: Format) -> String {
	let cues = parse(text);
	let mut out = String::new();
	match to {
		Format::Vtt => {
			out.push_str("WEBVTT\n\n");
			for c in &cues {
				out.push_str(&format!("{} --> {}\n", time(&c.start, '.'), time(&c.end, '.')));
				for l in &c.lines {
					out.push_str(l);
					out.push('\n');
				}
				out.push('\n');
			}
		}
		Format::Srt => {
			for (i, c) in cues.iter().enumerate() {
				out.push_str(&format!(
					"{}\n{} --> {}\n",
					i + 1,
					time(&c.start, ','),
					time(&c.end, ',')
				));
				for l in &c.lines {
					out.push_str(l);
					out.push('\n');
				}
				out.push('\n');
			}
		}
		_ => {
			for c in &cues {
				let line = c.lines.iter().map(|l| strip_tags(l)).collect::<Vec<_>>().join(" ");
				let line = line.trim();
				if !line.is_empty() {
					out.push_str(line);
					out.push('\n');
				}
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	const SRT: &str = "1\r\n00:00:01,000 --> 00:00:02,500\r\nHello <i>there</i>\r\n\r\n2\r\n00:00:03,000 --> 00:00:04,000\r\nTwo\r\nlines\r\n";
	const VTT: &str = "WEBVTT\n\nNOTE made by hand\n\n00:01.000 --> 00:02.500 align:start\nHello\n\nintro\n00:00:03.000 --> 00:00:04.000\nAgain\n";

	#[test]
	fn srt_to_vtt() {
		let v = convert(SRT, Format::Vtt);
		assert!(v.starts_with("WEBVTT\n\n00:00:01.000 --> 00:00:02.500\nHello <i>there</i>\n"));
		assert!(v.contains("00:00:03.000 --> 00:00:04.000\nTwo\nlines\n"));
	}

	#[test]
	fn vtt_to_srt_numbers_cues_and_drops_settings() {
		let s = convert(VTT, Format::Srt);
		assert_eq!(
			s,
			"1\n00:00:01,000 --> 00:00:02,500\nHello\n\n2\n00:00:03,000 --> 00:00:04,000\nAgain\n\n"
		);
	}

	#[test]
	fn plain_text_has_no_timing_or_tags() {
		assert_eq!(convert(SRT, Format::Txt), "Hello there\nTwo lines\n");
	}

	#[test]
	fn round_trip_keeps_cues() {
		let back = convert(&convert(SRT, Format::Vtt), Format::Srt);
		assert_eq!(parse(&back), parse(SRT));
	}
}
