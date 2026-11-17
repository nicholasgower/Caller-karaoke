//! Shareable web links that carry a dance and its tune set.
//!
//! The browser build can encode the current session into the page URL, so a
//! caller can send a friend a link that rebuilds the same dance and the same
//! ordered tune set without a network request. The dance card and every tune's
//! ABC text travel inside the link: the payload is JSON, deflated, and written
//! as URL-safe base64 after a short format prefix.
//!
//! Nothing here performs I/O or touches the UI, and nothing is web-specific
//! except the URL the caller passes in. The codec is plain Rust so it is tested
//! on every target; only the browser reads the location and the clipboard.
//!
//! Links are untrusted input. A decode caps the encoded length, caps the
//! decompressed JSON, and validates every field before the app uses it.

use std::io::Write;

#[cfg(any(target_arch = "wasm32", test, feature = "headless"))]
use std::io::Read;

use flate2::Compression;
#[cfg(any(target_arch = "wasm32", test, feature = "headless"))]
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use serde::{Deserialize, Serialize};

use caller_core::{Dance, Tune};

/// URL argument that carries the payload.
pub(crate) const PARAM: &str = "set";
/// Prefix that names the payload format, before the base64 text.
const CODE_PREFIX: &str = "1.";
/// Current payload schema.
pub(crate) const SCHEMA_VERSION: u32 = 1;
/// Longest code accepted from a URL, in bytes. Generous, but bounded: a link is
/// parsed once at startup and must not be able to exhaust memory.
const MAX_CODE_LEN: usize = 512 * 1024;
/// Longest decompressed JSON accepted. Also bounds a compression bomb.
const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
/// Longest ordered tune list a link may carry.
const MAX_TUNES: usize = 100;
/// Longest dance card a link may carry.
const MAX_FIGURES: usize = 400;
const MAX_TITLE_LEN: usize = 512;
const MAX_TEXT_LEN: usize = 16 * 1024;
const MAX_FIGURE_TEXT_LEN: usize = 4 * 1024;
const MAX_ABC_LEN: usize = 256 * 1024;

/// The session data a share link carries.
///
/// The tune list is the tune set in play order, including the tune that was
/// playing. `current` picks which of them starts, so a friend hears the same
/// opening tune. Preferences are deliberately absent: the recipient keeps
/// their own band, role terms, and timing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct SharePayload {
    pub schema_version: u32,
    pub dance: Dance,
    pub tunes: Vec<Tune>,
    /// Index into `tunes` of the tune that was playing.
    pub current: usize,
    /// Whether **Cycle tunes** was on when the link was made.
    pub rotation: bool,
    /// How many 64-beat passes each tune was set to last.
    pub passes_per_tune: u32,
}

impl SharePayload {
    /// Reject a payload the app cannot safely or meaningfully load.
    ///
    /// Runs on both encode and decode: the same rules keep a hand-built link
    /// from smuggling in an enormous card, and keep the app from producing a
    /// link it would itself refuse.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "This link uses share format {}, which this version of Caller Karaoke does not understand.",
                self.schema_version
            ));
        }
        self.dance
            .validate()
            .map_err(|error| format!("The shared dance is not usable: {error}"))?;
        if self.dance.figures.len() > MAX_FIGURES {
            return Err(format!(
                "The shared dance has {} figures; the limit is {MAX_FIGURES}.",
                self.dance.figures.len()
            ));
        }
        check_text("dance title", &self.dance.title, MAX_TITLE_LEN)?;
        for (name, text) in [
            ("choreographer", &self.dance.choreographer),
            ("formation", &self.dance.formation),
            ("preamble", &self.dance.preamble),
            ("notes", &self.dance.notes),
            ("dance source id", &self.dance.source.id),
            ("dance source provider", &self.dance.source.provider),
            ("dance source URL", &self.dance.source.url),
        ] {
            check_text(name, text, MAX_TEXT_LEN)?;
        }
        for figure in &self.dance.figures {
            check_text("figure text", &figure.text, MAX_FIGURE_TEXT_LEN)?;
            if let Some(phrase) = &figure.phrase {
                check_text("figure phrase", phrase, MAX_TITLE_LEN)?;
            }
        }

        if self.tunes.is_empty() {
            return Err("The shared session has no tunes.".to_string());
        }
        if self.tunes.len() > MAX_TUNES {
            return Err(format!(
                "The shared session has {} tunes; the limit is {MAX_TUNES}.",
                self.tunes.len()
            ));
        }
        if self.current >= self.tunes.len() {
            return Err("The shared session points past the end of its tune list.".to_string());
        }
        for tune in &self.tunes {
            check_text("tune title", &tune.title, MAX_TITLE_LEN)?;
            for (name, text) in [
                ("tune composer", &tune.composer),
                ("tune key", &tune.key),
                ("tune meter", &tune.meter),
                ("tune rhythm", &tune.rhythm),
                ("tune credits", &tune.credits),
                ("source id", &tune.source.id),
                ("source provider", &tune.source.provider),
                ("source URL", &tune.source.url),
            ] {
                check_text(name, text, MAX_TEXT_LEN)?;
            }
            check_text("tune ABC", &tune.abc, MAX_ABC_LEN)?;
            if tune.abc.trim().is_empty() {
                return Err(format!(
                    "The shared tune \"{}\" has no ABC music.",
                    tune.title
                ));
            }
            if let Some(original) = &tune.original_abc {
                check_text("original tune ABC", original, MAX_ABC_LEN)?;
            }
        }
        Ok(())
    }
}

fn check_text(name: &str, text: &str, max: usize) -> Result<(), String> {
    if text.len() > max {
        return Err(format!(
            "The shared {name} is {} bytes; the limit is {max}.",
            text.len()
        ));
    }
    Ok(())
}

/// Encode a session as the text of a `?set=` argument.
pub(crate) fn encode(payload: &SharePayload) -> Result<String, String> {
    payload.validate()?;
    let json = serde_json::to_vec(payload)
        .map_err(|error| format!("Could not write the share link: {error}"))?;
    if json.len() > MAX_JSON_BYTES {
        return Err(format!(
            "This session is too large to share in a link ({} KB).",
            json.len() / 1024
        ));
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&json)
        .map_err(|error| format!("Could not compress the share link: {error}"))?;
    let compressed = encoder
        .finish()
        .map_err(|error| format!("Could not compress the share link: {error}"))?;
    let code = format!("{CODE_PREFIX}{}", encode_base64url(&compressed));
    if code.len() > MAX_CODE_LEN {
        return Err(format!(
            "This session is too large to share in a link ({} KB).",
            code.len() / 1024
        ));
    }
    Ok(code)
}

/// Decode a `?set=` argument back into a session.
///
/// Only the browser build and the tests can receive a link code.
#[cfg(any(target_arch = "wasm32", test, feature = "headless"))]
pub(crate) fn decode(code: &str) -> Result<SharePayload, String> {
    let code = code.trim();
    if code.is_empty() {
        return Err("The link is empty.".to_string());
    }
    if code.len() > MAX_CODE_LEN {
        return Err("The share link is too long.".to_string());
    }
    let body = code
        .strip_prefix(CODE_PREFIX)
        .ok_or_else(|| "This is not a Caller Karaoke share link.".to_string())?;
    let compressed = decode_base64url(body)?;
    let mut decoder = ZlibDecoder::new(compressed.as_slice()).take(MAX_JSON_BYTES as u64 + 1);
    let mut json = Vec::new();
    decoder
        .read_to_end(&mut json)
        .map_err(|error| format!("Could not read the share link: {error}"))?;
    if json.len() > MAX_JSON_BYTES {
        return Err("The share link expands to more data than a session can hold.".to_string());
    }
    let payload: SharePayload = serde_json::from_slice(&json)
        .map_err(|error| format!("The share link is not valid session data: {error}"))?;
    payload.validate()?;
    Ok(payload)
}

/// The `set` argument in a URL or bare query/fragment text.
///
/// Accepts a full URL from the address bar, a `?set=…` query, or a `#set=…`
/// fragment, so a link still opens if the argument was moved into the fragment
/// by a proxy or a hand edit.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn code_from_url(url: &str) -> Option<String> {
    let mut segments = Vec::new();
    if let Some((_, query)) = url.split_once('?') {
        segments.push(query.split('#').next().unwrap_or(""));
    }
    if let Some((_, fragment)) = url.split_once('#') {
        segments.push(fragment);
    }
    for segment in segments {
        for pair in segment.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key == PARAM {
                let value = value.split('#').next().unwrap_or(value).trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Put a code into a page URL, replacing any query or fragment it already has.
///
/// The payload goes in the fragment: browsers do not send it to the web server,
/// so a static or documentation host never sees the shared cards. The reader in
/// [`code_from_url`] accepts a query argument too, for hand-edited links.
///
/// A bare code is returned when `base` is empty, which keeps the tests and the
/// non-browser builds from inventing an origin.
pub(crate) fn link_for(base: &str, code: &str) -> String {
    let base = base.split(['?', '#']).next().unwrap_or(base);
    if base.is_empty() {
        format!("#{PARAM}={code}")
    } else {
        format!("{base}#{PARAM}={code}")
    }
}

// ---------------------------------------------------------------------------
// URL-safe base64 (RFC 4648 §5), without padding.
// ---------------------------------------------------------------------------

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn encode_base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(b2 & 0x3f) as usize] as char);
        }
    }
    out
}

#[cfg(any(target_arch = "wasm32", test, feature = "headless"))]
fn decode_base64url(text: &str) -> Result<Vec<u8>, String> {
    let text = text.trim_end_matches('=');
    if text.len() % 4 == 1 {
        return Err("The share link has an invalid length.".to_string());
    }
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for byte in text.bytes() {
        let value = base64_value(byte).ok_or_else(|| {
            "The share link contains characters that are not part of a link.".to_string()
        })?;
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    if buffer != 0 {
        return Err("The share link is corrupted.".to_string());
    }
    Ok(out)
}

#[cfg(any(target_arch = "wasm32", test, feature = "headless"))]
fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use caller_core::{Figure, SourceRef};

    fn dance() -> Dance {
        Dance {
            source: SourceRef::new("contradb", "2409", "https://contradb.com/dances/2409"),
            title: "Another Last Dance".into(),
            choreographer: "Erik Hoffman".into(),
            formation: "improper".into(),
            preamble: String::new(),
            notes: "A test card with ünïcode.".into(),
            figures: vec![
                Figure::new(Some("A1".into()), 8.0, "neighbors do si do once"),
                Figure::new(Some("A2".into()), 8.0, "ladles allemande left 1½"),
            ],
        }
    }

    fn tune(id: &str, title: &str) -> Tune {
        Tune {
            source: SourceRef::new(
                "abcsessiontunes",
                id,
                format!("https://thesession.org/tunes/{id}"),
            ),
            title: title.into(),
            composer: "Traditional".into(),
            key: "D".into(),
            meter: "4/4".into(),
            rhythm: "reel".into(),
            abc: format!("X:1\nT:{title}\nK:D\n|: FA | G2 |\n"),
            original_abc: None,
            credits: "From ABC Session Tunes".into(),
        }
    }

    fn payload() -> SharePayload {
        SharePayload {
            schema_version: SCHEMA_VERSION,
            dance: dance(),
            tunes: vec![tune("1", "Sandy Boys"), tune("2", "Belle of the Ball")],
            current: 1,
            rotation: true,
            passes_per_tune: 8,
        }
    }

    #[test]
    fn a_session_round_trips_through_a_link_code() {
        let payload = payload();
        let code = encode(&payload).expect("encode");
        assert!(code.starts_with(CODE_PREFIX), "code was {code}");
        assert!(
            code.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.'
            }),
            "every code character is URL-safe"
        );
        assert_eq!(decode(&code).expect("decode"), payload);
    }

    #[test]
    fn embedded_abc_survives_even_when_it_is_large() {
        let mut payload = payload();
        payload.tunes[0].abc = (0..200)
            .map(|index| format!("|: FA G2 BG | bar {index} :|\n"))
            .collect();
        let code = encode(&payload).expect("encode");
        let restored = decode(&code).expect("decode");
        assert_eq!(restored.tunes[0].abc, payload.tunes[0].abc);
    }

    #[test]
    fn a_code_is_much_smaller_than_the_raw_json() {
        let payload = payload();
        let code = encode(&payload).expect("encode");
        let json = serde_json::to_vec(&payload).expect("json");
        assert!(
            code.len() < json.len(),
            "deflate should shrink a card and its ABC: {} vs {}",
            code.len(),
            json.len()
        );
    }

    #[test]
    fn base64_round_trips_every_tail_length() {
        for length in 0..16 {
            let bytes: Vec<u8> = (0..length).map(|index| (index * 37 + 11) as u8).collect();
            let text = encode_base64url(&bytes);
            assert_eq!(decode_base64url(&text).expect("decode"), bytes);
            assert!(
                text.bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
                "no padding or reserved characters in {text}"
            );
        }
    }

    #[test]
    fn corrupt_codes_are_rejected_with_a_reason() {
        let good = encode(&payload()).expect("encode");
        let body = good.strip_prefix(CODE_PREFIX).expect("prefix");

        for (label, code) in [
            ("empty", String::new()),
            ("wrong format", "2.AAAA".to_string()),
            ("bad characters", format!("{CODE_PREFIX}{body}!")),
            (
                "cut short",
                format!("{CODE_PREFIX}{}", &body[..body.len() / 2]),
            ),
            (
                "not compressed",
                format!("{CODE_PREFIX}{}", encode_base64url(b"{}")),
            ),
        ] {
            assert!(decode(&code).is_err(), "{label} should not decode: {code}");
        }
    }

    #[test]
    fn a_payload_that_breaks_the_limits_is_refused_by_encode() {
        let mut too_many = payload();
        too_many.tunes = (0..=MAX_TUNES)
            .map(|index| tune(&index.to_string(), "Reel"))
            .collect();
        assert!(encode(&too_many).is_err());

        let mut no_music = payload();
        no_music.tunes[0].abc.clear();
        assert!(encode(&no_music).is_err());

        let mut past_the_end = payload();
        past_the_end.current = 2;
        assert!(encode(&past_the_end).is_err());

        let mut no_dance = payload();
        no_dance.dance.figures.clear();
        assert!(encode(&no_dance).is_err());
    }

    #[test]
    fn a_decompression_bomb_is_cut_off() {
        // A valid deflate stream whose JSON is far larger than a session may be.
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(" ".repeat(MAX_JSON_BYTES + 1024).as_bytes())
            .expect("write");
        let compressed = encoder.finish().expect("finish");
        let code = format!("{CODE_PREFIX}{}", encode_base64url(&compressed));
        assert!(decode(&code).is_err());
    }

    #[test]
    fn the_set_argument_is_read_from_a_query_or_a_fragment() {
        let code = "1.AbC-_";
        assert_eq!(
            code_from_url(&format!("https://example.org/play?set={code}")).as_deref(),
            Some(code)
        );
        assert_eq!(
            code_from_url(&format!("https://example.org/play?x=1&set={code}&y=2")).as_deref(),
            Some(code)
        );
        assert_eq!(
            code_from_url(&format!("https://example.org/play#set={code}")).as_deref(),
            Some(code)
        );
        assert_eq!(code_from_url("?set="), None);
        assert_eq!(code_from_url("https://example.org/play?dataset=1"), None);
        assert_eq!(code_from_url("https://example.org/play"), None);
    }

    #[test]
    fn a_link_replaces_an_old_query_and_fragment() {
        assert_eq!(
            link_for("https://example.org/play?old=1#frag", "1.ABC"),
            "https://example.org/play#set=1.ABC"
        );
        assert_eq!(link_for("", "1.ABC"), "#set=1.ABC");
    }
}
