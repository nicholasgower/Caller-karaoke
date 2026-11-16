//! Sheet-music preview for ABC tunes.
//!
//! Playback keeps using `contra_music`'s ABC parser. This module is display
//! only: acorde engraves the tune, and resvg rasterizes the SVG into an egui
//! texture. Keeping the engraver separate means a notation feature added for
//! the preview can never change call timing.

use contra_music::{Key, Mode};
use eframe::egui;

/// Logical width the engraver lays the score out for, in points.
const SHEET_WIDTH: f32 = 900.0;
/// Distance between staff lines. Large enough to stay legible when the
/// preview is scaled down to fit a phone-width panel.
const STAFF_SIZE: f32 = 18.0;
const MEASURES_PER_SYSTEM: usize = 4;
/// Cap the rasterized texture so a very tall tune cannot allocate unbounded
/// memory on a phone. A 1200 px wide sheet is still over 2x the width of the
/// picker column in the desktop layout.
const MAX_RASTER_WIDTH: f32 = 1200.0;
const MAX_RASTER_HEIGHT: f32 = 2400.0;

/// Acorde draws chord symbols at the staff size (18 pt here), which loses
/// almost all detail once the sheet is scaled into a phone-width preview.
/// Doubling the letters restores their weight without touching the staff or
/// the notes; the engraver's symbol lane already leaves room for one line of
/// text above the staff.
const CHORD_SYMBOL_SCALE: f32 = 2.0;
/// Width of one italic serif digit as a fraction of the font size. A row-start
/// measure number keeps its left edge on the staff margin, so a second digit
/// runs right into the treble clef; shifting by one extra digit's advance
/// keeps the number clear while a single digit stays where the engraver put it.
/// The fraction is slightly wider than a plain 0.5 em because italic numerals
/// overhang their advance.
const MEASURE_NUMBER_DIGIT_ADVANCE: f32 = 0.55;

/// A rasterized sheet-music texture ready to draw in the tune picker.
pub struct SheetPreview {
    texture: egui::TextureHandle,
    /// Logical size of the engraved sheet, in points.
    size: egui::Vec2,
}

impl SheetPreview {
    /// Parse `abc`, engrave it, and rasterize it for the current display.
    pub fn render(ctx: &egui::Context, abc: &str) -> Result<Self, String> {
        let svg = abc_to_svg(abc)?;
        let pixels_per_point = ctx.pixels_per_point().clamp(1.0, 2.0);
        let (image, size) = rasterize(&svg, pixels_per_point)?;
        let texture = ctx.load_texture("tune-sheet", image, egui::TextureOptions::LINEAR);
        Ok(Self { texture, size })
    }

    /// Bytes the rasterized texture occupies in the GPU/CPU texture store.
    pub fn texture_bytes(&self) -> u64 {
        let [width, height] = self.texture.size();
        (width as u64) * (height as u64) * 4
    }

    /// Draw the sheet at the panel width, preserving its aspect ratio.
    pub fn ui(&self, ui: &mut egui::Ui) {
        let width = ui.available_width().min(self.size.x).max(1.0);
        let height = width * self.size.y / self.size.x;
        ui.add(
            egui::Image::new(egui::load::SizedTexture::new(self.texture.id(), self.size))
                .fit_to_exact_size(egui::vec2(width, height))
                .alt_text("Sheet music preview")
                .corner_radius(4),
        );
    }
}

/// Engrave an ABC tune to SVG. Public so tests can check the notation without
/// creating a GPU texture.
pub fn abc_to_svg(abc: &str) -> Result<String, String> {
    engrave_abc(abc).map(|svg| restyle_annotations(&svg))
}

/// The engraver's unmodified output. Kept separate so tests can compare the
/// preview's typography against what acorde actually drew.
fn engrave_abc(abc: &str) -> Result<String, String> {
    let abc = normalize_modal_keys(abc);
    let score = acorde::io::parse_abc(&abc).map_err(|error| format!("ABC parser: {error}"))?;
    let options = acorde_render_svg::SvgRenderOptions {
        width: SHEET_WIDTH,
        staff_size: STAFF_SIZE,
        measures_per_system: MEASURES_PER_SYSTEM,
        interactive: false,
    };
    acorde_render_svg::render_svg(&score, &options).map_err(|error| format!("engraver: {error}"))
}

/// Preview-only typography fixes applied to the engraver's SVG.
///
/// This is a deliberate text-level pass rather than a forked engraver: acorde
/// emits every annotation as a plain `<text>` element with a stable class, so
/// rewriting two attributes keeps the display change isolated from playback and
/// from the ABC parser. Elements without a matching class are copied verbatim,
/// byte for byte.
fn restyle_annotations(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len() + 256);
    let mut cursor = 0;
    while let Some(offset) = svg[cursor..].find("<text") {
        let start = cursor + offset;
        let Some(tag_end) = svg[start..].find('>').map(|end| start + end + 1) else {
            break;
        };
        let Some(content_end) = svg[tag_end..].find("</text>").map(|end| tag_end + end) else {
            break;
        };
        let element_end = content_end + "</text>".len();
        out.push_str(&svg[cursor..start]);
        match restyle_text_element(&svg[start..tag_end], &svg[tag_end..content_end]) {
            Some(element) => out.push_str(&element),
            None => out.push_str(&svg[start..element_end]),
        }
        cursor = element_end;
    }
    out.push_str(&svg[cursor..]);
    out
}

/// Rewrite one `<text>` element, or return `None` when it is not an annotation
/// the preview adjusts. `tag` is the start tag without its `>`, `text` is the
/// element's character data.
fn restyle_text_element(tag: &str, text: &str) -> Option<String> {
    let classes: Vec<&str> = attribute(tag, "class")?.split_whitespace().collect();
    if classes.contains(&"acorde-chord-symbol") {
        let size: f32 = attribute(tag, "font-size")?.parse().ok()?;
        let tag = with_attribute(
            tag,
            "font-size",
            &format!("{:.2}", size * CHORD_SYMBOL_SCALE),
        )?;
        return Some(format!("{tag}>{text}</text>"));
    }
    if classes.contains(&"acorde-measure-number") {
        // Acorde writes a number only for the first measure of a system
        // (`col == 0 && row_idx > 0`), so every measure number here starts a
        // row; mid-row numbers do not exist in the output.
        let digits = text.trim().chars().count();
        if digits < 2 {
            return None;
        }
        let size: f32 = attribute(tag, "font-size")?.parse().ok()?;
        let x: f32 = attribute(tag, "x")?.parse().ok()?;
        let shift = (digits - 1) as f32 * size * MEASURE_NUMBER_DIGIT_ADVANCE;
        let x = (x - shift).max(0.0);
        let tag = with_attribute(tag, "x", &format!("{x:.2}"))?;
        return Some(format!("{tag}>{text}</text>"));
    }
    None
}

/// Value of the `name="..."` attribute in an SVG start tag.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!(" {name}=\"");
    let rest = &tag[tag.find(&needle)? + needle.len()..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Replace an existing attribute value without touching the rest of the tag.
fn with_attribute(tag: &str, name: &str, value: &str) -> Option<String> {
    let needle = format!(" {name}=\"");
    let value_start = tag.find(&needle)? + needle.len();
    let value_end = value_start + tag[value_start..].find('"')?;
    let mut out = String::with_capacity(tag.len() + value.len());
    out.push_str(&tag[..value_start]);
    out.push_str(value);
    out.push_str(&tag[value_end..]);
    Some(out)
}

/// Acorde understands major and minor key signatures, but not ABC's modal
/// keys (`A mix`, `Edor`, ...). Playback does understand them, so rewrite each
/// modal key to the standard major key with the same accidentals. The score
/// then draws the same sharps and flats that the synthesiser plays, and the
/// original tonic can stay in the ABC view.
fn normalize_modal_keys(abc: &str) -> String {
    abc.lines()
        .map(normalize_key_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_key_line(line: &str) -> String {
    if line.starts_with("w:") || line.starts_with("W:") || line.starts_with("%%") {
        return line.to_string();
    }
    let (body, comment) = match line.split_once('%') {
        Some((body, comment)) => (body, Some(comment)),
        None => (line, None),
    };

    let mut out = String::with_capacity(line.len());
    let mut rest = body;
    if let Some(value) = body.strip_prefix("K:") {
        out.push_str("K:");
        out.push_str(&normalize_key_value(value));
        rest = "";
    }
    while let Some(start) = rest.find("[K:") {
        let Some(end) = rest[start + 3..].find(']') else {
            break;
        };
        out.push_str(&rest[..start + 3]);
        out.push_str(&normalize_key_value(&rest[start + 3..start + 3 + end]));
        out.push(']');
        rest = &rest[start + 3 + end + 1..];
    }
    out.push_str(rest);
    if let Some(comment) = comment {
        out.push('%');
        out.push_str(comment);
    }
    out
}

fn normalize_key_value(value: &str) -> String {
    let original = value.trim();
    let Some(key) = Key::parse(original) else {
        return original.to_string();
    };
    if matches!(key.mode, Mode::Major | Mode::Minor) {
        return original.to_string();
    }
    let Some(major) = major_key_with(&key.accidentals) else {
        return original.to_string();
    };
    let mut out = major.to_string();
    for parameter in original
        .split_whitespace()
        .filter(|token| token.contains('='))
    {
        out.push(' ');
        out.push_str(parameter);
    }
    out
}

/// Canonical circle-of-fifths spelling for a major key with the same
/// accidentals as `accidentals` (indexed C, D, E, F, G, A, B).
fn major_key_with(accidentals: &[i32; 7]) -> Option<&'static str> {
    const MAJOR_KEYS: [&str; 15] = [
        "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#",
    ];
    MAJOR_KEYS
        .iter()
        .copied()
        .find(|name| Key::parse(name).is_some_and(|key| key.accidentals == *accidentals))
}

/// Rasterize SVG onto a paper-white background. Returns the image and the
/// sheet's logical size in points.
fn rasterize(svg: &str, pixels_per_point: f32) -> Result<(egui::ColorImage, egui::Vec2), String> {
    let mut options = resvg::usvg::Options::default();
    {
        let fontdb = options.fontdb_mut();
        fontdb.load_font_data(epaint_default_fonts::UBUNTU_LIGHT.to_vec());
        fontdb.set_serif_family("Ubuntu");
        fontdb.set_sans_serif_family("Ubuntu");
    }
    let tree = resvg::usvg::Tree::from_str(svg, &options)
        .map_err(|error| format!("could not read engraver output: {error}"))?;
    let logical = tree.size();
    if !logical.width().is_finite()
        || !logical.height().is_finite()
        || logical.width() <= 0.0
        || logical.height() <= 0.0
    {
        return Err("the engraver produced an empty sheet".to_string());
    }

    let mut scale = pixels_per_point;
    scale = scale.min(MAX_RASTER_WIDTH / logical.width());
    scale = scale.min(MAX_RASTER_HEIGHT / logical.height());
    let raster_width = (logical.width() * scale).ceil().max(1.0) as u32;
    let raster_height = (logical.height() * scale).ceil().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(raster_width, raster_height)
        .ok_or_else(|| "the sheet is too large to rasterize".to_string())?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let image = egui::ColorImage::from_rgba_unmultiplied(
        [raster_width as usize, raster_height as usize],
        pixmap.data(),
    );
    Ok((image, egui::vec2(logical.width(), logical.height())))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_ABC: &str = include_str!("../../../assets/defaults/sandy_boys.abc");

    /// Every `<text>` element whose class list contains `class`, with the byte
    /// offset where it starts in `svg`.
    fn text_elements<'a>(svg: &'a str, class: &str) -> Vec<(usize, &'a str)> {
        svg.match_indices("<text")
            .filter_map(|(start, _)| {
                let end = start + svg[start..].find("</text>")? + "</text>".len();
                let element = &svg[start..end];
                let classes = attribute(element, "class")?;
                classes
                    .split_whitespace()
                    .any(|name| name == class)
                    .then_some((start, element))
            })
            .collect()
    }

    fn text_content(element: &str) -> &str {
        let start = element.find('>').expect("text element has a start tag") + 1;
        let end = element
            .rfind("</text>")
            .expect("text element has an end tag");
        &element[start..end]
    }

    fn attr_f32(element: &str, name: &str) -> f32 {
        attribute(element, name)
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("text element has a numeric {name}"))
    }

    /// Leftmost x of the treble clef's ink in the vertical band a row-start
    /// measure number occupies. Read from the clef path that precedes the
    /// number so the assertion follows the engraver if it moves the clef.
    fn treble_clef_left_edge(svg_before_number: &str, baseline: f32, size: f32) -> f32 {
        let clef = svg_before_number
            .rfind("class=\"acorde-clef acorde-clef-treble\"")
            .expect("each system draws its treble clef before its first measure");
        let path_start = svg_before_number[clef..]
            .find(" d=\"")
            .expect("the clef glyph is a path")
            + clef
            + 4;
        let path_end = path_start
            + svg_before_number[path_start..]
                .find('"')
                .expect("the path data ends with a quote");
        let points: Vec<f32> = svg_before_number[path_start..path_end]
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter_map(|token| token.parse().ok())
            .collect();
        let top = baseline - 0.75 * size;
        let bottom = baseline + 0.15 * size;
        points
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|pair| pair[1] >= top && pair[1] <= bottom)
            .map(|pair| pair[0])
            .fold(f32::INFINITY, f32::min)
    }

    #[test]
    fn chord_symbols_render_at_twice_the_engraver_size() {
        let raw = engrave_abc(DEFAULT_ABC).expect("the bundled tune engraves");
        let styled = abc_to_svg(DEFAULT_ABC).expect("the preview typography applies");
        let raw_chords = text_elements(&raw, "acorde-chord-symbol");
        let styled_chords = text_elements(&styled, "acorde-chord-symbol");

        assert!(raw_chords.len() >= 4, "the bundled tune has chord symbols");
        assert_eq!(raw_chords.len(), styled_chords.len());
        for ((_, raw), (_, styled)) in raw_chords.iter().zip(&styled_chords) {
            let raw_size = attr_f32(raw, "font-size");
            let styled_size = attr_f32(styled, "font-size");
            assert!(
                (styled_size - CHORD_SYMBOL_SCALE * raw_size).abs() < 0.01,
                "chord symbol {raw_size} should render at {styled_size}"
            );
            // A rough cap height keeps the enlarged text inside the view box.
            assert!(
                attr_f32(styled, "y") - 0.75 * styled_size > 0.0,
                "the enlarged chord symbol would be clipped at the top"
            );
            assert!(
                (attr_f32(styled, "y") - attr_f32(raw, "y")).abs() < 0.01,
                "doubling the font must not move the chord baseline"
            );
        }

        // Chords in the same annotation lane must not run into each other.
        let mut lanes: Vec<(f32, f32, f32)> = styled_chords
            .iter()
            .map(|(_, element)| {
                let size = attr_f32(element, "font-size");
                let width = text_content(element).chars().count() as f32 * 0.6 * size;
                (attr_f32(element, "y"), attr_f32(element, "x"), width)
            })
            .collect();
        lanes.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        for pair in lanes.windows(2) {
            if (pair[0].0 - pair[1].0).abs() < 0.01 {
                assert!(
                    pair[0].1 + 0.5 * pair[0].2 < pair[1].1 - 0.5 * pair[1].2,
                    "chord symbols {:?} and {:?} collide",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    #[test]
    fn double_digit_row_start_measure_numbers_shift_clear_of_the_clef() {
        let raw_svg = engrave_abc(DEFAULT_ABC).expect("the bundled tune engraves");
        let styled_svg = abc_to_svg(DEFAULT_ABC).expect("the preview typography applies");
        let raw_numbers = text_elements(&raw_svg, "acorde-measure-number");
        let styled_numbers = text_elements(&styled_svg, "acorde-measure-number");

        assert_eq!(raw_numbers.len(), styled_numbers.len());
        assert!(
            raw_numbers
                .iter()
                .any(|(_, text)| text_content(text).trim().len() == 1)
        );
        assert!(
            raw_numbers
                .iter()
                .any(|(_, text)| text_content(text).trim().len() == 2)
        );

        for ((offset, raw), (_, styled)) in raw_numbers.iter().zip(&styled_numbers) {
            let offset = *offset;
            let (raw, styled) = (*raw, *styled);
            let digits = text_content(raw).trim().chars().count();
            let size = attr_f32(raw, "font-size");
            let clef_left = treble_clef_left_edge(&raw_svg[..offset], attr_f32(raw, "y"), size);
            assert!(clef_left.is_finite(), "the clef has ink beside the number");
            // 0.6 em per numeral is wider than a serif digit advance, so the
            // clearance assertion stays conservative.
            let width = digits as f32 * 0.6 * size;
            let raw_right = attr_f32(raw, "x") + width;

            if digits == 1 {
                assert!(
                    (attr_f32(styled, "x") - attr_f32(raw, "x")).abs() < 0.01,
                    "a single-digit measure number must not move"
                );
                assert!(
                    raw_right <= clef_left,
                    "premise: a single digit clears the clef"
                );
            } else {
                let shift = (digits - 1) as f32 * size * MEASURE_NUMBER_DIGIT_ADVANCE;
                assert!(
                    (attr_f32(raw, "x") - attr_f32(styled, "x") - shift).abs() < 0.01,
                    "a {digits}-digit number shifts by one extra digit's advance"
                );
                assert!(
                    raw_right > clef_left,
                    "premise: the engraver's {digits}-digit number overlaps the clef"
                );
            }
            assert!(
                attr_f32(styled, "x") + width <= clef_left,
                "measure number {} intrudes into the treble clef",
                text_content(styled)
            );
            assert!(attr_f32(styled, "x") >= 0.0, "the number stays on the page");
        }
    }

    #[test]
    fn bundled_tune_engraves_to_svg() {
        let svg = abc_to_svg(DEFAULT_ABC).expect("the bundled tune engraves");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("acorde-chord-symbol"));
    }

    #[test]
    fn modal_tune_engraves_with_the_played_key_signature() {
        let normalized = normalize_modal_keys(DEFAULT_ABC);
        let score = acorde::io::parse_abc(&normalized).expect("the normalized tune parses");
        assert_eq!(score.settings.key_signature.fifths, 2);
    }

    #[test]
    fn empty_abc_is_an_error_not_a_panic() {
        assert!(abc_to_svg("").is_err());
    }

    #[test]
    fn modal_keys_are_rewritten_to_their_key_signature() {
        assert_eq!(normalize_key_line("K:A mix"), "K:D");
        assert_eq!(normalize_key_line("K:AMix"), "K:D");
        assert_eq!(normalize_key_line("K:Ador"), "K:G");
        assert_eq!(normalize_key_line("K:Edor"), "K:D");
        assert_eq!(normalize_key_line("K:Gmaj"), "K:Gmaj");
        assert_eq!(normalize_key_line("K:Em"), "K:Em");
    }

    #[test]
    fn key_rewriting_keeps_clefs_parameters_and_comments() {
        assert_eq!(
            normalize_key_line("K:A mix clef=bass % modal"),
            "K:D clef=bass% modal"
        );
        assert_eq!(normalize_key_line("[K:A mix] A B c d |"), "[K:D] A B c d |");
    }

    #[test]
    fn preview_creates_a_texture() {
        let ctx = egui::Context::default();
        let preview = SheetPreview::render(&ctx, DEFAULT_ABC).expect("the preview renders");
        assert!(preview.size.x > 0.0);
        assert!(preview.size.y > 0.0);
    }

    #[test]
    fn rasterized_sheet_puts_dark_ink_on_white_paper() {
        let svg = abc_to_svg(DEFAULT_ABC).expect("the bundled tune engraves");
        let (image, size) = rasterize(&svg, 1.0).expect("the sheet rasterizes");

        assert!(image.size[0] >= size.x as usize);
        assert!(image.size[1] >= size.y as usize);
        assert_eq!(image.pixels[0], egui::Color32::WHITE);
        let dark_pixels = image
            .pixels
            .iter()
            .filter(|pixel| pixel.r() < 128 && pixel.g() < 128 && pixel.b() < 128)
            .count();
        assert!(dark_pixels > 1_000, "engraved sheet has visible notation");
    }
}
