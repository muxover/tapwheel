//! "Report a problem": a GitHub issue filled in with what helps find the problem, opened in the
//! browser for the user to read, change and post. TapWheel itself sends nothing.

/// GitHub turns away issue links much longer than this, so the log is cut to fit.
pub const MAX_URL: usize = 7500;
const NEW_ISSUE: &str = "https://github.com/muxover/tapwheel/issues/new";

/// What goes into the report besides the user's own words.
pub struct Report<'a> {
	pub version: &'a str,
	/// Like "Windows 11, build 26100".
	pub windows: &'a str,
	/// "Store" or "Portable".
	pub edition: &'a str,
	/// Like "1920x1080 at 150%, 2560x1440 at 100%".
	pub displays: &'a str,
	/// The newest lines of the log, oldest first.
	pub log: &'a [String],
	/// The user's profile folder, written as %USERPROFILE% in the log so their name stays out.
	pub home: Option<&'a str>,
}

/// The issue link. Log lines are dropped from the oldest end until the whole link fits.
pub fn issue_url(r: &Report) -> String {
	let hide = |line: &str| match r.home {
		Some(home) if !home.is_empty() => replace_ignore_case(line, home, "%USERPROFILE%"),
		_ => line.to_string(),
	};
	let lines: Vec<String> = r.log.iter().map(|l| hide(l)).collect();
	let mut from = 0;
	loop {
		let log = lines[from..].join("\n");
		let body = format!(
			"## What happened\n\n\n## What you expected\n\n\n## Steps to try it\n\n1. \n\n## This PC\n\n- TapWheel {} ({})\n- {}\n- Screens: {}\n\n## The end of the log\n\n```text\n{}\n```\n",
			r.version, r.edition, r.windows, r.displays, log
		);
		let url = format!("{NEW_ISSUE}?labels=bug&body={}", encode(&body));
		if url.len() <= MAX_URL || from >= lines.len() {
			return url;
		}
		from += 1;
	}
}

fn replace_ignore_case(text: &str, find: &str, with: &str) -> String {
	let lower = text.to_lowercase();
	let needle = find.to_lowercase();
	// lowercasing can change byte lengths outside ASCII; then only exact matches are replaced
	if lower.len() != text.len() || needle.len() != find.len() {
		return text.replace(find, with);
	}
	let mut out = String::new();
	let mut at = 0;
	while let Some(i) = lower[at..].find(&needle) {
		out.push_str(&text[at..at + i]);
		out.push_str(with);
		at += i + needle.len();
	}
	out.push_str(&text[at..]);
	out
}

/// Percent-encodes everything but letters, digits and `-_.~`, as a URL query value.
fn encode(s: &str) -> String {
	let mut out = String::with_capacity(s.len() * 3);
	for b in s.bytes() {
		if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
			out.push(b as char);
		} else {
			out.push_str(&format!("%{b:02X}"));
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn report(log: &[String]) -> String {
		issue_url(&Report {
			version: "0.2.0",
			windows: "Windows 11, build 26100",
			edition: "Store",
			displays: "1920x1080 at 150%",
			log,
			home: Some(r"C:\Users\Jax"),
		})
	}

	fn decode(s: &str) -> String {
		let b = s.as_bytes();
		let mut out = Vec::new();
		let mut i = 0;
		while i < b.len() {
			if b[i] == b'%' {
				out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
				i += 3;
			} else {
				out.push(b[i]);
				i += 1;
			}
		}
		String::from_utf8(out).unwrap()
	}

	#[test]
	fn the_report_carries_the_pc_and_hides_the_users_name() {
		let url = report(&[r"compress failed for c:\users\jax\Videos\trip.mp4".into()]);
		assert!(url.starts_with("https://github.com/muxover/tapwheel/issues/new?labels=bug&body="));
		let body = decode(url.split("body=").nth(1).unwrap());
		assert!(body.contains("TapWheel 0.2.0 (Store)"));
		assert!(body.contains("Windows 11, build 26100"));
		assert!(body.contains("1920x1080 at 150%"));
		assert!(body.contains(r"%USERPROFILE%\Videos\trip.mp4"), "{body}");
		assert!(!body.to_lowercase().contains("jax"));
	}

	#[test]
	fn a_long_log_is_cut_from_the_oldest_end_to_fit() {
		let log: Vec<String> = (0..2000)
			.map(|i| format!("line {i}: something happened here"))
			.collect();
		let url = report(&log);
		assert!(url.len() <= MAX_URL, "{} chars", url.len());
		let body = decode(url.split("body=").nth(1).unwrap());
		assert!(body.contains("line 1999:"), "the newest line stays");
		assert!(!body.contains("line 0:"), "the oldest goes first");
	}
}
