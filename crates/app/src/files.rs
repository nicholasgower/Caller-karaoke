//! Local file transfer: exporting saved content and importing ABC and dance
//! JSON files.
//!
//! The platform split lives at the bottom of this module. Everything above it
//! is pure: serializing the dance document, building a [`Tune`] from ABC text,
//! and making a title safe for a file name. That keeps the formats testable
//! without a window, a browser, or an Android device.

use std::path::Path;

use caller_core::{Dance, Figure, SourceRef, Tune};
use sha2::{Digest, Sha256};

/// Marker written at the top of an exported dance document.
///
/// A bare [`Dance`] is also accepted on import, so the marker exists to reject
/// an unrelated JSON object rather than to make the file readable.
pub const DANCE_FORMAT: &str = "caller-karaoke-dance";
/// Markers still accepted on import.
///
/// Files exported under the app's earlier name carry this marker. Reading them
/// keeps a caller's saved cards usable without a conversion step; new exports
/// always write [`DANCE_FORMAT`].
pub const LEGACY_DANCE_FORMATS: &[&str] = &["contra-karaoke-dance"];
/// Schema version of the exported dance document.
pub const DANCE_VERSION: u32 = 1;
/// Provider key used for content that did not come from a remote source.
pub const LOCAL_PROVIDER: &str = "local";
/// Largest file the importers will read. A tune or card is a few kilobytes;
/// the cap keeps a mistaken pick from loading a video into memory.
#[cfg_attr(
    target_os = "android",
    allow(dead_code) // the Android picker returns text, not a file path
)]
pub const MAX_IMPORT_BYTES: u64 = 4 * 1024 * 1024;
/// Longest title-derived file name, in characters.
const MAX_FILE_STEM: usize = 80;

/// Which kind of file a dialog is choosing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportKind {
    Abc,
    DanceJson,
}

// The accessors are shared by three platform modules; a build compiles only
// one of them, so the others' accessors look unused to the lint.
#[allow(dead_code)]
impl ImportKind {
    /// Stable code shared with the Android activity.
    pub fn code(self) -> i32 {
        match self {
            Self::Abc => 1,
            Self::DanceJson => 2,
        }
    }

    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            1 => Some(Self::Abc),
            2 => Some(Self::DanceJson),
            _ => None,
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Abc => &["abc"],
            Self::DanceJson => &["json"],
        }
    }

    pub fn filter_name(self) -> &'static str {
        match self {
            Self::Abc => "ABC tune",
            Self::DanceJson => "Dance JSON",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Self::Abc => "text/vnd.abc",
            Self::DanceJson => "application/json",
        }
    }

    /// The `accept` attribute for the browser's file input.
    pub fn accept(self) -> &'static str {
        match self {
            Self::Abc => ".abc,text/vnd.abc,text/plain",
            Self::DanceJson => ".json,application/json",
        }
    }

    pub fn multiple(self) -> bool {
        matches!(self, Self::Abc)
    }
}

/// Which kind of text an export writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Abc,
    DanceJson,
}

// See [`ImportKind`]: each platform module uses a different subset.
#[allow(dead_code)]
impl ExportKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Abc => "abc",
            Self::DanceJson => "json",
        }
    }

    pub fn filter_name(self) -> &'static str {
        match self {
            Self::Abc => "ABC tune",
            Self::DanceJson => "Dance JSON",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Abc => &["abc", "txt"],
            Self::DanceJson => &["json"],
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Self::Abc => "text/vnd.abc",
            Self::DanceJson => "application/json",
        }
    }
}

/// One file read by an import dialog, whichever platform supplied it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedFile {
    pub name: String,
    pub text: String,
}

/// What an export did: the user cancelled, or the text reached a destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportOutcome {
    /// Only the desktop dialog can be dismissed; Android and the browser
    /// always write or download.
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    Cancelled,
    Saved(String),
}

/// A file that was exported on Android and can be handed to the share sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShareRequest {
    pub name: String,
    pub text: String,
    pub kind: ExportKind,
}

/// The text and suggested file name for a tune's ABC export.
///
/// The text is returned unchanged: what the tune preview shows is exactly what
/// the user gets in the file.
pub fn tune_export(tune: &Tune) -> (String, String) {
    (
        format!("{}.abc", safe_filename(&tune.title)),
        tune.abc.clone(),
    )
}

/// Serialize a dance into the versioned exchange document.
pub fn dance_json(dance: &Dance) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct Document<'a> {
        format: &'static str,
        version: u32,
        #[serde(flatten)]
        dance: &'a Dance,
    }

    serde_json::to_string_pretty(&Document {
        format: DANCE_FORMAT,
        version: DANCE_VERSION,
        dance,
    })
    .map_err(|error| format!("Could not write the dance file: {error}"))
}

/// The input shape accepted by the dance importer.
///
/// It covers the exported document (with `format` and `version`) and a plain
/// dance object, including the field names ContraDB uses in its own JSON
/// (`id`, `choreographer_name`, `hook`). Figures may be objects or plain
/// strings, because a hand-written card often is just a list of moves.
#[derive(Debug, serde::Deserialize)]
struct DanceFile {
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    version: Option<u32>,
    #[serde(default)]
    source: Option<SourceRef>,
    /// ContraDB dance number. Used to build a canonical source reference when
    /// the file does not carry one.
    #[serde(default)]
    id: Option<serde_json::Value>,
    #[serde(default)]
    title: String,
    #[serde(default, alias = "choreographer_name", alias = "author")]
    choreographer: String,
    #[serde(default)]
    formation: String,
    #[serde(default)]
    preamble: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    hook: String,
    #[serde(default)]
    figures: Vec<FigureFile>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum FigureFile {
    Text(String),
    Row {
        #[serde(default)]
        phrase: Option<String>,
        #[serde(default)]
        beats: Option<f64>,
        #[serde(default, alias = "figure", alias = "figure_text")]
        text: String,
    },
}

/// Whether `format` is a marker this build writes or still reads.
fn is_dance_format(format: &str) -> bool {
    std::iter::once(DANCE_FORMAT)
        .chain(LEGACY_DANCE_FORMATS.iter().copied())
        .any(|known| format.eq_ignore_ascii_case(known))
}

/// Parse a dance exchange document or a plain/ContraDB-shaped dance object.
///
/// The dance is validated before it is returned, so a caller can insert it into
/// the library without a second check.
pub fn dance_from_json(text: &str) -> Result<Dance, String> {
    let file: DanceFile = serde_json::from_str(text)
        .map_err(|error| format!("Not a readable dance file: {error}"))?;

    if let Some(format) = &file.format
        && !is_dance_format(format)
    {
        return Err(format!(
            "This is not a Caller Karaoke dance file (format \"{format}\")."
        ));
    }
    if let Some(version) = file.version
        && version > DANCE_VERSION
    {
        return Err(format!(
            "This dance file uses version {version}; this build reads version {DANCE_VERSION}."
        ));
    }

    let mut figures: Vec<Figure> = Vec::new();
    for figure in file.figures {
        match figure {
            FigureFile::Text(text) => {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    figures.push(Figure::new(None, 8.0, text));
                }
            }
            FigureFile::Row {
                phrase,
                beats,
                text,
            } => {
                let text = text.trim().to_string();
                if text.is_empty() {
                    continue;
                }
                let beats = match beats {
                    Some(beats) if !beats.is_finite() || beats <= 0.0 => {
                        return Err(format!(
                            "Figure {} has an invalid beat count ({beats}).",
                            figures.len() + 1
                        ));
                    }
                    Some(beats) => beats,
                    None => 8.0,
                };
                let phrase = phrase
                    .map(|phrase| phrase.trim().to_string())
                    .filter(|phrase| !phrase.is_empty());
                figures.push(Figure::new(phrase, beats, text));
            }
        }
    }

    let notes = if file.notes.trim().is_empty() {
        file.hook
    } else {
        file.notes
    };

    let source = file.source.unwrap_or_else(|| {
        let id = file
            .id
            .as_ref()
            .and_then(|id| match id {
                serde_json::Value::Number(number) => Some(number.to_string()),
                serde_json::Value::String(value) if !value.trim().is_empty() => {
                    Some(value.trim().to_string())
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                content_id(&dance_fingerprint(
                    &file.title,
                    &file.choreographer,
                    &figures,
                ))
            });
        if file.id.is_some() {
            let url = format!("https://contradb.com/dances/{id}");
            SourceRef::new("contradb", id, url)
        } else {
            SourceRef::new(LOCAL_PROVIDER, id, "")
        }
    });

    let dance = Dance {
        source: normalize_source(source, &file.title, &file.choreographer, &figures),
        title: file.title,
        choreographer: file.choreographer,
        formation: file.formation,
        preamble: file.preamble,
        notes,
        figures,
    };
    dance.validate().map_err(|error| error.to_string())?;
    Ok(dance)
}

/// A source with an empty id would collide with every other imported card, so
/// derive one from the content. A missing provider becomes the local one.
fn normalize_source(
    source: SourceRef,
    title: &str,
    choreographer: &str,
    figures: &[Figure],
) -> SourceRef {
    let provider = if source.provider.trim().is_empty() {
        LOCAL_PROVIDER.to_string()
    } else {
        source.provider.trim().to_string()
    };
    let id = if source.id.trim().is_empty() {
        content_id(&dance_fingerprint(title, choreographer, figures))
    } else {
        source.id.trim().to_string()
    };
    SourceRef::new(provider, id, source.url.trim())
}

fn dance_fingerprint(title: &str, choreographer: &str, figures: &[Figure]) -> String {
    let mut text = format!("{title}\n{choreographer}");
    for figure in figures {
        text.push('\n');
        text.push_str(figure.text.trim());
        text.push('\u{1f}');
        text.push_str(&figure.beats.to_string());
    }
    text
}

/// What an imported ABC file produced.
#[derive(Clone, Debug, PartialEq)]
pub struct AbcImport {
    pub tune: Tune,
    /// More `X:` tunes in the same file. Only the first one is imported.
    pub additional_tunes: usize,
}

/// Build a saved-library tune from an ABC file.
///
/// A file can hold several tunes; the app parser reads the first, so the
/// importer reports how many were left behind instead of silently dropping
/// them. The ABC text is stored as received (apart from a leading byte-order
/// mark), including its line endings.
pub fn tune_from_abc(file_name: &str, text: &str) -> Result<AbcImport, String> {
    let abc = text.strip_prefix('\u{feff}').unwrap_or(text);
    if abc.trim().is_empty() {
        return Err("The file is empty.".to_string());
    }
    // The ABC grammar parses one tune; a second `X:` is either read as music
    // or rejected, depending on the blank lines. Cut the file at the next tune
    // so a book of tunes still imports its first one cleanly.
    let (abc, additional_tunes) = first_tune(abc);
    let score = caller_music::parse_abc(abc).map_err(|error| error.to_string())?;
    let fallback_title = file_stem(file_name);
    let title = header_value(abc, 'T')
        .map(|title| title.trim().to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or(fallback_title);
    let key = key_label(&score.key);
    let meter = format!("{}/{}", score.meter.numerator, score.meter.denominator);
    let rhythm = header_value(abc, 'R').unwrap_or_default();
    let composer = header_value(abc, 'C').unwrap_or_default();
    let id = content_id(abc);
    Ok(AbcImport {
        tune: Tune {
            source: SourceRef::new(LOCAL_PROVIDER, id, ""),
            title,
            composer,
            key,
            meter,
            rhythm,
            abc: abc.to_string(),
            original_abc: Some(abc.to_string()),
            credits: "Imported from a local ABC file.".to_string(),
        },
        additional_tunes,
    })
}

/// The display spelling of a parsed key, including its mode.
fn key_label(key: &caller_music::Key) -> String {
    let suffix = match key.mode {
        caller_music::Mode::Major => "",
        caller_music::Mode::Minor => "m",
        caller_music::Mode::Dorian => " dor",
        caller_music::Mode::Mixolydian => " mix",
        caller_music::Mode::Phrygian => " phr",
        caller_music::Mode::Lydian => " lyd",
        caller_music::Mode::Locrian => " loc",
    };
    format!("{}{suffix}", key.tonic_name)
}

/// Split the first tune out of a file that may hold several.
///
/// A tune starts with an `X:` line. The first tune's text keeps everything
/// before the second `X:` line; trailing blank lines are dropped because the
/// parser treats a blank line as the end of the tune.
fn first_tune(abc: &str) -> (&str, usize) {
    let mut starts = Vec::new();
    let mut offset = 0;
    for line in abc.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("X:")
            && rest.trim().parse::<u32>().is_ok()
        {
            starts.push(offset);
        }
        offset += line.len();
    }
    if starts.len() < 2 {
        return (abc, 0);
    }
    let first = abc[..starts[1]].trim_end();
    (first, starts.len() - 1)
}

/// First value of a one-letter ABC header field, outside comments.
fn header_value(abc: &str, field: char) -> Option<String> {
    abc.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix(field)?.strip_prefix(':')?;
        let value = rest.split('%').next().unwrap_or(rest).trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// The file name without its last extension, cleaned up for a title.
fn file_stem(name: &str) -> String {
    let stem = name
        .rsplit_once(['/', '\\'])
        .map(|(_, name)| name)
        .unwrap_or(name);
    let stem = stem.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(stem);
    let stem = stem.trim();
    if stem.is_empty() {
        "Imported tune".to_string()
    } else {
        stem.to_string()
    }
}

/// A stable, provider-independent id for local content.
///
/// The first 16 hex digits of the SHA-256 are enough to keep two imported files
/// apart; the id never leaves the local library.
fn content_id(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut id = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}

/// A file name that is safe on Windows, Linux, Android, and the browser.
///
/// Control characters, path separators, and the characters Windows reserves
/// become `_`; leading and trailing dots and spaces are removed so the name
/// cannot turn into a hidden file or lose its extension; Windows device names
/// (`CON`, `NUL`, `COM1`, ...) get a trailing underscore.
pub fn safe_filename(title: &str) -> String {
    // Trim separators and hidden-file dots before replacing characters, so
    // " .hidden. " becomes "hidden" rather than "_hidden_".
    let trimmed = title.trim().trim_matches('.');
    let mut name: String = trimmed
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | ' ' | '(' | ')') {
                character
            } else {
                '_'
            }
        })
        .collect();
    name = name.trim().to_string();
    if name.is_empty() {
        name = "untitled".to_string();
    }
    if is_reserved_device_name(&name) {
        name.push('_');
    }
    if name.chars().count() > MAX_FILE_STEM {
        name = name.chars().take(MAX_FILE_STEM).collect::<String>();
        name = name.trim_end().to_string();
    }
    name
}

fn is_reserved_device_name(name: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or(name);
    RESERVED
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// Files waiting for the app to import, whichever platform's dialog produced
/// them.
static IMPORT_QUEUE: std::sync::Mutex<Vec<(ImportKind, ImportedFile)>> =
    std::sync::Mutex::new(Vec::new());

fn push_import(kind: ImportKind, file: ImportedFile) {
    let mut queue = match IMPORT_QUEUE.lock() {
        Ok(queue) => queue,
        Err(poisoned) => poisoned.into_inner(),
    };
    queue.push((kind, file));
}

/// Take every file an import dialog has produced since the last call.
pub fn take_imports() -> Vec<(ImportKind, ImportedFile)> {
    let mut queue = match IMPORT_QUEUE.lock() {
        Ok(queue) => queue,
        Err(poisoned) => poisoned.into_inner(),
    };
    std::mem::take(&mut queue)
}

/// Open the platform's file picker for `kind`.
///
/// Native dialogs return their files through [`take_imports`] immediately; the
/// browser and Android open an asynchronous picker and queue the files when the
/// user chooses them. An error is only returned when the picker itself cannot
/// open; a cancelled dialog is not an error.
pub fn choose_files(kind: ImportKind) -> Result<(), String> {
    let result = platform::choose_files(kind);
    // A browser or Android picker queues its files later; ask for a frame so
    // the app checks the queue even if the page is otherwise idle.
    crate::media_keys::request_repaint();
    result
}

/// Export `text` with the platform's save mechanism.
pub fn export_text(
    kind: ExportKind,
    default_name: &str,
    text: &str,
    export_dir: Option<&Path>,
) -> Result<ExportOutcome, String> {
    platform::export_text(kind, default_name, text, export_dir)
}

/// Hand an exported file to Android's share sheet. Other platforms do not offer
/// this prompt, so the function is never called there.
pub fn share_text(share: &ShareRequest) -> Result<(), String> {
    platform::share_text(share)
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod platform {
    use std::io::Read;

    use super::*;

    pub(super) fn choose_files(kind: ImportKind) -> Result<(), String> {
        let dialog = rfd::FileDialog::new().add_filter(kind.filter_name(), kind.extensions());
        let paths = if kind.multiple() {
            dialog.pick_files().unwrap_or_default()
        } else {
            dialog.pick_file().into_iter().collect()
        };
        for path in paths {
            let file = read_file(&path)?;
            push_import(kind, file);
        }
        Ok(())
    }

    fn read_file(path: &Path) -> Result<ImportedFile, String> {
        let metadata = std::fs::metadata(path)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        if metadata.len() > MAX_IMPORT_BYTES {
            return Err(format!(
                "{} is too large to import (limit {} MB).",
                path.display(),
                MAX_IMPORT_BYTES / (1024 * 1024)
            ));
        }
        let mut text = String::new();
        std::fs::File::open(path)
            .and_then(|mut file| file.read_to_string(&mut text))
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("import")
            .to_string();
        Ok(ImportedFile {
            name,
            text: text.strip_prefix('\u{feff}').unwrap_or(&text).to_string(),
        })
    }

    pub(super) fn export_text(
        kind: ExportKind,
        default_name: &str,
        text: &str,
        _export_dir: Option<&Path>,
    ) -> Result<ExportOutcome, String> {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(default_name)
            .add_filter(kind.filter_name(), kind.extensions())
            .save_file()
        else {
            return Ok(ExportOutcome::Cancelled);
        };
        std::fs::write(&path, text)
            .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
        Ok(ExportOutcome::Saved(path.display().to_string()))
    }

    pub(super) fn share_text(_share: &ShareRequest) -> Result<(), String> {
        Err("Sharing is available on Android.".to_string())
    }
}

#[cfg(target_os = "android")]
mod platform {
    use super::*;

    pub(super) fn choose_files(kind: ImportKind) -> Result<(), String> {
        let mime = kind.mime();
        let multiple = kind.multiple();
        let opened = crate::media_keys::with_activity(|env, class| {
            let mime = env.new_string(mime)?;
            env.call_static_method(
                class,
                "openDocument",
                "(ILjava/lang/String;Z)V",
                &[
                    jni::objects::JValue::Int(kind.code()),
                    jni::objects::JValue::Object(&mime),
                    jni::objects::JValue::Bool(u8::from(multiple)),
                ],
            )?;
            Ok(())
        });
        if opened {
            Ok(())
        } else {
            Err("The Android file picker is not ready yet; try again in a moment.".to_string())
        }
    }

    pub(super) fn export_text(
        kind: ExportKind,
        default_name: &str,
        text: &str,
        export_dir: Option<&Path>,
    ) -> Result<ExportOutcome, String> {
        // `export_dir` is the app's private config directory. It is always
        // writable in the sandbox, and the share prompt below hands the file to
        // the system so the user can send it elsewhere.
        let dir = export_dir
            .map(|dir| dir.join("exports"))
            .unwrap_or_else(std::env::temp_dir);
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("Could not create {}: {error}", dir.display()))?;
        // The caller already supplies a name with the extension; adding it
        // again would write `Title.abc.abc`.
        let suffix = format!(".{}", kind.extension());
        let file_name = if default_name.to_ascii_lowercase().ends_with(&suffix) {
            default_name.to_string()
        } else {
            format!("{default_name}{suffix}")
        };
        let path = dir.join(file_name);
        std::fs::write(&path, text)
            .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
        Ok(ExportOutcome::Saved(path.display().to_string()))
    }

    pub(super) fn share_text(share: &ShareRequest) -> Result<(), String> {
        let shared = crate::media_keys::with_activity(|env, class| {
            let name = env.new_string(&share.name)?;
            let mime = env.new_string(share.kind.mime())?;
            let text = env.new_string(&share.text)?;
            env.call_static_method(
                class,
                "shareText",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                &[
                    jni::objects::JValue::Object(&name),
                    jni::objects::JValue::Object(&mime),
                    jni::objects::JValue::Object(&text),
                ],
            )?;
            Ok(())
        });
        if shared {
            Ok(())
        } else {
            Err("Could not open the Android share sheet.".to_string())
        }
    }
}

/// Called by `MainActivity` after the system document picker returns a file.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_callerkaraoke_app_MainActivity_nativeImportedFile(
    mut env: jni::JNIEnv<'_>,
    _class: jni::objects::JClass<'_>,
    kind: i32,
    name: jni::objects::JString<'_>,
    text: jni::objects::JString<'_>,
) {
    let Some(kind) = ImportKind::from_code(kind) else {
        return;
    };
    let (Ok(name), Ok(text)) = (env.get_string(&name), env.get_string(&text)) else {
        let _ = env.exception_clear();
        return;
    };
    push_import(
        kind,
        ImportedFile {
            name: name.into(),
            text: text.into(),
        },
    );
    crate::media_keys::request_repaint();
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen_futures::{JsFuture, spawn_local};

    use super::*;

    fn js_error(error: JsValue) -> String {
        error
            .as_string()
            .unwrap_or_else(|| "the browser refused the file operation".to_string())
    }

    pub(super) fn choose_files(kind: ImportKind) -> Result<(), String> {
        let window = web_sys::window().ok_or("There is no browser window.")?;
        let document = window.document().ok_or("There is no document.")?;
        let input = document
            .create_element("input")
            .map_err(js_error)?
            .dyn_into::<web_sys::HtmlInputElement>()
            .map_err(|_| "Could not create a file input.".to_string())?;
        input.set_type("file");
        input.set_accept(kind.accept());
        input.set_multiple(kind.multiple());

        let picker = input.clone();
        let on_change = Closure::<dyn FnMut()>::new(move || {
            let Some(files) = picker.files() else {
                return;
            };
            let mut chosen = Vec::new();
            for index in 0..files.length() {
                if let Some(file) = files.get(index) {
                    chosen.push(file);
                }
            }
            let cleanup = picker.clone();
            spawn_local(async move {
                for file in chosen {
                    let value = JsFuture::from(file.text()).await;
                    let Ok(value) = value else {
                        continue;
                    };
                    if let Some(text) = value.as_string() {
                        if text.len() as u64 > MAX_IMPORT_BYTES {
                            continue;
                        }
                        push_import(
                            kind,
                            ImportedFile {
                                name: file.name(),
                                text,
                            },
                        );
                    }
                }
                cleanup.remove();
                crate::media_keys::request_repaint();
            });
        });
        input
            .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())
            .map_err(js_error)?;
        // The closure lives for the page: a file input is cheap and there is no
        // place to release it while the change event can still fire.
        on_change.forget();
        document
            .body()
            .ok_or("There is no page body.")?
            .append_child(&input)
            .map_err(js_error)?;
        input.click();
        Ok(())
    }

    pub(super) fn export_text(
        kind: ExportKind,
        default_name: &str,
        text: &str,
        _export_dir: Option<&Path>,
    ) -> Result<ExportOutcome, String> {
        let window = web_sys::window().ok_or("There is no browser window.")?;
        let document = window.document().ok_or("There is no document.")?;
        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(text));
        let parts: JsValue = parts.into();
        let options = web_sys::BlobPropertyBag::new();
        options.set_type(kind.mime());
        let blob =
            web_sys::Blob::new_with_str_sequence_and_options(&parts, &options).map_err(js_error)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_error)?;
        let anchor = document
            .create_element("a")
            .map_err(js_error)?
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .map_err(|_| "Could not create a download link.".to_string())?;
        anchor.set_href(&url);
        anchor.set_download(default_name);
        document
            .body()
            .ok_or("There is no page body.")?
            .append_child(&anchor)
            .map_err(js_error)?;
        anchor.click();
        anchor.remove();
        let _ = web_sys::Url::revoke_object_url(&url);
        Ok(ExportOutcome::Saved(default_name.to_string()))
    }

    pub(super) fn share_text(_share: &ShareRequest) -> Result<(), String> {
        Err("Sharing is available on Android.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use caller_core::Dance;

    fn sample_dance() -> Dance {
        crate::defaults::default_dance()
    }

    #[test]
    fn abc_export_is_the_tune_text_verbatim() {
        let tune = crate::defaults::default_tune();
        let (name, text) = tune_export(&tune);
        assert_eq!(text, tune.abc);
        assert!(name.ends_with(".abc"));
        assert!(!name.contains('/'));
        assert!(!name.contains('\\'));
    }

    #[test]
    fn dance_json_round_trips_through_the_import_parser() {
        let dance = sample_dance();
        let json = dance_json(&dance).expect("serialize");
        assert!(json.contains(DANCE_FORMAT));
        assert!(json.contains(&format!("\"version\": {DANCE_VERSION}")));
        assert!(json.contains("\"figures\""));
        let restored = dance_from_json(&json).expect("parse the exported document");
        assert_eq!(restored, dance);
    }

    #[test]
    fn dances_saved_under_the_old_name_still_import() {
        let dance = sample_dance();
        let mut document: serde_json::Value =
            serde_json::from_str(&dance_json(&dance).expect("serialize")).expect("valid json");
        document["format"] = serde_json::Value::String("contra-karaoke-dance".to_owned());
        let restored = dance_from_json(&document.to_string()).expect("legacy document imports");
        assert_eq!(restored, dance);
    }

    #[test]
    fn plain_and_contradb_shaped_dances_import() {
        let plain = r#"{
            "title": "Hand written card",
            "choreographer": "A caller",
            "formation": "improper",
            "figures": [
                {"phrase": "A1", "beats": 8, "text": "neighbors balance and swing"},
                "ladles chain"
            ]
        }"#;
        let dance = dance_from_json(plain).expect("plain dance imports");
        assert_eq!(dance.title, "Hand written card");
        assert_eq!(dance.figures.len(), 2);
        assert_eq!(dance.figures[1].phrase, None);
        assert_eq!(dance.figures[1].beats, 8.0);
        assert_eq!(dance.source.provider, LOCAL_PROVIDER);
        assert!(!dance.source.id.is_empty());

        let contradb = r#"{
            "id": 2409,
            "title": "Another Last Dance",
            "choreographer_name": "Erik Hoffman",
            "formation": "improper",
            "hook": "a hook",
            "figures": [{"beats": 8, "text": "neighbors do si do"}]
        }"#;
        let dance = dance_from_json(contradb).expect("ContraDB shape imports");
        assert_eq!(dance.source.provider, "contradb");
        assert_eq!(dance.source.id, "2409");
        assert_eq!(dance.notes, "a hook");
    }

    #[test]
    fn malformed_dance_files_are_rejected() {
        assert!(dance_from_json("not json").is_err());
        assert!(dance_from_json("{}").is_err(), "no title and no figures");
        assert!(
            dance_from_json(r#"{"title":"Empty","figures":[]}"#).is_err(),
            "a card needs figures"
        );
        assert!(
            dance_from_json(
                r#"{"format":"something-else","title":"x","figures":[{"text":"swing","beats":8}]}"#
            )
            .is_err(),
            "an unrelated JSON document must not import"
        );
        assert!(
            dance_from_json(
                r#"{"format":"caller-karaoke-dance","version":99,"title":"x","figures":[{"text":"swing","beats":8}]}"#
            )
            .is_err(),
            "a newer document is not silently accepted"
        );
        assert!(
            dance_from_json(r#"{"title":"Bad beats","figures":[{"text":"swing","beats":-1}]}"#)
                .is_err()
        );
    }

    #[test]
    fn minimal_abc_imports_with_a_local_source() {
        let abc = "X:1\nT:Test Reel\nC:Composer\nR:reel\nM:4/4\nL:1/8\nK:D\nDEFG ABcd|\n";
        let import = tune_from_abc("test.abc", abc).expect("import");
        assert_eq!(import.tune.title, "Test Reel");
        assert_eq!(import.tune.composer, "Composer");
        assert_eq!(import.tune.rhythm, "reel");
        assert_eq!(import.tune.key, "D");
        assert_eq!(import.tune.meter, "4/4");
        assert_eq!(import.tune.abc, abc);
        assert_eq!(import.tune.original_abc.as_deref(), Some(abc));
        assert_eq!(import.tune.source.provider, LOCAL_PROVIDER);
        assert_eq!(import.additional_tunes, 0);
    }

    #[test]
    fn abc_import_uses_the_file_stem_and_counts_extra_tunes() {
        let abc =
            "X:1\nT:First\nM:4/4\nL:1/8\nK:D\nDEFG|\n\nX:2\nT:Second\nM:4/4\nL:1/8\nK:G\nGABc|\n";
        let import = tune_from_abc("my_tune.abc", abc).expect("import the first tune");
        assert_eq!(import.tune.title, "First");
        assert_eq!(import.additional_tunes, 1);
        assert!(import.tune.abc.starts_with("X:1"));
        assert!(!import.tune.abc.contains("X:2"));

        let untitled = "X:1\nT:\nM:4/4\nL:1/8\nK:D\nDEFG|\n";
        let import = tune_from_abc("named_file.abc", untitled).expect("import");
        assert_eq!(import.tune.title, "named_file");
    }

    #[test]
    fn malformed_abc_is_rejected() {
        assert!(tune_from_abc("empty.abc", "").is_err());
        assert!(tune_from_abc("broken.abc", "this is not abc").is_err());
        assert!(tune_from_abc("no_notes.abc", "X:1\nT:Empty\nM:4/4\nL:1/8\nK:D\n").is_err());
    }

    #[test]
    fn the_same_abc_text_gets_the_same_id() {
        let abc = "X:1\nT:Same\nL:1/8\nK:D\nDEFG|\n";
        let first = tune_from_abc("a.abc", abc).expect("import");
        let second = tune_from_abc("b.abc", abc).expect("import");
        assert_eq!(first.tune.source.id, second.tune.source.id);

        let different = tune_from_abc("c.abc", "X:1\nT:Same\nL:1/8\nK:D\nGABc|\n")
            .expect("import a different tune");
        assert_ne!(first.tune.source.id, different.tune.source.id);
    }

    #[test]
    fn file_names_drop_path_characters_and_controls() {
        assert_eq!(safe_filename("Sandy Boys"), "Sandy Boys");
        assert_eq!(safe_filename("a/b\\c:d*e?f"), "a_b_c_d_e_f");
        assert_eq!(safe_filename("  .hidden.  "), "hidden");
        assert_eq!(safe_filename(""), "untitled");
        assert_eq!(safe_filename("   "), "untitled");
        assert_eq!(safe_filename("nul"), "nul_");
        assert_eq!(safe_filename("CON"), "CON_");
        assert_eq!(safe_filename("COM1"), "COM1_");
        assert_eq!(safe_filename("tab\there"), "tab_here");
        assert!(!safe_filename("Bellevue\u{7}").contains('\u{7}'));
    }

    #[test]
    fn file_names_are_bounded_and_keep_unicode_letters() {
        let long = "a".repeat(200);
        assert_eq!(safe_filename(&long).chars().count(), MAX_FILE_STEM);
        assert_eq!(safe_filename("Café Waltz"), "Café Waltz");
    }
}
