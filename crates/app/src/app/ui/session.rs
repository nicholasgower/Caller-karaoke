use super::*;
use crate::app::*;
use crate::audio::AudioEngine;
use crate::theme;
use crate::tts::Speech;
use caller_core::{CardPadding, Preferences};
use eframe::egui;

/// The instrument line under the tune.
///
/// Each editable band row appears once, with its player count when the section
/// has more than one player. `×2` reads as two players on that part, which is
/// how a band describes a section.
fn instrument_summary(prefs: &Preferences) -> String {
    let slots = prefs.band_slots();
    let parts = slots
        .iter()
        .map(|slot| part_summary(slot.part.label(), slot.voice.label(), slot.player_count()))
        .collect::<Vec<_>>();
    parts.join(" · ")
}

fn part_summary(part: &str, instrument: &str, players: usize) -> String {
    format!("{part}: {}", part_copy_label(instrument, players))
}

fn part_copy_label(instrument: &str, players: usize) -> String {
    if players > 1 {
        format!("{instrument} ×{players}")
    } else {
        instrument.to_string()
    }
}

/// The session tempo slider's effective range.
///
/// The user's configured window (200–300 BPM by default) is widened, never
/// narrowed, to include the current tempo. A tempo restored from a file or set
/// before the window changed therefore stays selectable instead of snapping to
/// an endpoint.
fn tempo_slider_bounds(min_bpm: f64, max_bpm: f64, current_bpm: f64) -> (f64, f64) {
    let min = min_bpm.clamp(caller_core::Tempo::MIN_BPM, caller_core::Tempo::MAX_BPM);
    let max = max_bpm.clamp(caller_core::Tempo::MIN_BPM, caller_core::Tempo::MAX_BPM);
    let (low, high) = if min <= max { (min, max) } else { (max, min) };
    (low.min(current_bpm), high.max(current_bpm))
}

/// The tempo, transport, loop/count-in/speech, and volume controls.
///
/// On a phone the controls are stacked so each row owns the full width: a
/// full-width rail with a value box is usable with a thumb, while the desktop
/// wrapping row leaves the tempo slider barely a finger wide. The wide layout
/// keeps the original single wrapping row.
fn session_controls(
    ui: &mut egui::Ui,
    prefs: &mut Preferences,
    audio: &mut AudioEngine,
    speech: &mut Speech,
    compact: bool,
) -> SessionControlChanges {
    let mut changes = SessionControlChanges::default();
    let (tempo_min, tempo_max) =
        tempo_slider_bounds(prefs.tempo_min_bpm, prefs.tempo_max_bpm, prefs.tempo_bpm);

    if compact {
        ui.horizontal(|ui| {
            ui.label("Tempo");
            // Leave room for the value box and the " BPM" suffix.
            ui.spacing_mut().slider_width = (ui.available_width() - 100.0).max(96.0);
            let response = scroll_safe_slider(ui, &mut prefs.tempo_bpm, |ui, tempo| {
                number_slider(
                    ui,
                    egui::Slider::new(tempo, tempo_min..=tempo_max)
                        .fixed_decimals(0)
                        .suffix(" BPM"),
                )
            });
            // A stable accessible name, so the slider can be found by name in
            // every layout (and by the MCP session tools).
            let response = response.accessible_name("Tempo");
            if response.changed() {
                changes.timeline_dirty = true;
                changes.music_dirty = true;
            }
        });

        // Equal-width transport buttons filling the row: no control hides
        // behind hover, each is a full touch target, and the last button ends at
        // the content edge instead of leaving empty space on the right. Play and
        // Pause share one toggle button.
        filled_button_row(ui, 3, |ui, width| {
            let size = egui::vec2(width, theme::TOUCH_TARGET);
            if play_pause_button(ui, audio.is_playing(), size).clicked() {
                changes.action = Some("toggle");
            }
            if ui
                .add_sized(size, egui::Button::new("Restart"))
                .on_hover_text("Restart (R)")
                .clicked()
            {
                changes.action = Some("restart");
            }
            let ending = audio.ending_requested();
            let in_ending = audio.in_ending();
            let can_end = !in_ending && !audio.is_finished() && audio.has_buffer();
            let end_label = if in_ending {
                "Ending…"
            } else if ending {
                "Cancel"
            } else {
                "End"
            };
            let response = ui
                .add_enabled_ui(can_end, |ui| {
                    ui.add_sized(size, egui::Button::new(end_label))
                })
                .inner
                .on_hover_text(if ending {
                    "Cancel the slow ending"
                } else {
                    "End the tune slowly after this time through"
                });
            if response.clicked() {
                changes.action = Some(if ending { "cancel-ending" } else { "end" });
            }
        });

        ui.horizontal_wrapped(|ui| {
            if ui.checkbox(&mut prefs.loop_enabled, "Loop").changed() {
                audio.set_looping(prefs.loop_enabled);
                changes.save = true;
            }
            if ui
                .checkbox(&mut prefs.tune_rotation.enabled, "Cycle tunes")
                .on_hover_text("Play through the tune set, changing at a repeat")
                .changed()
            {
                changes.rotation_dirty = true;
                changes.save = true;
            }
            if ui
                .checkbox(&mut prefs.count_in_enabled, "Count-in")
                .changed()
            {
                changes.count_in_dirty = true;
            }
            if ui.checkbox(&mut prefs.tts_enabled, "Speak calls").changed() {
                speech.set_enabled(prefs.tts_enabled);
                if !prefs.tts_enabled {
                    speech.stop();
                }
                changes.save = true;
            }
        });

        ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
        ui.horizontal(|ui| {
            ui.label("Caller");
            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(96.0);
            let response = scroll_safe_slider(ui, &mut prefs.caller_volume, |ui, volume| {
                number_slider(ui, egui::Slider::new(volume, 0.0..=1.5).max_decimals(2))
            });
            let response = response.accessible_name("Caller volume");
            if response.changed() {
                audio.set_caller_volume(prefs.caller_volume);
                changes.save = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Band");
            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(96.0);
            let response = scroll_safe_slider(ui, &mut prefs.master_volume, |ui, volume| {
                number_slider(ui, egui::Slider::new(volume, 0.0..=1.5).max_decimals(2))
            });
            let response = response.accessible_name("Band volume");
            if response.changed() {
                audio.set_volume(prefs.master_volume);
                changes.save = true;
            }
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.label("Tempo");
            if scroll_safe_slider(ui, &mut prefs.tempo_bpm, |ui, tempo| {
                number_slider(
                    ui,
                    egui::Slider::new(tempo, tempo_min..=tempo_max)
                        .text("BPM")
                        .fixed_decimals(0),
                )
            })
            .accessible_name("Tempo")
            .changed()
            {
                changes.timeline_dirty = true;
                changes.music_dirty = true;
            }
            // One Play/Pause toggle: the icon is painted because the bundled
            // fonts have no play or pause glyph.
            let side = ui.spacing().interact_size.y;
            if play_pause_button(ui, audio.is_playing(), egui::Vec2::splat(side)).clicked() {
                changes.action = Some("toggle");
            }
            if ui.button("Restart").on_hover_text("Restart (R)").clicked() {
                changes.action = Some("restart");
            }
            let ending = audio.ending_requested();
            let in_ending = audio.in_ending();
            let can_end = !in_ending && !audio.is_finished() && audio.has_buffer();
            let end_label = if in_ending {
                "Ending…"
            } else if ending {
                "Cancel ending"
            } else {
                "End tune"
            };
            if ui
                .add_enabled(can_end, egui::Button::new(end_label))
                .on_hover_text(if ending {
                    "Cancel the slow ending"
                } else {
                    "End the tune slowly after this time through"
                })
                .clicked()
            {
                changes.action = Some(if ending { "cancel-ending" } else { "end" });
            }
            if ui.checkbox(&mut prefs.loop_enabled, "Loop").changed() {
                audio.set_looping(prefs.loop_enabled);
                changes.save = true;
            }
            if ui
                .checkbox(&mut prefs.tune_rotation.enabled, "Cycle tunes")
                .on_hover_text("Play through the tune set, changing at a repeat")
                .changed()
            {
                changes.rotation_dirty = true;
                changes.save = true;
            }
            if ui
                .checkbox(&mut prefs.count_in_enabled, "Count-in")
                .changed()
            {
                changes.count_in_dirty = true;
            }
            if ui.checkbox(&mut prefs.tts_enabled, "Speak calls").changed() {
                speech.set_enabled(prefs.tts_enabled);
                if !prefs.tts_enabled {
                    speech.stop();
                }
                changes.save = true;
            }
        });
        // Caller and band levels sit on their own row so the two sliders keep
        // a usable width instead of overflowing the wrapped control row at a
        // wide window.
        ui.horizontal(|ui| {
            ui.label("Caller");
            if scroll_safe_slider(ui, &mut prefs.caller_volume, |ui, volume| {
                number_slider(ui, egui::Slider::new(volume, 0.0..=1.5))
            })
            .accessible_name("Caller volume")
            .changed()
            {
                audio.set_caller_volume(prefs.caller_volume);
                changes.save = true;
            }
            ui.label("Band");
            if scroll_safe_slider(ui, &mut prefs.master_volume, |ui, volume| {
                number_slider(ui, egui::Slider::new(volume, 0.0..=1.5))
            })
            .accessible_name("Band volume")
            .changed()
            {
                audio.set_volume(prefs.master_volume);
                changes.save = true;
            }
        });
    }

    changes
}

/// The single Play/Pause button.
///
/// `egui::Button` keeps the fill, stroke, focus, and click handling the other
/// transport buttons have; only the label is empty because the glyph is
/// painted. No bundled font has a play or pause glyph — Ubuntu-Light covers no
/// geometric shapes and the monochrome emoji fonts are not linked — so a
/// "▶"/"⏸" label would render as an empty box.
///
/// The accessible name follows the action: "Play" while the session is
/// stopped, "Pause" while the music runs. A screen reader announces what the
/// press will do, and the MCP session tools find the button by that name.
fn play_pause_button(ui: &mut egui::Ui, playing: bool, size: egui::Vec2) -> egui::Response {
    let name = if playing { "Pause" } else { "Play" };
    let response = ui
        .add_sized(size, egui::Button::new(""))
        .on_hover_text(format!("{name} (Space)"));
    if ui.is_rect_visible(response.rect) {
        let color = ui.style().interact(&response).fg_stroke.color;
        ui.painter()
            .extend(play_pause_shapes(response.rect, playing, color));
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), name));
    response
}

/// The painted icon inside the Play/Pause button, centered in `rect`.
///
/// A filled triangle pointing right reads as play and two upright bars read as
/// pause at any size. Every point stays inside `rect`, so the icon never paints
/// over the button's border.
fn play_pause_shapes(rect: egui::Rect, playing: bool, color: egui::Color32) -> Vec<egui::Shape> {
    let center = rect.center();
    let icon = rect.width().min(rect.height()) * 0.38;
    if playing {
        let bar = icon * 0.32;
        let gap = icon * 0.24;
        let top = center.y - icon / 2.0;
        let radius = egui::CornerRadius::same((bar * 0.35).round() as u8);
        let left = center.x - gap / 2.0 - bar;
        vec![
            egui::Shape::rect_filled(
                egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(bar, icon)),
                radius,
                color,
            ),
            egui::Shape::rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(center.x + gap / 2.0, top),
                    egui::vec2(bar, icon),
                ),
                radius,
                color,
            ),
        ]
    } else {
        let half = icon / 2.0;
        vec![egui::Shape::convex_polygon(
            vec![
                center + egui::vec2(-half * 0.8, -half),
                center + egui::vec2(-half * 0.8, half),
                center + egui::vec2(half, 0.0),
            ],
            color,
            egui::Stroke::NONE,
        )]
    }
}

/// The transport state named by the progress bar.
///
/// A caller can hear whether the music is running; without a visible state a
/// muted or unplugged session looks exactly like a paused one, and an agent
/// testing the session tab has nothing to assert against. The progress bar is
/// the one row that is always visible while the session runs, so it carries the
/// state as well as the beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TransportState {
    Ready,
    CountIn,
    Playing,
    Paused,
    Ending,
    Finished,
}

impl TransportState {
    fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::CountIn => "Count-in",
            Self::Playing => "Playing",
            Self::Paused => "Paused",
            Self::Ending => "Ending",
            Self::Finished => "Finished",
        }
    }
}

/// Name the current transport state.
///
/// Finished and Ready win over Paused: a completed run and a session that never
/// started are not paused states, even though neither is playing.
fn transport_state(
    started: bool,
    playing: bool,
    finished: bool,
    ending: bool,
    beat: f64,
    count_in_beats: f64,
) -> TransportState {
    if finished {
        TransportState::Finished
    } else if !started {
        TransportState::Ready
    } else if !playing {
        TransportState::Paused
    } else if ending {
        TransportState::Ending
    } else if count_in_beats > 0.0 && beat < -0.001 {
        TransportState::CountIn
    } else {
        TransportState::Playing
    }
}

/// Beats still to count before the loop starts.
///
/// Rounds up (the number a caller says) and caps at the count-in the installed
/// timeline actually has. The audio clock can land a hair below the first beat
/// at the very start, and `ceil` would otherwise show one more than configured.
fn count_in_beats_left(beat: f64, count_in_beats: f64) -> u32 {
    if count_in_beats <= 0.0 {
        return 0;
    }
    ((-beat).ceil().max(1.0) as u32).min(count_in_beats.ceil() as u32)
}

/// Width for the music progress bar: exactly the space the panel offers.
///
/// `egui::ProgressBar` rounds the outer rect it paints. An infinite desired
/// width puts that rect's right edge (and its rounded cap) at infinity: only
/// the left cap is drawn, and the oversized rect widens the scroll content so
/// the rows below it also run past the panel edge. A measured, finite width
/// keeps both caps on screen and the bar inside the panel.
fn progress_bar_width(available_width: f32) -> f32 {
    if available_width.is_finite() {
        available_width.max(0.0)
    } else {
        0.0
    }
}

/// The progress bar text: the explicit transport state plus the position.
fn progress_text(state: TransportState, beat: f64, loop_beats: f64, count_in_beats: f64) -> String {
    match state {
        TransportState::CountIn => {
            let left = count_in_beats_left(beat, count_in_beats);
            if left == 1 {
                "Count-in · 1 beat left".to_string()
            } else {
                format!("Count-in · {left} beats left")
            }
        }
        TransportState::Ending => "Ending · slow finish".to_string(),
        _ => format!(
            "{} · beat {:.0} / {:.0}",
            state.label(),
            beat.clamp(0.0, loop_beats),
            loop_beats
        ),
    }
}

/// The count-in named on the readiness card.
///
/// Reads the installed timeline's count-in, never a constant: a caller who
/// turned the count-in off must not be told four beats will play.
fn count_in_label(count_in_beats: f64) -> String {
    if count_in_beats > 0.0 {
        format!("{count_in_beats:.0}-beat count-in")
    } else {
        "no count-in".to_string()
    }
}

/// The detail line under the current call: the lead left, rounded to the
/// nearest half beat, or a muted note once the dancers are executing.
fn executes_in_text(phase: CallPhase, execute_beat: f64, beat: f64) -> String {
    if phase == CallPhase::Executing {
        "figure in progress".to_string()
    } else {
        format!(
            "executes in {} beats",
            format_half_beats((execute_beat - beat).max(0.0))
        )
    }
}

/// Lines of call text the prompt card reserves, even for a one-line call.
///
/// A longer call wraps and grows the card: the budget is a minimum, never a
/// clip. It is deliberately not measured from the text, because CALL NOW and
/// UPCOMING MOVE show different calls and must still be the same height.
const PROMPT_CARD_CALL_LINES: f32 = 2.0;

/// The text rows the prompt card reserves, counted in rows rather than points.
#[derive(Clone, Copy, Debug, PartialEq)]
struct CardRows {
    /// Lines of call text, at the call font size.
    call: f32,
    /// Rows of small text: the state label above the call.
    label: f32,
    /// Rows of body text: the timing detail and the next-call footer.
    body: f32,
}

impl CardMode {
    /// The rows the card reserves for this mode.
    ///
    /// All three modes reserve identical rows. CALL NOW fills the footer with
    /// the following call; UPCOMING MOVE already names that call in its main
    /// line and leaves the footer blank; the last figure has nothing to
    /// preview. Reserving the row in every mode is what stops the panel — and
    /// the controls below it — from resizing when the mode changes.
    fn card_rows(self) -> CardRows {
        CardRows {
            call: PROMPT_CARD_CALL_LINES,
            label: 1.0,
            body: 2.0,
        }
    }
}

/// The smallest body height the prompt card may take, in points.
///
/// `call_row`, `small_row`, and `body_row` are the measured heights of the text
/// the card draws, and `row_gap` the spacing between its widgets, so the
/// reservation matches the current font settings instead of assuming a ratio.
/// Every mode passes the same rows, which is what makes CALL NOW and UPCOMING
/// MOVE the same height.
fn prompt_card_min_height(
    call_row: f32,
    small_row: f32,
    body_row: f32,
    row_gap: f32,
    rows: CardRows,
) -> f32 {
    // The call is one widget however many lines it wraps to, so the gaps are
    // between widgets, not between text lines.
    let widgets = rows.label + rows.body + 1.0;
    let text_rows = rows.label * small_row + rows.body * body_row + rows.call * call_row;
    let gaps = (widgets - 1.0).max(0.0) * row_gap;
    text_rows + gaps
}

/// The call previewed in the prompt card's footer.
///
/// UPCOMING MOVE already names its call in the main line, so its footer stays
/// empty instead of repeating the words; when no call follows the card, the
/// footer stays empty too and never echoes the call already on the card.
fn card_footer(
    mode: CardMode,
    next: Option<&caller_core::ScheduledCall>,
) -> Option<&caller_core::ScheduledCall> {
    if mode == CardMode::UpcomingMove {
        return None;
    }
    next
}

/// Draw the card footer: the call to say after the one above it, with the beat
/// it executes and the beat its own call starts.
fn prompt_card_footer(
    ui: &mut egui::Ui,
    next: &caller_core::ScheduledCall,
    compact: bool,
    glossary_enabled: bool,
) -> Option<&'static caller_core::glossary::GlossaryEntry> {
    let timing = if compact {
        format!(
            "beat {:.0} · call {:.1}",
            next.execute_beat, next.start_beat
        )
    } else {
        format!(
            "at beat {:.0} (call at {:.1})",
            next.execute_beat, next.start_beat
        )
    };
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("NEXT:").small().color(theme::muted(ui)));
        let call_color = theme::on_card(ui);
        clicked = glossary_text(ui, &next.call.text, glossary_enabled, |text| {
            egui::RichText::new(text).strong().color(call_color)
        });
        ui.label(egui::RichText::new(timing).small().color(theme::muted(ui)));
    });
    clicked
}

/// The "Dance card" heading and its glossary button.
///
/// The button is the discoverable entry point for the reference; the glossary
/// terms themselves open the same screen. It keeps a full touch target on a
/// phone, where the heading row is otherwise only text.
fn dance_card_header(ui: &mut egui::Ui, compact: bool, glossary_enabled: bool) -> bool {
    let mut open = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Dance card").strong());
        if !glossary_enabled {
            return;
        }
        let button = if compact {
            ui.add_sized([110.0, theme::TOUCH_TARGET], egui::Button::new("Glossary"))
        } else {
            ui.button("Glossary")
        };
        if button.on_hover_text("Open the dance glossary").clicked() {
            open = true;
        }
    });
    open
}

impl CallerApp {
    pub(in crate::app) fn session_ui(&mut self, ui: &mut egui::Ui, beat: f64) {
        let layout = session_layout(ui.available_width(), ui.available_height());
        if layout.page_scroll {
            // A short phone cannot fit the prompt, controls, and dance card at
            // once. Scroll the whole page: the prompt starts at the top of the
            // content, and the card is reachable below the fold.
            egui::ScrollArea::vertical()
                .id_salt("session-page")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.session_body(ui, beat, layout, false);
                    ui.add_space(24.0);
                });
        } else {
            self.session_body(ui, beat, layout, true);
        }
    }

    fn session_body(
        &mut self,
        ui: &mut egui::Ui,
        beat: f64,
        layout: SessionLayout,
        card_scroll: bool,
    ) {
        let compact = layout.compact;
        let glossary_enabled = self.prefs.glossary_enabled;
        if compact {
            // The phone layout gives every interactive control a 44 pt row.
            ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
        }
        // The prompt is the first thing under the tab row: it is what the caller
        // reads mid-dance, so it keeps one fixed place at the top instead of
        // sitting between the summary and the dance card.
        let mut glossary_click = self.prompt_card(ui, beat, compact);
        ui.add_space(8.0);
        let SessionControlChanges {
            timeline_dirty,
            music_dirty,
            count_in_dirty,
            rotation_dirty,
            save,
            action,
        } = session_controls(
            ui,
            &mut self.prefs,
            &mut self.audio,
            &mut self.speech,
            compact,
        );
        let mut rotation_changed = rotation_dirty;

        if self.needs_render {
            ui.horizontal_wrapped(|ui| {
                let message = if self.started {
                    "Playback settings will apply without restarting; press Play to apply them now."
                } else {
                    "Press Play to prepare the music with these settings."
                };
                ui.label(egui::RichText::new(message).color(theme::muted(ui)));
            });
        }
        if let Some(warning) = &self.length_warning {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(warning)
                        .strong()
                        .color(theme::warning(ui)),
                );
            });
        }
        if let Some(arrangement) = &self.arrangement {
            for warning in &arrangement.warnings {
                ui.label(egui::RichText::new(warning).small().color(theme::muted(ui)));
            }
        }
        if let Some(error) = &self.audio_error {
            ui.colored_label(theme::error(ui), format!("Audio device: {error}"));
        }
        if self.prefs.tts_enabled {
            let status = self.speech.status();
            if let Some(error) = &status.error {
                ui.colored_label(
                    theme::error(ui),
                    format!("Neural voice: {error}. Using the platform voice."),
                );
            } else if !self.speech.available() {
                ui.colored_label(
                    theme::error(ui),
                    "Text-to-speech is not available on this system.",
                );
            } else if let Some(progress) = status.progress {
                ui.add(
                    egui::ProgressBar::new(progress)
                        .show_percentage()
                        .text(status.message.clone()),
                );
            } else if !status.ready {
                ui.label(
                    egui::RichText::new(&status.message)
                        .small()
                        .color(theme::muted(ui)),
                );
            }
        }

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        let mut open_glossary = false;
        if layout.split {
            // Wide windows use the remaining height for two boxes: dance
            // information and the beat meter on the left, the dance card on the
            // right. The prompt and the controls above stay full width.
            let height = ui.available_height().max(160.0);
            let frame = egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(10));
            ui.columns(2, |columns| {
                frame.show(&mut columns[0], |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("session-info")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            rotation_changed |= self.session_summary(ui, compact);
                            self.session_meter(ui, beat);
                        });
                });
                frame.show(&mut columns[1], |ui| {
                    open_glossary |= dance_card_header(ui, compact, glossary_enabled);
                    egui::ScrollArea::vertical()
                        .id_salt("dance-card")
                        .max_height((height - 44.0).max(120.0))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if let Some(entry) = self.dance_card_list(ui, beat, compact) {
                                glossary_click = Some(entry);
                            }
                        });
                });
            });
        } else {
            rotation_changed |= self.session_summary(ui, compact);
            self.session_meter(ui, beat);
            ui.add_space(8.0);
            open_glossary |= dance_card_header(ui, compact, glossary_enabled);
            if card_scroll {
                egui::ScrollArea::vertical()
                    .id_salt("dance-card")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(entry) = self.dance_card_list(ui, beat, compact) {
                            glossary_click = Some(entry);
                        }
                    });
            } else if let Some(entry) = self.dance_card_list(ui, beat, compact) {
                glossary_click = Some(entry);
            }
        }
        if open_glossary {
            self.open_glossary(None);
        } else if let Some(entry) = glossary_click {
            self.open_glossary(Some(entry.term));
        }

        if count_in_dirty {
            // One change moves the timeline, the rendered count-in, and the
            // transport state together. It may defer itself to the loop start
            // when the count-in already playing must not be cut short.
            self.apply_count_in_change();
        } else {
            if timeline_dirty && !(music_dirty && self.audio.is_playing()) {
                self.rebuild_timeline();
            }
            if music_dirty {
                // Re-render on the next frame after the control is released;
                // the old buffer keeps playing until then so playback does not
                // restart.
                self.needs_render = true;
                self.pending_audio = true;
            }
        }
        match action {
            Some("toggle") => self.toggle_playback(),
            Some("restart") => self.restart(),
            Some("end") | Some("cancel-ending") => self.end_tune(),
            _ => {}
        }
        if rotation_changed {
            self.rotation_settings_changed();
        }
        if save || timeline_dirty || music_dirty || rotation_changed {
            self.save_prefs();
        }
    }

    /// The dance, tune, and tune-set summary above the beat meter.
    ///
    /// Returns true when the tune-rotation interval changed, so the caller can
    /// discard work prepared for the old interval.
    fn session_summary(&mut self, ui: &mut egui::Ui, compact: bool) -> bool {
        let mut rotation_changed = false;
        if compact {
            ui.label(egui::RichText::new(&self.dance.title).size(22.0).strong());
        } else {
            ui.heading(&self.dance.title);
        }
        ui.label(
            egui::RichText::new(format!(
                "{} · {} · {} beats",
                empty_label(&self.dance.choreographer, "unknown choreographer"),
                empty_label(&self.dance.formation, "formation unknown"),
                self.dance.total_beats()
            ))
            .color(theme::muted(ui)),
        );
        let instruments = instrument_summary(&self.prefs);
        if compact {
            // Two deliberate lines instead of one wrapped line, so the tune
            // and the instrument summary never break mid-phrase at phone width.
            ui.label(
                egui::RichText::new(format!(
                    "Tune: {} · {} · {}",
                    self.tune.title,
                    self.tune.display_key(),
                    self.tune.display_rhythm()
                ))
                .strong(),
            );
            ui.label(egui::RichText::new(&instruments).color(theme::muted(ui)));
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label("Tune:");
                ui.label(
                    egui::RichText::new(format!(
                        "{} · {} · {}",
                        self.tune.title,
                        self.tune.display_key(),
                        self.tune.display_rhythm()
                    ))
                    .strong(),
                );
                ui.label(egui::RichText::new(&instruments).color(theme::muted(ui)));
            });
        }

        if self.prefs.tune_rotation.enabled {
            let tunes = self.rotation_tunes();
            let position = self.rotation_position();
            let next = self.next_rotation_tune().map(|tune| tune.title);
            let interval = self.prefs.tune_rotation.passes_per_tune;
            let minutes = self
                .prefs
                .tune_rotation
                .seconds_per_tune(self.prefs.tempo_bpm)
                / 60.0;
            let summary = if tunes.len() < 2 {
                format!(
                    "Tune set: add at least two saved tunes in Select tune ({} now).",
                    tunes.len()
                )
            } else {
                let place = match position {
                    Some(index) => format!("Tune {} of {}", index + 1, tunes.len()),
                    None => "Current tune is not in the set".to_string(),
                };
                let next = next.unwrap_or_else(|| "the first tune".to_string());
                format!(
                    "{place} · next: {next} · changes every {interval} times through (≈{minutes:.1} min)"
                )
            };
            let mut edit = false;
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(summary).small().color(theme::muted(ui)));
                if ui.button("Edit tune set").clicked() {
                    edit = true;
                }
            });
            if tunes.len() >= 2 {
                // On a phone the label goes above the rail: a one-row label,
                // slider, value, and "times through" is wider than 360 points.
                let response = slider_row(
                    ui,
                    compact,
                    "Change tune every",
                    &mut self.prefs.tune_rotation.passes_per_tune,
                    |ui, passes| {
                        number_slider(
                            ui,
                            egui::Slider::new(
                                passes,
                                caller_core::TuneRotation::MIN_PASSES
                                    ..=caller_core::TuneRotation::MAX_PASSES,
                            )
                            .text("times through"),
                        )
                    },
                );
                if response.changed() {
                    rotation_changed = true;
                }
            }
            if edit {
                self.screen = Screen::TunePicker;
            }
        }
        if crate::platform::platform() == "web" {
            self.share_link_ui(ui, compact);
        }
        rotation_changed
    }

    /// The web-only share control.
    ///
    /// The link is generated on the press and kept in the panel instead of
    /// every frame: a tune set can carry a lot of ABC text, and the panel is
    /// closed whenever the dance, tune, or tune set changes.
    pub(in crate::app) fn share_link_ui(&mut self, ui: &mut egui::Ui, compact: bool) {
        ui.add_space(6.0);
        if self.share.is_none() {
            let button = if compact {
                ui.add_sized(
                    [ui.available_width().min(240.0), theme::TOUCH_TARGET],
                    egui::Button::new("Share this session"),
                )
            } else {
                ui.button("Share this session")
            };
            if button
                .on_hover_text("Copy a link that rebuilds this dance and tune set in the web app")
                .clicked()
            {
                self.open_share_link();
            }
            return;
        }

        let Some(panel) = self.share.as_ref() else {
            return;
        };
        let link = panel.link.clone();
        let message = panel.message.clone();
        let message_is_error = message.starts_with("Could not");
        let mut copy = false;
        let mut close = false;
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Share this session").strong());
                ui.label(
                    egui::RichText::new(
                        "The link carries the dance card and every tune, so it opens the same session with no download.",
                    )
                    .small()
                    .color(theme::muted(ui)),
                );
                egui::ScrollArea::horizontal()
                    .id_salt("share-link")
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&link).monospace().small())
                                .selectable(true)
                                .extend(),
                        );
                    });
                ui.horizontal_wrapped(|ui| {
                    let size = egui::vec2(
                        if compact { 132.0 } else { 96.0 },
                        if compact { theme::TOUCH_TARGET } else { 0.0 },
                    );
                    let button = |ui: &mut egui::Ui, text: &str| {
                        if compact {
                            ui.add_sized(size, egui::Button::new(text))
                        } else {
                            ui.button(text)
                        }
                    };
                    if button(ui, "Copy link")
                        .on_hover_text("Copy the share link to the clipboard")
                        .clicked()
                    {
                        copy = true;
                    }
                    if button(ui, "Close").clicked() {
                        close = true;
                    }
                });
                if !message.is_empty() {
                    let color = if message_is_error {
                        theme::error(ui)
                    } else {
                        theme::muted(ui)
                    };
                    ui.label(egui::RichText::new(&message).small().color(color));
                }
            });
        if copy {
            self.copy_share_link();
        }
        if close {
            self.share = None;
        }
    }

    /// The beat meter and its transport state.
    ///
    /// The count-in state and its remaining beats come from the installed
    /// timeline, so the meter can never advertise a count-in the music does not
    /// contain (or hide one that it does).
    fn session_meter(&mut self, ui: &mut egui::Ui, beat: f64) {
        ui.add_space(6.0);
        let progress = if beat < 0.0 {
            0.0
        } else {
            (beat / self.timeline.music_loop_beats).clamp(0.0, 1.0)
        };
        let transport = transport_state(
            self.started,
            self.audio.is_playing(),
            self.audio.is_finished(),
            self.audio.in_ending(),
            beat,
            self.timeline.count_in_beats,
        );
        let progress_text = progress_text(
            transport,
            beat,
            self.timeline.music_loop_beats,
            self.timeline.count_in_beats,
        );
        let progress_width = progress_bar_width(ui.available_width());
        ui.add(
            egui::ProgressBar::new(progress as f32)
                .text(progress_text)
                .desired_width(progress_width),
        );
    }

    /// The large call prompt, including the in-card preview of the next call.
    ///
    /// It sits at the top of the session, directly under the tab row, because
    /// it is the one panel a caller reads while the dance is running. The
    /// controls, tune summary, progress bar, and dance card follow below it.
    ///
    /// The next call is a footer row inside the same card rather than a line
    /// under it: a row that appears only while a call is being spoken moves
    /// every control below it, and mid-dance the caller is watching the card.
    /// Every state reserves the same rows (see [`prompt_card_min_height`]) so
    /// switching between CALL NOW, UPCOMING MOVE, and the final figure never
    /// resizes the panel.
    fn prompt_card(
        &self,
        ui: &mut egui::Ui,
        beat: f64,
        compact: bool,
    ) -> Option<&'static caller_core::glossary::GlossaryEntry> {
        let active = if self.started {
            self.timeline
                .scheduled_call_at(beat, self.prefs.loop_enabled)
        } else {
            None
        };
        let next = if self.started {
            self.timeline
                .next_scheduled_call(beat, self.prefs.loop_enabled)
        } else {
            None
        };
        let phase = active.as_ref().map(|call| phase_at(call, beat));
        let card_mode = card_state(phase, next.is_some());
        let card_fill = match card_tone(self.started, phase, card_mode) {
            CardTone::CallNow => theme::call_now_bg(ui),
            CardTone::UpcomingMove => theme::upcoming_move_bg(ui),
            CardTone::Neutral => theme::call_active_bg(ui),
        };

        let call_size = if compact { 30.0 } else { 34.0 };
        let count_size = if compact { 48.0 } else { 56.0 };
        let count_in_beats = self.timeline.count_in_beats;
        let count_in_label = count_in_label(count_in_beats);
        // Only a card that is naming a call previews the one after it; the
        // ready, count-in-without-a-call, ending, and finished cards have
        // nothing to look ahead to.
        let footer = if active.is_some() {
            card_footer(card_mode, next.as_ref())
        } else {
            None
        };
        // While a run is live the card reserves the same rows in every state,
        // so the count-in, CALL NOW, UPCOMING MOVE, and the slow ending all keep
        // the controls below in place. The idle READY card and the completed
        // card are left content-sized: reserving two call lines there would be
        // half an empty box, and nothing is moving.
        let min_height = if self.started && !self.audio.is_finished() {
            let call_row = ui
                .ctx()
                .fonts_mut(|fonts| fonts.row_height(&egui::FontId::proportional(call_size)));
            let small_row = ui.text_style_height(&egui::TextStyle::Small);
            let body_row = ui.text_style_height(&egui::TextStyle::Body);
            Some(prompt_card_min_height(
                call_row,
                small_row,
                body_row,
                ui.spacing().item_spacing.y,
                card_mode.card_rows(),
            ))
        } else {
            None
        };
        let mut clicked = None;
        egui::Frame::group(ui.style())
            .fill(card_fill)
            .inner_margin(egui::Margin::same(if compact { 10 } else { 12 }))
            .show(ui, |ui| {
                if compact {
                    // A content-sized prompt card leaves a ragged strip of
                    // background on a phone; use the full width for the text.
                    ui.set_width(ui.available_width());
                }
                // A minimum, never a clip: a call that wraps past the reserved
                // rows grows the card instead of cutting the words off.
                let content_top = ui.cursor().top();
                // The card holds text only. Undo egui's assumption that a
                // horizontal row is a 40 pt (or 44 pt touch) control: on a
                // text row that padding spreads the label, the call, and the
                // footer far apart and inflates the reserved height.
                ui.spacing_mut().interact_size.y = 0.0;
                if !self.started {
                    ui.label(egui::RichText::new("READY").small().color(theme::muted(ui)));
                    ui.label(
                        egui::RichText::new("Press Play to begin")
                            .size(30.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} calls · {} beats · {}",
                            self.timeline.calls.len(),
                            self.timeline.music_loop_beats,
                            count_in_label
                        ))
                        .color(theme::muted(ui)),
                    );
                } else if count_in_beats > 0.0 && beat < -0.001 {
                    ui.label(
                        egui::RichText::new("COUNT-IN")
                            .small()
                            .color(theme::muted(ui)),
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(
                                count_in_beats_left(beat, count_in_beats).to_string(),
                            )
                            .size(count_size)
                            .strong(),
                        );
                        ui.vertical(|ui| {
                            if let Some(call) = &active {
                                ui.label(
                                    egui::RichText::new("CALL NOW")
                                        .small()
                                        .color(theme::on_card(ui)),
                                );
                                let call_color = theme::on_card(ui);
                                clicked = glossary_text(
                                    ui,
                                    &call.call.text,
                                    self.prefs.glossary_enabled,
                                    |text| {
                                        egui::RichText::new(text)
                                            .size(26.0)
                                            .strong()
                                            .color(call_color)
                                    },
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("Listen for the first chord").size(20.0),
                                );
                                if let Some(next) = &next {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            egui::RichText::new("First call:")
                                                .color(theme::muted(ui)),
                                        );
                                        let next_color = theme::muted(ui);
                                        clicked = glossary_text(
                                            ui,
                                            &next.call.text,
                                            self.prefs.glossary_enabled,
                                            |text| egui::RichText::new(text).color(next_color),
                                        );
                                    });
                                }
                            }
                        });
                    });
                } else if let Some(call) = &active {
                    match card_mode {
                        CardMode::UpcomingMove => {
                            if let Some(upcoming) = &next {
                                clicked = upcoming_move_card(
                                    ui,
                                    upcoming,
                                    beat,
                                    self.prefs.show_original,
                                    self.prefs.glossary_enabled,
                                    call_size,
                                );
                            }
                        }
                        CardMode::FinalMove => {
                            ui.label(
                                egui::RichText::new("DANCING")
                                    .small()
                                    .color(theme::muted(ui)),
                            );
                            let call_color = theme::muted(ui);
                            clicked = glossary_text(
                                ui,
                                &call.call.text,
                                self.prefs.glossary_enabled,
                                |text| {
                                    egui::RichText::new(text)
                                        .size(call_size)
                                        .strong()
                                        .color(call_color)
                                },
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new("final figure").color(theme::muted(ui)),
                                );
                                if self.prefs.show_original {
                                    ui.label(
                                        egui::RichText::new("·").small().color(theme::muted(ui)),
                                    );
                                    let original_color = theme::muted(ui);
                                    clicked = glossary_text(
                                        ui,
                                        &call.call.original_text,
                                        self.prefs.glossary_enabled,
                                        |text| {
                                            egui::RichText::new(text).small().color(original_color)
                                        },
                                    );
                                }
                            });
                        }
                        CardMode::CurrentCall => {
                            let phase = phase_at(call, beat);
                            let label = match phase {
                                CallPhase::Upcoming => "UPCOMING",
                                CallPhase::Calling => "CALL NOW",
                                CallPhase::Executing => "DANCING",
                            };
                            ui.label(egui::RichText::new(label).small().color(
                                if phase == CallPhase::Calling {
                                    theme::on_card(ui)
                                } else {
                                    theme::muted(ui)
                                },
                            ));
                            let call_color = if phase == CallPhase::Executing {
                                theme::muted(ui)
                            } else {
                                theme::on_card(ui)
                            };
                            clicked = glossary_text(
                                ui,
                                &call.call.text,
                                self.prefs.glossary_enabled,
                                |text| {
                                    egui::RichText::new(text)
                                        .size(call_size)
                                        .strong()
                                        .color(call_color)
                                },
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(executes_in_text(
                                        phase,
                                        call.execute_beat,
                                        beat,
                                    ))
                                    .color(theme::muted(ui)),
                                );
                                if self.prefs.show_original {
                                    ui.label(
                                        egui::RichText::new("·").small().color(theme::muted(ui)),
                                    );
                                    let original_color = theme::muted(ui);
                                    clicked = glossary_text(
                                        ui,
                                        &call.call.original_text,
                                        self.prefs.glossary_enabled,
                                        |text| {
                                            egui::RichText::new(text).small().color(original_color)
                                        },
                                    );
                                }
                            });
                        }
                    }
                } else if self.audio.is_finished() {
                    ui.label(egui::RichText::new("Dance complete").size(28.0).strong());
                    if self.audio.ending_requested() {
                        ui.label("The tune finished on a held last note.");
                    }
                    ui.label("Press Restart to run it again.");
                } else if self.audio.in_ending() {
                    ui.label(
                        egui::RichText::new("SLOW FINISH")
                            .small()
                            .color(theme::muted(ui)),
                    );
                    ui.label(egui::RichText::new(&self.tune.title).size(28.0).strong());
                    ui.label(
                        egui::RichText::new("Slowing down and holding the last note…")
                            .color(theme::muted(ui)),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("Press Play to begin")
                            .size(28.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} calls · {} beats · {}",
                            self.timeline.calls.len(),
                            self.timeline.music_loop_beats,
                            count_in_label
                        ))
                        .color(theme::muted(ui)),
                    );
                }
                // The last row of the same card: the call to say after this
                // one. When there is no following call the row stays empty,
                // and the reserved height keeps the card from shrinking.
                if let Some(next) = footer {
                    clicked = prompt_card_footer(ui, next, compact, self.prefs.glossary_enabled);
                }
                // Pad inside the card instead of setting a minimum on the
                // content ui: the blank space belongs under the last row, not
                // between the rows, and short content still leaves the card at
                // its reserved height while long content grows it.
                let used = ui.cursor().top() - content_top;
                if let Some(min_height) = min_height
                    && used < min_height
                {
                    ui.add_space(min_height - used);
                }
            });
        clicked
    }

    /// The dance card figures with their call text and beat counts.
    ///
    /// `full_width` makes each figure card span the panel on a phone, where a
    /// content-sized card leaves ragged edges and narrows the call text.
    fn dance_card_list(
        &self,
        ui: &mut egui::Ui,
        beat: f64,
        full_width: bool,
    ) -> Option<&'static caller_core::glossary::GlossaryEntry> {
        let animate = self.prefs.animations_enabled;
        // The padding setting tightens both the box around each figure and the
        // gap between its lines, so a compact card fits the whole dance on
        // screen. Sparse is the room the card had before the setting.
        let padding = self.prefs.card_padding;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = padding.gap();
            if padding == CardPadding::Compact {
                // A card row is text, not a 40-point control, but
                // `horizontal_wrapped` reserves the ambient control height for
                // its phrase row. That one assumption doubled every row; the
                // prompt card already opts out of it the same way.
                ui.spacing_mut().interact_size.y = 0.0;
            }
            self.dance_card_figures(ui, beat, full_width, padding, animate)
        })
        .inner
    }

    /// Draw one padded box per figure inside [`Self::dance_card_list`].
    fn dance_card_figures(
        &self,
        ui: &mut egui::Ui,
        beat: f64,
        full_width: bool,
        padding: CardPadding,
        animate: bool,
    ) -> Option<&'static caller_core::glossary::GlossaryEntry> {
        let mut cursor = 0.0;
        let mut clicked = None;
        for (figure_index, figure) in self.dance.figures.iter().enumerate() {
            let figure_end = cursor + figure.beats;
            let active_figure = beat >= cursor && beat < figure_end;
            let active_t = animate_amount(
                ui.ctx(),
                ("figure-active", figure_index),
                active_figure,
                animate,
            );
            let calls: Vec<(usize, caller_core::CallEvent)> = self
                .timeline
                .calls
                .iter()
                .enumerate()
                .filter(|(_, call)| call.figure_index == figure_index)
                .map(|(index, call)| (index, call.clone()))
                .collect();
            // The figure card's top-right corner carries the lead for its
            // calls. A split figure has one lead per sub-call, in order.
            let call_lead = (!calls.is_empty()).then(|| {
                let leads = calls
                    .iter()
                    .map(|(_, call)| format_half_beats(call.lead_beats()))
                    .collect::<Vec<_>>()
                    .join(" · ");
                format!("call {leads} beats")
            });
            let fill = lerp_color(
                theme::call_upcoming_bg(ui),
                theme::call_active_bg(ui),
                active_t,
            );
            egui::Frame::group(ui.style())
                .fill(fill)
                .inner_margin(egui::Margin::same(padding.padding() as i8))
                .show(ui, |ui| {
                    if full_width {
                        ui.set_width(ui.available_width());
                    }
                    ui.horizontal(|ui| {
                        if let Some(phrase) = &figure.phrase {
                            ui.label(egui::RichText::new(phrase).strong().color(
                                if active_figure {
                                    theme::on_card(ui)
                                } else {
                                    theme::muted(ui)
                                },
                            ));
                        }
                        ui.label(
                            egui::RichText::new(format!("{} beats", figure.beats))
                                .small()
                                .color(theme::muted(ui)),
                        );
                        if let Some(call_lead) = &call_lead {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(call_lead)
                                            .small()
                                            .color(theme::subtle(ui)),
                                    );
                                },
                            );
                        }
                    });
                    for (index, call) in &calls {
                        let phase = self
                            .timeline
                            .scheduled_call_for_index(*index, beat, self.prefs.loop_enabled)
                            .as_ref()
                            .map(|scheduled| phase_at(scheduled, beat))
                            .unwrap_or(CallPhase::Upcoming);
                        let color = match phase {
                            CallPhase::Upcoming => theme::muted(ui),
                            CallPhase::Calling => theme::on_card(ui),
                            CallPhase::Executing => theme::executing(ui),
                        };
                        let size = if active_figure { 19.0 } else { 16.0 };
                        if let Some(entry) =
                            glossary_text(ui, &call.text, self.prefs.glossary_enabled, |text| {
                                egui::RichText::new(text).size(size).color(color)
                            })
                        {
                            clicked = Some(entry);
                        }
                        if self.prefs.show_original && call.original_text != call.text {
                            let original_color = theme::subtle(ui);
                            if let Some(entry) = glossary_text(
                                ui,
                                &call.original_text,
                                self.prefs.glossary_enabled,
                                |text| egui::RichText::new(text).small().color(original_color),
                            ) {
                                clicked = Some(entry);
                            }
                        }
                    }
                });
            cursor = figure_end;
        }
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tune_summary_names_every_band_row() {
        let prefs = Preferences::default();
        assert_eq!(
            instrument_summary(&prefs),
            "Melody: Fiddle ×2 · Accompaniment: Piano · Countermelody: Fiddle ×2 · \
             Percussion: Foot taps ×3"
        );

        let mut prefs = prefs;
        prefs.band = vec![
            BandSlot {
                part: BandPart::Melody,
                voice: BandVoice::Clarinet,
                players: 3,
                volume: 0.7,
                pan: -0.2,
                depth: 0.0,
            },
            BandSlot {
                part: BandPart::Accompaniment,
                voice: BandVoice::Guitar,
                players: 1,
                volume: 0.6,
                pan: 0.2,
                depth: 0.0,
            },
        ];
        assert_eq!(
            instrument_summary(&prefs),
            "Melody: Clarinet ×3 · Accompaniment: Guitar"
        );

        // Removing every pitched row would be repaired by validation, so the
        // summary always names something playable.
        prefs.band.retain(|slot| slot.is_pitched());
        prefs.validate();
        assert!(!instrument_summary(&prefs).is_empty());
    }

    /// One scheduled call, for the card-footer tests.
    fn scheduled(
        index: usize,
        text: &str,
        start_beat: f64,
        execute_beat: f64,
    ) -> caller_core::ScheduledCall {
        caller_core::ScheduledCall {
            index,
            call: caller_core::CallEvent {
                figure_index: index,
                phrase: None,
                call_start_beat: start_beat,
                execute_beat,
                end_beat: execute_beat + 8.0,
                text: text.to_string(),
                original_text: text.to_string(),
                syllables: text.split_whitespace().count(),
                words: text.split_whitespace().count(),
            },
            start_beat,
            execute_beat,
            end_beat: execute_beat + 8.0,
            wrapped: false,
        }
    }

    #[test]
    fn call_now_and_upcoming_move_reserve_the_same_card_height() {
        // Both cards show a different call, but the reserved rows come from the
        // font and the fixed row budget, so the panel cannot resize when the
        // mode changes.
        let (call_row, small_row, body_row, gap) = (40.0, 15.0, 19.0, 8.0);
        let call_now = prompt_card_min_height(
            call_row,
            small_row,
            body_row,
            gap,
            CardMode::CurrentCall.card_rows(),
        );
        let upcoming_move = prompt_card_min_height(
            call_row,
            small_row,
            body_row,
            gap,
            CardMode::UpcomingMove.card_rows(),
        );
        assert_eq!(call_now, upcoming_move);
        // The last figure reserves the same rows too, so the end of the dance
        // does not resize the session either.
        assert_eq!(
            call_now,
            prompt_card_min_height(
                call_row,
                small_row,
                body_row,
                gap,
                CardMode::FinalMove.card_rows(),
            )
        );
    }

    #[test]
    fn the_card_height_is_built_from_the_font_and_the_row_budget() {
        let rows = CardMode::CurrentCall.card_rows();
        let height = prompt_card_min_height(40.0, 15.0, 19.0, 8.0, rows);
        // One label row, two body rows (detail and footer), two call lines, and
        // the three gaps between those four widgets.
        assert_eq!(height, 15.0 + 2.0 * 19.0 + 2.0 * 40.0 + 3.0 * 8.0);
        // A bigger call font reserves more height, and never a negative one.
        assert!(prompt_card_min_height(56.0, 15.0, 19.0, 8.0, rows) > height);
        assert!(prompt_card_min_height(0.0, 0.0, 0.0, 0.0, rows) >= 0.0);
    }

    #[test]
    fn the_reserved_height_leaves_room_for_a_wrapping_call() {
        // The reservation must cover a second line: a call that wraps would
        // otherwise resize the card, which is the jump this fixes.
        let rows = CardMode::CurrentCall.card_rows();
        let one_line = prompt_card_min_height(40.0, 15.0, 19.0, 8.0, rows);
        let single_line_budget = prompt_card_min_height(
            40.0,
            15.0,
            19.0,
            8.0,
            CardRows {
                call: 1.0,
                label: 1.0,
                body: 2.0,
            },
        );
        assert!(one_line >= single_line_budget + 40.0);
    }

    #[test]
    fn the_card_footer_never_repeats_the_call_on_the_card() {
        let next = scheduled(1, "swing", 3.0, 6.0);
        // CALL NOW fills the footer with the following call.
        assert!(card_footer(CardMode::CurrentCall, Some(&next)).is_some());
        // UPCOMING MOVE already names that call in its main line.
        assert!(card_footer(CardMode::UpcomingMove, Some(&next)).is_none());
        // No following call: no footer, not an echo of the current one.
        assert!(card_footer(CardMode::CurrentCall, None).is_none());
        assert!(card_footer(CardMode::FinalMove, None).is_none());
    }

    #[test]
    fn transport_state_names_every_session_state() {
        assert_eq!(
            transport_state(false, false, false, false, 0.0, 4.0),
            TransportState::Ready
        );
        assert_eq!(
            transport_state(true, true, false, false, -2.0, 4.0),
            TransportState::CountIn
        );
        assert_eq!(
            transport_state(true, true, false, false, 4.0, 4.0),
            TransportState::Playing
        );
        assert_eq!(
            transport_state(true, false, false, false, 4.0, 4.0),
            TransportState::Paused
        );
        assert_eq!(
            transport_state(true, true, false, true, 62.0, 4.0),
            TransportState::Ending
        );
        assert_eq!(
            transport_state(true, false, true, false, 64.0, 4.0),
            TransportState::Finished
        );
    }

    #[test]
    fn a_paused_run_is_not_hidden_by_the_count_in_or_a_pending_ending() {
        // Pausing in the count-in must read as Paused, or a test cannot tell a
        // stopped transport from one that is waiting for the first beat.
        assert_eq!(
            transport_state(true, false, false, false, -1.0, 4.0),
            TransportState::Paused
        );
        // A finished run stays Finished even if the ending flag lingers.
        assert_eq!(
            transport_state(true, false, true, true, 64.0, 4.0),
            TransportState::Finished
        );
    }

    #[test]
    fn progress_bar_width_fills_the_panel_and_is_always_finite() {
        // The bar must end exactly at the content edge, so both rounded caps
        // are drawn inside the panel at any width.
        assert_eq!(progress_bar_width(366.0), 366.0);
        assert_eq!(progress_bar_width(336.0), 336.0);
        // An infinite or NaN available width must not reach
        // `ProgressBar::desired_width`: that is what put the right cap
        // off-screen and widened the scroll content.
        assert_eq!(progress_bar_width(f32::INFINITY), 0.0);
        assert_eq!(progress_bar_width(f32::NEG_INFINITY), 0.0);
        assert_eq!(progress_bar_width(f32::NAN), 0.0);
        assert_eq!(progress_bar_width(-24.0), 0.0);
    }

    #[test]
    fn progress_text_explains_the_state_and_position() {
        assert_eq!(
            progress_text(TransportState::Ready, 0.0, 64.0, 4.0),
            "Ready · beat 0 / 64"
        );
        assert_eq!(
            progress_text(TransportState::Playing, 12.4, 64.0, 4.0),
            "Playing · beat 12 / 64"
        );
        assert_eq!(
            progress_text(TransportState::Paused, 12.6, 64.0, 4.0),
            "Paused · beat 13 / 64"
        );
        assert_eq!(
            progress_text(TransportState::Finished, 64.0, 64.0, 4.0),
            "Finished · beat 64 / 64"
        );
        assert_eq!(
            progress_text(TransportState::Ending, 62.0, 64.0, 4.0),
            "Ending · slow finish"
        );
    }

    #[test]
    fn executes_in_text_rounds_to_the_nearest_half_beat() {
        assert_eq!(
            executes_in_text(CallPhase::Upcoming, 8.0, 4.0),
            "executes in 4.0 beats"
        );
        assert_eq!(
            executes_in_text(CallPhase::Calling, 8.0, 4.4),
            "executes in 3.5 beats"
        );
        assert_eq!(
            executes_in_text(CallPhase::Upcoming, 8.0, 4.75),
            "executes in 3.5 beats"
        );
        assert_eq!(
            executes_in_text(CallPhase::Executing, 8.0, 4.0),
            "figure in progress"
        );
        // A clock that has reached the execute beat shows zero, never a
        // negative countdown.
        assert_eq!(
            executes_in_text(CallPhase::Upcoming, 8.0, 9.0),
            "executes in 0.0 beats"
        );
    }

    #[test]
    fn count_in_text_counts_the_remaining_beats() {
        assert_eq!(
            progress_text(TransportState::CountIn, -4.0, 64.0, 4.0),
            "Count-in · 4 beats left"
        );
        // The audio clock can land just past the first beat; the display must
        // still start at four, not five.
        assert_eq!(
            progress_text(TransportState::CountIn, -4.000_001, 64.0, 4.0),
            "Count-in · 4 beats left"
        );
        assert_eq!(
            progress_text(TransportState::CountIn, -3.2, 64.0, 4.0),
            "Count-in · 4 beats left"
        );
        assert_eq!(
            progress_text(TransportState::CountIn, -0.2, 64.0, 4.0),
            "Count-in · 1 beat left"
        );
    }

    #[test]
    fn a_disabled_count_in_never_reports_count_in_state_or_text() {
        // The state comes from the installed timeline, so no count-in means no
        // COUNT-IN card even if a stale audio position is negative.
        assert_eq!(
            transport_state(true, true, false, false, -1.0, 0.0),
            TransportState::Playing
        );
        assert_eq!(count_in_beats_left(-1.0, 0.0), 0);
        assert_eq!(count_in_label(0.0), "no count-in");
        assert_eq!(count_in_label(4.0), "4-beat count-in");
    }

    #[test]
    fn the_painted_play_pause_icon_fits_its_button() {
        for side in [theme::TOUCH_TARGET, 40.0] {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(side, side));
            let play = play_pause_shapes(rect, false, egui::Color32::WHITE);
            let pause = play_pause_shapes(rect, true, egui::Color32::WHITE);
            assert_eq!(play.len(), 1, "play is one filled triangle");
            assert_eq!(pause.len(), 2, "pause is two bars");
            for shape in play.iter().chain(&pause) {
                let bounds = shape.visual_bounding_rect();
                assert!(
                    rect.contains_rect(bounds),
                    "{bounds:?} escapes a {side}-point button"
                );
            }
            // Upright bars read as pause even at a glance; a wide pair would
            // look like a broken stop icon.
            let bar = pause[0].visual_bounding_rect();
            assert!(bar.height() > bar.width(), "pause bars stay upright");
        }
    }

    #[test]
    fn the_tempo_slider_range_includes_the_current_tempo() {
        // The configured window is used as-is when the current tempo is inside.
        assert_eq!(tempo_slider_bounds(200.0, 300.0, 240.0), (200.0, 300.0));
        // A slower or faster tempo stays selectable by widening, not snapping.
        assert_eq!(tempo_slider_bounds(200.0, 300.0, 180.0), (180.0, 300.0));
        assert_eq!(tempo_slider_bounds(200.0, 300.0, 320.0), (200.0, 320.0));
        // An inverted pair is read as the same window.
        assert_eq!(tempo_slider_bounds(300.0, 200.0, 250.0), (200.0, 300.0));
        // The helpers also keep the global limits.
        assert_eq!(tempo_slider_bounds(50.0, 400.0, 240.0), (120.0, 320.0));
    }
}
