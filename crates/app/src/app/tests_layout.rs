//! Offscreen-layout tests: every screen at every supported width.
//!
//! Each test renders the real app through `egui_kittest` and reads the
//! accessibility tree, whose nodes carry the logical widget rectangles. A
//! vertical `ScrollArea` may extend below the viewport; horizontal overflow is
//! never allowed, because every scrolling container in the app scrolls
//! vertically only. Where a screen is not fully scrollable, its top-level
//! controls and the header must also stay inside the viewport vertically.
//!
//! The tests do not open a window, an audio device, or a network client, and
//! the headless app disables animations so frames settle immediately.

use super::*;
use crate::app::ui::preferences::{PreferencesCategory, PreferencesPage};
use contra_core::SourceRef;
use contra_sources::{DanceSearchPage, DanceSummary, TuneSearchPage, TuneSummary};
use eframe::egui;
use eframe::egui::accesskit;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

/// Widths the app must remain usable at. 360 is the minimum window width, 390
/// the phone reference, 640 the `COMPACT_WIDTH` breakpoint, and the rest cover
/// a tablet, a laptop, and a large desktop.
const WIDTHS: &[f32] = &[360.0, 390.0, 640.0, 800.0, 1280.0, 1920.0];

/// One tall and one short viewport per width, so the header and the shapes of
/// the forms are checked in both portrait and landscape-ish windows.
fn heights() -> impl Iterator<Item = f32> {
    [844.0_f32, 560.0].into_iter()
}

const SCREENS: [Screen; 7] = [
    Screen::Session,
    Screen::DancePicker,
    Screen::DanceDownload,
    Screen::TunePicker,
    Screen::TuneDownload,
    Screen::Preferences,
    Screen::Aliases,
];

fn demo_app(cc: &mut eframe::CreationContext<'_>) -> ContraApp {
    let mut app = ContraApp::new_headless(cc);
    app.clear_render_pending();
    app.animations_off_for_test();

    let mut dance_library = Library::default();
    let dance = crate::defaults::default_dance();
    dance_library.insert(dance.clone(), None, 0);
    let mut second = dance.clone();
    second.source = SourceRef::new("contradb", "2154", "https://contradb.com/dances/2154");
    second.title = "Belle of the Ball".into();
    dance_library.insert(second, None, 0);
    app.dance_library = dance_library;
    app.dance_library_query = "belle".into();

    let mut tune_library = Library::default();
    let tune = crate::defaults::default_tune();
    tune_library.insert(tune.clone(), None, 0);
    let mut second_tune = tune.clone();
    second_tune.source = SourceRef::new(
        "abcsessiontunes",
        "75412",
        "https://thesession.org/tunes/75412",
    );
    second_tune.title = "Saint Anne's Reel".into();
    tune_library.insert(second_tune, None, 0);
    app.tune_library = tune_library;
    app.tune_library_query = "saint".into();

    app.picker.dance_results = Some(DanceSearchPage {
        total: 23,
        page: 1,
        page_size: 10,
        dances: vec![
            DanceSummary {
                id: "2409".into(),
                title: "Another Last Dance".into(),
                choreographer: "Erik Hoffman".into(),
                formation: "improper".into(),
                hook: String::new(),
                user: "Koren A Wake".into(),
            },
            DanceSummary {
                id: "2380".into(),
                title: "Allison's Favorite".into(),
                choreographer: "Bob Isaacs".into(),
                formation: "improper".into(),
                hook: "A smooth, flowing dance".into(),
                user: "Koren A Wake".into(),
            },
        ],
    });
    app.picker.tune_results = Some(TuneSearchPage {
        total: 8,
        page: 1,
        page_size: 10,
        tunes: vec![TuneSummary {
            id: "75815".into(),
            title: "Sandy Boys".into(),
            composer: String::new(),
            key: "A mix".into(),
            meter: "4/4".into(),
            rhythm: "reel".into(),
        }],
    });
    app.picker.dance_preview = Some(dance.clone());
    app.picker.tune_preview = Some(tune.clone());
    app.error = Some("HTTP 500 Internal Server Error".into());
    app.picker.dance_query.user = "Koren".into();
    app.picker.tune_query.meter = Some("4/4".into());
    app
}

impl ContraApp {
    /// The headless renderer already turns animations off in preferences; keep
    /// the helper explicit so a layout test can never sit mid-transition.
    fn animations_off_for_test(&mut self) {
        self.prefs.animations_enabled = false;
    }

    /// Whether this screen scrolls as one page at this size.
    fn screen_scrolls(&self, size: egui::Vec2) -> bool {
        match self.screen {
            Screen::Preferences | Screen::Aliases | Screen::AbcEditor => true,
            Screen::Session => session_layout(size.x, size.y).page_scroll,
            // The saved-content pickers switch to a single scrolling column
            // below 760 points; above it the panes are fixed.
            Screen::DancePicker | Screen::TunePicker => size.x < 760.0,
            // The download screens scroll as one page only in the phone layout.
            Screen::DanceDownload | Screen::TuneDownload => compact_layout(size.x),
        }
    }

    /// Top-level controls that must stay vertically visible when the screen
    /// does not scroll.
    fn top_level_controls(&self) -> &'static [&'static str] {
        match self.screen {
            Screen::Session => &[
                "Play", "Pause", "Restart", "Loop", "Count-in", "Tempo", "Volume",
            ],
            Screen::DancePicker => &["Download dance"],
            Screen::TunePicker => &["Download tune"],
            Screen::DanceDownload => &["< Back to saved dances"],
            Screen::TuneDownload => &["< Back to saved tunes"],
            Screen::Preferences | Screen::Aliases | Screen::AbcEditor => &[],
        }
    }
}

/// Every node rectangle in the current tree, with its role and label.
fn node_rects(harness: &Harness<'_, ContraApp>) -> Vec<(egui::Rect, accesskit::Role, String)> {
    let mut nodes = Vec::new();
    for node in harness.root().children_recursive() {
        let access = node.accesskit_node();
        if access.is_hidden() {
            continue;
        }
        let Some(bounds) = access.bounding_box() else {
            continue;
        };
        let rect = egui::Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        );
        nodes.push((
            rect,
            access.role(),
            access.label().unwrap_or_default().to_owned(),
        ));
    }
    nodes
}

fn assert_inside_horizontally(
    nodes: &[(egui::Rect, accesskit::Role, String)],
    size: egui::Vec2,
    context: &str,
) {
    const EPSILON: f32 = 0.6;
    for (rect, role, label) in nodes {
        if rect.width() <= 0.0 && rect.height() <= 0.0 {
            continue;
        }
        assert!(
            rect.left() >= -EPSILON && rect.right() <= size.x + EPSILON,
            "{context}: {role:?} {label:?} at {rect:?} overflows the {:.0}-point viewport",
            size.x
        );
    }
}

/// The bundled dance is what a caller sees first, so its whole card must fit
/// on the desktop session tab without scrolling. The card pane hides its
/// scroll bar when the figures fit, which makes the bar a direct check.
#[test]
fn the_default_dance_card_fits_without_scrolling_on_a_desktop() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Session;
            app.started = false;
            app.needs_render = false;
            app
        });
    harness.run_steps(3);
    let scroll_bars: Vec<_> = node_rects(&harness)
        .into_iter()
        .filter(|(_, role, _)| *role == accesskit::Role::ScrollBar)
        .collect();
    assert!(
        scroll_bars.is_empty(),
        "the default dance card scrolls at 1280x800: {scroll_bars:?}"
    );
}

#[test]
fn every_screen_fits_horizontally_at_every_supported_width() {
    for screen in SCREENS {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(800.0, 844.0))
            .with_theme(egui::Theme::Dark)
            .with_os(egui::os::OperatingSystem::Windows)
            .allow_missing_glyphs()
            .with_max_steps(10)
            .build_eframe(|cc| {
                let mut app = demo_app(cc);
                app.screen = Screen::Session;
                app
            });
        for width in WIDTHS {
            for height in heights() {
                let size = egui::vec2(*width, height);
                harness.set_size(size);
                harness.state_mut().screen = screen;
                harness.run_steps(3);
                let nodes = node_rects(&harness);
                assert_inside_horizontally(
                    &nodes,
                    size,
                    &format!("{screen:?} at {width}x{height}"),
                );
            }
        }
    }
}

#[test]
fn phone_preferences_category_rows_share_one_left_edge() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(390.0, 844.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Preferences;
            app.prefs_page = PreferencesPage::Categories;
            app
        });
    harness.run_steps(3);
    let nodes = node_rects(&harness);
    let mut rows: Vec<egui::Rect> = Vec::new();
    for category in PreferencesCategory::ALL {
        let prefix = format!("{}. ", category.label());
        let Some((rect, _, _)) = nodes.iter().find(|(_, role, label)| {
            *role == accesskit::Role::Button && label.starts_with(&prefix)
        }) else {
            panic!("no category row for {}", category.label());
        };
        rows.push(*rect);
    }
    let first = rows[0];
    for (rect, category) in rows.iter().zip(PreferencesCategory::ALL) {
        assert!(
            (rect.left() - first.left()).abs() < 0.6,
            "{} starts at {:.1}, not {:.1}",
            category.label(),
            rect.left(),
            first.left()
        );
        assert!(
            (rect.height() - first.height()).abs() < 0.6,
            "{} is {:.1} points tall, not {:.1}",
            category.label(),
            rect.height(),
            first.height()
        );
        assert!(
            rect.width() >= 360.0 - 2.0 * 12.0,
            "{} row is only {:.1} wide",
            category.label(),
            rect.width()
        );
    }
}

#[test]
fn a_swipe_that_starts_on_a_control_does_not_change_tabs() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 844.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.headless = false;
            app.screen = Screen::Session;
            app
        });
    harness.run_steps(3);

    let play = harness
        .query_all_by_label("Play")
        .next()
        .expect("the session has a Play button")
        .rect()
        .center();
    harness.hover_at(play);
    harness.run_steps(1);
    harness.drag_at(play);
    harness.run_steps(1);
    let moved = play - egui::vec2(140.0, 0.0);
    harness.hover_at(moved);
    harness.run_steps(1);
    harness.drop_at(moved);
    harness.run_steps(2);
    assert_eq!(
        harness.state().screen,
        Screen::Session,
        "a drag that starts on a control belongs to that control"
    );

    // The gap between the two session boxes is not a widget, so a horizontal
    // drag there still changes tabs.
    let blank = egui::pos2(400.0, 700.0);
    harness.hover_at(blank);
    harness.run_steps(1);
    harness.drag_at(blank);
    harness.run_steps(1);
    harness.hover_at(blank - egui::vec2(140.0, 0.0));
    harness.run_steps(1);
    harness.drop_at(blank - egui::vec2(140.0, 0.0));
    harness.run_steps(2);
    assert_eq!(harness.state().screen, Screen::DancePicker);
}

#[test]
fn the_header_and_top_level_controls_stay_vertical_where_the_page_does_not_scroll() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 844.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(demo_app);

    for screen in SCREENS {
        for width in WIDTHS {
            for height in heights() {
                let size = egui::vec2(*width, height);
                harness.set_size(size);
                let app = harness.state_mut();
                app.screen = screen;
                if app.screen_scrolls(size) {
                    continue;
                }
                let controls = app.top_level_controls().to_vec();
                harness.run_steps(3);

                // The tab row and gear are always visible.
                for label in ["Session", "Select dance", "Select Tune", "Preferences"] {
                    let Some(node) = harness.query_all_by_label(label).next() else {
                        continue;
                    };
                    let rect = node.rect();
                    assert!(
                        rect.top() >= -0.6 && rect.bottom() <= size.y + 0.6,
                        "{screen:?} at {width}x{height}: header {label:?} at {rect:?} is outside"
                    );
                }
                for label in controls {
                    let Some(node) = harness.query_all_by_label(label).next() else {
                        continue;
                    };
                    let rect = node.rect();
                    assert!(
                        rect.top() >= -0.6 && rect.bottom() <= size.y + 0.6,
                        "{screen:?} at {width}x{height}: {label:?} at {rect:?} is outside the viewport"
                    );
                }
            }
        }
    }
}
