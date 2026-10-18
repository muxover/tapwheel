//! Reading text aloud with the voices built into Windows.

use std::path::Path;

use crate::Error;

/// The parts of a WAV file we need to join several together.
pub struct Wav {
	pub fmt: Vec<u8>,
	pub data: Vec<u8>,
}

pub fn parse_wav(b: &[u8]) -> Option<Wav> {
	if b.get(0..4)? != b"RIFF" || b.get(8..12)? != b"WAVE" {
		return None;
	}
	let (mut fmt, mut data) = (None, None);
	let mut i = 12;
	while i + 8 <= b.len() {
		let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().ok()?) as usize;
		let body = b.get(i + 8..(i + 8 + len).min(b.len()))?;
		match &b[i..i + 4] {
			b"fmt " => fmt = Some(body.to_vec()),
			b"data" => data = Some(body.to_vec()),
			_ => {}
		}
		i += 8 + len + (len & 1);
	}
	Some(Wav { fmt: fmt?, data: data? })
}

pub fn write_wav(fmt: &[u8], data: &[u8]) -> Vec<u8> {
	let mut out = b"RIFF".to_vec();
	out.extend(((4 + 8 + fmt.len() + 8 + data.len()) as u32).to_le_bytes());
	out.extend(b"WAVEfmt ");
	out.extend((fmt.len() as u32).to_le_bytes());
	out.extend(fmt);
	out.extend(b"data");
	out.extend((data.len() as u32).to_le_bytes());
	out.extend(data);
	out
}

/// Splits long text at paragraph and sentence ends so each piece is quick to speak.
pub fn chunks(text: &str, max: usize) -> Vec<String> {
	let mut out = Vec::new();
	let mut cur = String::new();
	for sentence in text.split_inclusive(['.', '!', '?', '\n']) {
		if cur.len() + sentence.len() > max && !cur.trim().is_empty() {
			out.push(std::mem::take(&mut cur));
		}
		cur.push_str(sentence);
	}
	if !cur.trim().is_empty() {
		out.push(cur);
	}
	out
}

#[cfg(windows)]
mod win {
	use windows::Media::SpeechSynthesis::SpeechSynthesizer;
	use windows::Storage::Streams::DataReader;
	use windows::core::HSTRING;

	use super::*;

	fn fail(e: windows::core::Error) -> Error {
		Error::Engine(format!("speech: {}", e.message()))
	}

	pub fn voices() -> Vec<(String, String)> {
		let Ok(all) = SpeechSynthesizer::AllVoices() else {
			return Vec::new();
		};
		all.into_iter()
			.filter_map(|v| Some((v.Id().ok()?.to_string(), v.DisplayName().ok()?.to_string())))
			.collect()
	}

	pub fn speak_wav(text: &str, voice: Option<&str>, rate: f32) -> Result<Vec<u8>, Error> {
		unsafe {
			let _ = windows_sys::Win32::System::Com::CoInitializeEx(
				std::ptr::null(),
				windows_sys::Win32::System::Com::COINIT_MULTITHREADED as u32,
			);
		}
		let synth = SpeechSynthesizer::new().map_err(fail)?;
		if let Some(id) = voice
			&& let Ok(all) = SpeechSynthesizer::AllVoices()
			&& let Some(v) = all
				.into_iter()
				.find(|v| v.Id().map(|i| i.to_string() == id).unwrap_or(false))
		{
			synth.SetVoice(&v).map_err(fail)?;
		}
		synth
			.Options()
			.map_err(fail)?
			.SetSpeakingRate(rate.clamp(0.5, 3.0) as f64)
			.map_err(fail)?;
		let mut fmt = None;
		let mut data = Vec::new();
		for piece in chunks(text, 2500) {
			let stream = synth
				.SynthesizeTextToStreamAsync(&HSTRING::from(piece))
				.map_err(fail)?
				.get()
				.map_err(fail)?;
			let size = stream.Size().map_err(fail)? as u32;
			let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).map_err(fail)?).map_err(fail)?;
			reader.LoadAsync(size).map_err(fail)?.get().map_err(fail)?;
			let mut buf = vec![0u8; size as usize];
			reader.ReadBytes(&mut buf).map_err(fail)?;
			let wav = parse_wav(&buf).ok_or_else(|| Error::Engine("the voice gave back nothing".into()))?;
			fmt.get_or_insert(wav.fmt);
			data.extend(wav.data);
		}
		let fmt = fmt.ok_or_else(|| Error::Engine("there's no text to read".into()))?;
		Ok(write_wav(&fmt, &data))
	}
}

#[cfg(windows)]
pub use win::voices;

#[cfg(not(windows))]
pub fn voices() -> Vec<(String, String)> {
	Vec::new()
}

/// Speaks `text` into a WAV file.
pub fn speak(text: &str, voice: Option<&str>, rate: f32, wav: &Path) -> Result<(), Error> {
	#[cfg(windows)]
	{
		std::fs::write(wav, win::speak_wav(text, voice, rate)?)?;
		Ok(())
	}
	#[cfg(not(windows))]
	{
		let _ = (text, voice, rate, wav);
		Err(Error::Engine("reading aloud needs Windows".into()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn wavs_join() {
		let fmt = vec![1, 0, 1, 0, 0x40, 0x1f, 0, 0, 0x80, 0x3e, 0, 0, 2, 0, 16, 0];
		let a = write_wav(&fmt, &[1, 2, 3, 4]);
		let back = parse_wav(&a).unwrap();
		assert_eq!(back.fmt, fmt);
		assert_eq!(back.data, [1, 2, 3, 4]);
	}

	#[test]
	fn long_text_splits_at_sentences() {
		let text = "One. Two! Three?\nFour.";
		let parts = chunks(text, 9);
		assert_eq!(parts.concat(), text);
		assert!(parts.iter().all(|p| p.len() <= 10));
	}
}
