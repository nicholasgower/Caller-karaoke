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

const SCREENS: [Screen; 8] = [
    Screen::Session,
    Screen::DancePicker,
    Screen::DanceDownload,
    Screen::TunePicker,
    Screen::TuneDownload,
    Screen::Preferences,
    Screen::Aliases,
    Screen::Glossary,
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
            Screen::Preferences | Screen::Aliases | Screen::AbcEditor | Screen::Glossary => true,
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
            Screen::Session => &["Play", "Restart", "Loop", "Count-in", "Tempo", "Volume"],
            Screen::DancePicker => &["Download dance"],
            Screen::TunePicker => &["Download tune"],
            Screen::DanceDownload => &["< Back to saved dances"],
            Screen::TuneDownload => &["< Back to saved tunes"],
            Screen::Preferences | Screen::Aliases | Screen::AbcEditor | Screen::Glossary => &[],
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

/// Every button is at least as tall as the session tab's Play button.
///
/// Play is the reference for an ordinary action button: 40 points on a desktop
/// and the 44-point touch target in the phone layout. A button built with
/// `small_button` (or any other shrunken size) is harder to hit and reads as a
/// lesser control, so it must not ship. Deliberately taller rows — a
/// multi-line search result, a phone Preferences category, a folder row — are
/// fine; only shrinking below Play is the regression this catches.
#[test]
fn no_button_is_shorter_than_the_session_play_button() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(demo_app);

    for size in [
        egui::vec2(1280.0, 800.0),
        egui::vec2(390.0, 844.0),
        // The minimum window: the phone layout must hold its touch height even
        // in the shortest supported window.
        egui::vec2(360.0, 560.0),
    ] {
        harness.set_size(size);
        harness.state_mut().screen = Screen::Session;
        harness.run_steps(3);
        let play_height = node_rects(&harness)
            .into_iter()
            .find(|(_, role, label)| *role == accesskit::Role::Button && label == "Play")
            .map(|(rect, _, _)| rect.height())
            .unwrap_or_else(|| panic!("the session has a Play button at {size:?}"));
        assert!(
            play_height >= 39.4,
            "the Play button itself shrank to {play_height:.1} points at {size:?}"
        );

        for screen in SCREENS {
            harness.state_mut().screen = screen;
            harness.run_steps(3);
            for (rect, role, label) in node_rects(&harness) {
                if role != accesskit::Role::Button {
                    continue;
                }
                assert!(
                    rect.height() + 0.6 >= play_height,
                    "{screen:?} at {size:?}: button {label:?} is {:.1} points tall, \
                     shorter than the {play_height:.1}-point Play button",
                    rect.height()
                );
            }
        }
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
fn preferences_header_places_feedback_and_update_actions_beside_the_title() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .with_accessibility_check(false)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Preferences;
            app.prefs_page = PreferencesPage::Category(PreferencesCategory::Calling);
            app
        });
    harness.run_steps(3);

    let nodes = node_rects(&harness);
    let heading = nodes
        .iter()
        .find(|(_, role, label)| *role != accesskit::Role::Button && label == "Preferences")
        .map(|(rect, _, _)| *rect)
        .expect("the Preferences title is on screen");
    let rect_for = |label: &str| {
        nodes
            .iter()
            .find(|(_, role, text)| *role == accesskit::Role::Button && text == label)
            .map(|(rect, _, _)| *rect)
            .unwrap_or_else(|| panic!("the {label} button is on screen"))
    };
    let feedback = rect_for("Submit feedback");
    let updates = rect_for("Check for updates");

    // The title and both actions share one row, with the update action in the
    // rightmost position and feedback immediately to its left.
    assert!(
        (feedback.center().y - heading.center().y).abs() < 14.0,
        "Submit feedback is not on the title row: {feedback:?} vs {heading:?}"
    );
    assert!(
        (updates.center().y - heading.center().y).abs() < 14.0,
        "Check for updates is not on the title row: {updates:?} vs {heading:?}"
    );
    assert!(
        feedback.left() > heading.right(),
        "the actions should sit to the right of the title: {feedback:?} vs {heading:?}"
    );
    assert!(
        updates.left() > feedback.right(),
        "Check for updates should be the rightmost action: {updates:?} vs {feedback:?}"
    );
    assert!(
        updates.right() > 1000.0,
        "Check for updates should sit at the top right of the page: {updates:?}"
    );

    // At the 360-point minimum width both long labels move under the title but
    // stay on screen, so the header cannot widen the page. The phone category
    // list is in front here so the test does not depend on the settings below
    // it; the header is drawn by both pages.
    harness.set_size(egui::vec2(360.0, 640.0));
    harness.state_mut().prefs_page = PreferencesPage::Categories;
    harness.run_steps(3);
    let nodes = node_rects(&harness);
    for label in ["Submit feedback", "Check for updates"] {
        let rect = nodes
            .iter()
            .find(|(_, role, text)| *role == accesskit::Role::Button && text == label)
            .map(|(rect, _, _)| *rect)
            .unwrap_or_else(|| panic!("the {label} button is on screen at 360 points"));
        assert!(
            rect.left() >= 0.0 && rect.right() <= 360.0 + 0.6,
            "{label} overflows the minimum window: {rect:?}"
        );
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
fn the_preferences_search_finds_a_setting_and_opens_its_category() {
    // A phone starts on the category list and drills into one category, so the
    // test stays on the two small pages the search connects rather than the
    // long desktop settings column beside the list.
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

    harness.get_by_label("Search settings").click();
    harness.run_steps(1);
    // One key per frame: the results replace the category list below the
    // field, and the field must keep focus across that layout change.
    for character in "theme".chars() {
        harness
            .get_by_label("Search settings")
            .type_text(&character.to_string());
        harness.run_steps(1);
    }

    assert_eq!(harness.state().prefs_query, "theme");
    assert!(
        harness
            .query_all_by_label("Theme. Appearance")
            .next()
            .is_some(),
        "the search should offer the Theme setting in Appearance"
    );

    harness.get_by_label("Theme. Appearance").click();
    harness.run_steps(3);

    assert_eq!(
        harness.state().prefs_page,
        PreferencesPage::Category(PreferencesCategory::Appearance)
    );
    assert!(
        harness.state().prefs_query.is_empty(),
        "opening a result returns the screen to the normal category layout"
    );
    assert!(
        harness.state().prefs_reveal.is_none(),
        "the requested section is revealed once and then stops pulling the list"
    );
}

#[test]
fn the_desktop_preferences_search_opens_a_result() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Preferences;
            app.prefs_page = PreferencesPage::Categories;
            app.prefs_query = "theme".into();
            app
        });
    harness.run_steps(3);

    assert!(
        harness
            .query_all_by_label("Theme. Appearance")
            .next()
            .is_some(),
        "the desktop search should list Theme in Appearance"
    );
    harness.get_by_label("Theme. Appearance").click();
    harness.run_steps(3);

    assert_eq!(
        harness.state().prefs_page,
        PreferencesPage::Category(PreferencesCategory::Appearance)
    );
    assert!(harness.state().prefs_query.is_empty());
}

#[test]
fn the_preferences_search_reports_when_nothing_matches() {
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

    harness.get_by_label("Search settings").click();
    harness.run_steps(1);
    harness
        .get_by_label("Search settings")
        .type_text("no such setting");
    harness.run_steps(2);

    assert!(
        harness
            .query_all_by_label("No settings match \"no such setting\".")
            .next()
            .is_some(),
        "the phone search should explain an empty result"
    );
    assert!(
        harness.query_all_by_label("Clear search").next().is_some(),
        "the phone search offers a full-width Clear search button"
    );
}

#[test]
fn a_glossary_term_opens_its_entry_and_the_tab_returns_to_the_session() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Session;
            app.error = None;
            app.started = true;
            app.needs_render = false;
            app.audio.play();
            app.audio.seek_beat(4.0);
            app
        });
    harness.run_steps(3);

    let term = harness
        .query_all_by_label("balance and swing")
        .next()
        .expect("the prompt and dance card link the balance and swing figure");
    term.click();
    harness.run_steps(3);

    assert_eq!(harness.state().screen, Screen::Glossary);
    assert_eq!(
        harness.state().glossary_focus.as_deref(),
        Some("Balance and swing"),
        "the entry stays focused after the opening frame"
    );
    assert!(harness.state().glossary_focus_scrolled);

    harness.get_by_label("Session").click();
    harness.run_steps(2);
    assert_eq!(harness.state().screen, Screen::Session);
}

#[test]
fn the_glossary_tab_opens_the_reference_from_the_header() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Session;
            app.error = None;
            app
        });
    harness.run_steps(3);

    harness.get_by_label("Dance glossary").click();
    harness.run_steps(2);
    assert_eq!(
        harness.state().screen,
        Screen::Glossary,
        "the fourth tab opens the dance glossary"
    );
}

#[test]
fn disabling_the_glossary_hides_its_links_and_entry_points() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Session;
            app.error = None;
            app.prefs.glossary_enabled = false;
            app
        });
    harness.run_steps(3);

    assert!(
        harness
            .query_all_by_label("Dance glossary")
            .next()
            .is_none(),
        "the glossary tab should not be offered while the reference is off"
    );
    assert!(
        harness.query_all_by_label("Glossary").next().is_none(),
        "the Dance card header should not offer the glossary while it is off"
    );
    assert!(
        harness
            .query_all_by_label("balance and swing")
            .next()
            .is_none(),
        "dance terms must not remain links while the glossary is off"
    );

    harness.state_mut().open_glossary(None);
    assert_eq!(
        harness.state().screen,
        Screen::Session,
        "the glossary screen is unreachable while it is off"
    );

    harness.state_mut().screen = Screen::Preferences;
    harness.state_mut().prefs_page = PreferencesPage::Category(PreferencesCategory::About);
    harness.state_mut().about_reveal = true;
    harness.run_steps(3);
    assert!(
        harness
            .query_all_by_label("Open the dance glossary")
            .next()
            .is_none(),
        "the About link must disappear with the rest of the glossary"
    );
}

#[test]
fn hovering_a_dance_card_term_shows_its_glossary_definition() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Session;
            app.error = None;
            app
        });
    harness.ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.interaction.tooltip_delay = 0.0
    });
    harness.run_steps(3);

    let term = harness
        .query_all_by_label("balance and swing")
        .next()
        .expect("the dance card links the balance and swing figure");
    let term_center = term.rect().center();
    harness.hover_at(term_center);
    harness.run_steps(3);

    let definition = harness
        .query_all_by_label("Balance first, then swing the same partner or neighbor.")
        .next()
        .expect("hovering a linked term should show its glossary definition");
    let tooltip = definition.rect();
    assert!(
        tooltip.top() > 100.0,
        "the desktop tooltip should sit near the pointer, not at the top of the screen: {tooltip:?}"
    );
    assert!(
        (tooltip.center().x - term_center.x).abs() < 420.0,
        "the desktop tooltip should stay beside the hovered term: {tooltip:?} vs {term_center:?}"
    );
}

/// Drag one finger from `from` to `to`, the way a touch screen reports it.
///
/// `egui-winit` and the web backend both send the touch event plus the pointer
/// events any backend synthesizes from it, in the same frame. A test that sends
/// pointer events alone is a mouse gesture, which the app deliberately ignores
/// for tab changes.
fn touch_drag(harness: &mut Harness<'_, ContraApp>, from: egui::Pos2, to: egui::Pos2) {
    const STEPS: u32 = 4;
    let touch = |phase, pos| egui::Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(1),
        phase,
        pos,
        force: None,
    };
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };

    harness.input_mut().events.extend([
        touch(egui::TouchPhase::Start, from),
        egui::Event::PointerMoved(from),
        button(from, true),
    ]);
    harness.run_steps(1);

    for step in 1..=STEPS {
        let t = step as f32 / STEPS as f32;
        let pos = egui::pos2(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        harness.input_mut().events.extend([
            touch(egui::TouchPhase::Move, pos),
            egui::Event::PointerMoved(pos),
        ]);
        harness.run_steps(1);
    }

    harness.input_mut().events.extend([
        touch(egui::TouchPhase::End, to),
        button(to, false),
        egui::Event::PointerGone,
    ]);
    harness.run_steps(2);
}

/// Tap once with a finger, the way a touch screen reports it.
fn touch_tap(harness: &mut Harness<'_, ContraApp>, pos: egui::Pos2) {
    let touch = |phase, pos| egui::Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(1),
        phase,
        pos,
        force: None,
    };
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };

    harness.input_mut().events.extend([
        touch(egui::TouchPhase::Start, pos),
        egui::Event::PointerMoved(pos),
        button(pos, true),
    ]);
    harness.run_steps(1);
    harness.input_mut().events.extend([
        touch(egui::TouchPhase::End, pos),
        button(pos, false),
        egui::Event::PointerGone,
    ]);
    harness.run_steps(2);
}

/// A swipe that starts on a control belongs to that control, and a mouse drag
/// is never a tab swipe.
#[test]
fn only_a_finger_swipe_on_empty_content_changes_tabs() {
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

    // The gap between the two session boxes is not a widget, so a horizontal
    // drag there is a tab swipe when it comes from a finger.
    let blank = egui::pos2(400.0, 700.0);

    // A mouse drag over the same empty content is not a swipe: it selects text
    // and grabs content, so the tab must not change under the user.
    harness.hover_at(blank);
    harness.run_steps(1);
    harness.drag_at(blank);
    harness.run_steps(1);
    harness.hover_at(blank - egui::vec2(140.0, 0.0));
    harness.run_steps(1);
    harness.drop_at(blank - egui::vec2(140.0, 0.0));
    harness.run_steps(2);
    assert_eq!(
        harness.state().screen,
        Screen::Session,
        "a mouse drag must not change tabs"
    );

    let play = harness
        .query_all_by_label("Play")
        .next()
        .expect("the session has a Play button")
        .rect()
        .center();
    touch_drag(&mut harness, play, play - egui::vec2(140.0, 0.0));
    assert_eq!(
        harness.state().screen,
        Screen::Session,
        "a swipe that starts on a control belongs to that control"
    );

    // An empty gap is not a control, so a finger swipe there changes tabs.
    touch_drag(&mut harness, blank, blank - egui::vec2(140.0, 0.0));
    assert_eq!(harness.state().screen, Screen::DancePicker);
}

/// On a touch screen, a level control ignores a drag until it has been tapped.
#[test]
fn phone_sliders_are_guarded_until_tapped() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(390.0, 844.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = demo_app(cc);
            app.screen = Screen::Session;
            app
        });
    harness.run_steps(3);

    let tempo = node_rects(&harness)
        .into_iter()
        .find(|(_, role, label)| *role == accesskit::Role::Slider && label == "Tempo")
        .map(|(rect, _, _)| rect)
        .expect("the session has a Tempo slider");
    let start = egui::pos2(tempo.left() + tempo.width() * 0.3, tempo.center().y);
    let end = start + egui::vec2(80.0, 0.0);
    let before = harness.state().prefs.tempo_bpm;

    // A finger drag that starts on the locked slider leaves the tempo alone.
    touch_drag(&mut harness, start, end);
    assert_eq!(
        harness.state().prefs.tempo_bpm,
        before,
        "a locked slider changed the tempo"
    );

    // A tap arms it; the drag that follows adjusts the level.
    touch_tap(&mut harness, start);
    assert_eq!(
        harness.state().prefs.tempo_bpm,
        before,
        "the arming tap changed the tempo"
    );

    touch_drag(&mut harness, start, end);
    assert!(
        harness.state().prefs.tempo_bpm > before,
        "the armed slider ignored the drag"
    );
}

#[test]
fn the_transport_has_one_play_pause_button() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 844.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(demo_app);
    harness.run_steps(3);

    // Stopped: one button, and its accessible name is the action it offers.
    assert!(
        harness.get_by_label("Play").rect().height() >= 40.0,
        "the Play/Pause button keeps a full-size target"
    );
    assert!(
        harness.query_all_by_label("Pause").next().is_none(),
        "the separate Pause button is gone"
    );

    harness.get_by_label("Play").click();
    harness.run_steps(3);
    assert!(harness.state().audio.is_playing());
    assert!(
        harness.query_all_by_label("Play").next().is_none(),
        "a running session offers Pause, not Play"
    );
    assert!(
        harness.query_all_by_label("Pause").next().is_some(),
        "the same button now pauses"
    );

    harness.get_by_label("Pause").click();
    harness.run_steps(3);
    assert!(!harness.state().audio.is_playing());
    assert!(
        harness.query_all_by_label("Play").next().is_some(),
        "pausing turns the button back into Play"
    );
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
                for label in [
                    "Session",
                    "Select dance",
                    "Select Tune",
                    "Dance glossary",
                    "Preferences",
                ] {
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
