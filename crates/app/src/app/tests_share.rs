//! Tests for the web share link: encoding a session and rebuilding it.
//!
//! The browser entry point reads the `?set=` argument; everything after that is
//! ordinary app state, so the tests drive the same `apply_share_code` path
//! without a browser.

use super::*;
use caller_core::{Figure, SourceRef};

fn test_tune(id: &str, title: &str) -> Tune {
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
        abc: format!("X:1\nT:{title}\nK:D\n|: FA G2 |\n"),
        original_abc: None,
        credits: "ABC Session Tunes".into(),
    }
}

fn test_dance() -> Dance {
    Dance {
        source: SourceRef::new("contradb", "9001", "https://contradb.com/dances/9001"),
        title: "A Shared Dance".into(),
        choreographer: "A. Caller".into(),
        formation: "improper".into(),
        preamble: String::new(),
        notes: String::new(),
        figures: vec![
            Figure::new(Some("A1".into()), 8.0, "neighbors do si do"),
            Figure::new(Some("A2".into()), 8.0, "ladles allemande left 1½"),
        ],
    }
}

fn payload() -> SharePayload {
    SharePayload {
        schema_version: share::SCHEMA_VERSION,
        dance: test_dance(),
        tunes: vec![
            test_tune("11", "Opening Reel"),
            test_tune("12", "Middle Reel"),
            test_tune("13", "Closing Reel"),
        ],
        current: 1,
        rotation: true,
        passes_per_tune: 3,
    }
}

fn test_app() -> (CallerApp, egui::Context) {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());
    let app = CallerApp::new_headless(&cc);
    (app, ctx)
}

#[test]
fn a_shared_link_rebuilds_the_dance_and_the_ordered_tune_set() {
    let (mut app, _ctx) = test_app();
    let expected = payload();
    let code = share::encode(&expected).expect("encode");
    let message = app.apply_share_code(&code).expect("apply");

    assert_eq!(app.dance, expected.dance);
    assert_eq!(app.tune, expected.tunes[expected.current]);
    assert_eq!(app.screen, Screen::Session);
    assert_eq!(app.error, None);
    assert!(message.contains("A Shared Dance"));
    assert_eq!(app.status, message);

    // The set is the explicit list, in link order, with the stored interval.
    assert_eq!(app.prefs.tune_rotation.source, TuneSetSource::List);
    assert!(app.prefs.tune_rotation.enabled);
    assert_eq!(app.prefs.tune_rotation.passes_per_tune, 3);
    let sources: Vec<_> = app
        .prefs
        .tune_rotation
        .tunes
        .iter()
        .map(|source| source.id.as_str())
        .collect();
    assert_eq!(sources, ["11", "12", "13"]);

    // Every card and tune was saved, so the session still resolves after the
    // link is gone and no download is needed.
    assert!(app.dance_library.get("contradb", "9001").is_some());
    for tune in &expected.tunes {
        assert!(
            app.tune_library
                .get(&tune.source.provider, &tune.source.id)
                .is_some(),
            "{} should be saved",
            tune.title
        );
    }
    assert_eq!(app.rotation_tunes().len(), 3);
}

#[test]
fn a_link_round_trips_through_the_app_made_url() {
    let (mut app, _ctx) = test_app();
    app.use_dance(test_dance());
    for tune in payload().tunes {
        app.tune_library
            .insert(tune.clone(), None, library::now_seconds());
        app.prefs.tune_rotation.add(tune.source.clone());
    }
    app.tune = payload().tunes[2].clone();
    app.prefs.tune_rotation.enabled = true;
    app.prefs.tune_rotation.passes_per_tune = 5;

    let link = app.share_link().expect("build a link");
    let code = share::code_from_url(&link).expect("the link carries the argument");

    let (mut other, _ctx) = test_app();
    other.apply_share_code(&code).expect("apply");
    assert_eq!(other.dance.title, "A Shared Dance");
    assert_eq!(other.tune.source.id, "13");
    assert_eq!(other.prefs.tune_rotation.passes_per_tune, 5);
    assert_eq!(other.rotation_tunes().len(), 3);
    assert_eq!(
        other
            .rotation_tunes()
            .iter()
            .map(|tune| tune.title.as_str())
            .collect::<Vec<_>>(),
        ["Opening Reel", "Middle Reel", "Closing Reel"]
    );
}

#[test]
fn a_broken_link_leaves_the_current_session_alone() {
    let (mut app, _ctx) = test_app();
    let dance = app.dance.clone();
    let tune = app.tune.clone();
    let prefs = app.prefs.clone();
    let dances = app.dance_library.clone();
    let tunes = app.tune_library.clone();

    let error = app
        .apply_share_code("1.this is not base64")
        .expect_err("an invalid link must be refused");
    assert!(error.contains("link"), "unexpected message: {error}");

    assert_eq!(app.dance, dance);
    assert_eq!(app.tune, tune);
    assert_eq!(app.prefs, prefs);
    assert_eq!(app.dance_library, dances);
    assert_eq!(app.tune_library, tunes);
}

#[test]
fn a_link_with_one_tune_does_not_turn_on_cycling() {
    let (mut app, _ctx) = test_app();
    let mut payload = payload();
    payload.tunes.truncate(1);
    payload.current = 0;
    payload.rotation = true;
    let code = share::encode(&payload).expect("encode");
    app.apply_share_code(&code).expect("apply");

    assert!(!app.prefs.tune_rotation.enabled);
    assert_eq!(app.prefs.tune_rotation.tunes.len(), 1);
    app.rotation_settings_changed();
    assert_eq!(app.rotation_tunes().len(), 1);
}

#[test]
fn the_current_tune_joins_the_link_when_it_is_outside_the_set() {
    let (mut app, _ctx) = test_app();
    // Put one other tune in the rotation, while the default tune keeps playing.
    let other = test_tune("77", "Set Reel");
    app.tune_library
        .insert(other.clone(), None, library::now_seconds());
    app.prefs.tune_rotation.add(other.source.clone());

    let payload = app.share_payload();
    assert_eq!(payload.current, 0);
    assert_eq!(payload.tunes[0].source.id, app.tune.source.id);
    assert_eq!(payload.tunes[1].source.id, "77");
    assert_eq!(
        payload.tunes[payload.current].source.id, app.tune.source.id,
        "the opening tune is the one that was playing"
    );
}

#[test]
fn repeated_tune_sources_collapse_without_changing_the_opening_tune() {
    let (mut app, _ctx) = test_app();
    let mut payload = payload();
    // A hand-edited link can repeat a tune. The opening index still points at
    // the middle tune, which must remain selected after the duplicate is gone.
    payload.tunes.insert(0, payload.tunes[0].clone());
    payload.current = 2;
    let code = share::encode(&payload).expect("encode");
    app.apply_share_code(&code).expect("apply");

    assert_eq!(app.tune.title, "Middle Reel");
    assert_eq!(app.rotation_tunes().len(), 3);
}

#[test]
fn opening_the_share_panel_builds_a_decodable_link() {
    let (mut app, _ctx) = test_app();
    app.open_share_link();

    let panel = app.share.as_ref().expect("the panel opens");
    let code = share::code_from_url(&panel.link).expect("the link carries the argument");
    let payload = share::decode(&code).expect("the link decodes");
    assert_eq!(payload.dance, app.dance);
    assert_eq!(payload.tunes[0], app.tune);
    // Without a browser the copy step reports why instead of failing silently.
    assert!(panel.message.starts_with("Could not copy"));
}

#[test]
fn changing_the_session_closes_a_stale_share_panel() {
    let (mut app, _ctx) = test_app();
    app.open_share_link();
    assert!(app.share.is_some());

    app.use_tune(test_tune("88", "Another Tune"));
    assert!(app.share.is_none(), "a new tune makes the link stale");
}

#[test]
fn the_share_panel_fits_desktop_and_phone_widths() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    for (width, compact) in [(360.0_f32, true), (390.0, true), (1280.0, false)] {
        let (mut app, _ctx) = test_app();
        app.open_share_link();
        assert!(app.share.is_some(), "the panel opens at {width} points");

        let mut harness = Harness::builder()
            .with_size(egui::vec2(width, 800.0))
            .with_theme(egui::Theme::Dark)
            .build_ui_state(
                |ui, app: &mut CallerApp| {
                    egui::Frame::NONE.show(ui, |ui| app.share_link_ui(ui, compact));
                },
                app,
            );
        harness.run();

        // Both actions and the panel heading are drawn; a screen reader and the
        // MCP tree can find the control by name.
        harness.get_by_label("Share this session");
        harness.get_by_label("Close");

        let copy = harness.get_by_label("Copy link").rect();
        assert!(
            copy.right() <= width + 0.5 && copy.left() >= -0.5,
            "Copy link overflows a {width}-point viewport: {copy:?}"
        );
        if compact {
            assert!(
                copy.height() >= 44.0,
                "Copy link keeps a phone touch target: {copy:?}"
            );
        }

        harness.get_by_label("Copy link").click();
        harness.run_steps(2);
        assert!(
            harness
                .state()
                .share
                .as_ref()
                .expect("the panel stays open")
                .message
                .starts_with("Could not copy"),
            "a native test has no browser clipboard to copy to"
        );

        harness.get_by_label("Close").click();
        harness.run_steps(2);
        assert!(harness.state().share.is_none(), "Close hides the panel");
    }
}
