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
