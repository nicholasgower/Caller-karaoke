//! Session-tab state and input for the MCP server.
//!
//! The generic egui tools can click anything, but testing the session tab needs
//! two things they do not provide: readable toggle state (egui-mcp's widget
//! projection drops AccessKit's `toggled` flag) and a way to set a slider to a
//! value (a click on the rail does nothing; only a drag from the handle moves
//! it). This module reads the session controls straight from the accessibility
//! tree and drives them with the same synthetic input a user produces.
//!
//! Everything is located by accessible name, so the desktop and phone layouts
//! (which use different labels for the transport buttons and tabs) are both
//! supported:
//!
//! | Control | Name |
//! | --- | --- |
//! | Tempo slider | `Tempo` |
//! | Volume slider | `Volume` |
//! | Transport buttons | `Play`, `Pause`, `Restart`, `End tune`/`End`, `Cancel ending`/`Cancel` |
//! | Toggles | `Loop`, `Cycle tunes`, `Count-in`, `Speak calls` |
//! | Preferences gear | `Preferences` (a header button, not a tab) |
//! | Transport readout | the progress bar text, e.g. `Playing · beat 12 / 64` |

use std::time::Duration;

use accesskit::{Role, Toggled};
use accesskit_consumer::{NodeRef, Tree};
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_mcp::Bridge;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The tempo and volume slider ranges, matching the session controls.
pub const TEMPO_RANGE: (f64, f64) = (120.0, 320.0);
pub const VOLUME_RANGE: (f64, f64) = (0.0, 1.0);

/// How far a slider may land from a requested value. A 100-point rail quantises
/// a drag to a pixel, so one tempo step is about 2 BPM and one volume step 0.01.
const TEMPO_TOLERANCE: f64 = 2.0;
const VOLUME_TOLERANCE: f64 = 0.02;

/// The transport state the session progress bar names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransportState {
    Ready,
    CountIn,
    Playing,
    Paused,
    Ending,
    Finished,
}

impl TransportState {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "Ready" => Some(Self::Ready),
            "Count-in" => Some(Self::CountIn),
            "Playing" => Some(Self::Playing),
            "Paused" => Some(Self::Paused),
            "Ending" => Some(Self::Ending),
            "Finished" => Some(Self::Finished),
            _ => None,
        }
    }
}

/// A snapshot of the session tab, readable after any input.
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
pub struct SessionState {
    /// The tab in front: `session`, `dance`, `tune`, or `preferences`.
    pub screen: Option<String>,

    /// `ready`, `count_in`, `playing`, `paused`, `ending`, or `finished`.
    pub transport: Option<TransportState>,

    /// The beat the progress bar displays. Whole beats: the bar rounds.
    pub beat: Option<f64>,

    /// Beats left before the loop starts, while the count-in runs.
    pub count_in_beats_left: Option<u32>,

    /// The tempo in BPM, as the session slider reports it.
    pub tempo_bpm: Option<f64>,

    /// The master volume, 0–1.
    pub volume: Option<f64>,

    /// The **Loop** toggle.
    pub loop_enabled: Option<bool>,

    /// The **Cycle tunes** toggle.
    pub cycle_tunes: Option<bool>,

    /// The **Count-in** toggle.
    pub count_in: Option<bool>,

    /// The **Speak calls** toggle.
    pub speak_calls: Option<bool>,
}

impl SessionState {
    /// Read the session controls from one accessibility-tree snapshot.
    ///
    /// Fields that are absent (for example the sliders while another tab is in
    /// front) stay `None` rather than guessing.
    pub fn read(snap: &egui_mcp::TreeSnapshot) -> Result<Self, String> {
        let tree = snap.tree.as_ref().ok_or("no accessibility tree yet")?;
        let mut state = Self::default();
        let mut tabs = Vec::new();

        visit(tree.state().root(), &mut |node| {
            if node.is_hidden() {
                return;
            }
            let label = node.label().unwrap_or_default();
            match node.role() {
                Role::ProgressIndicator => {
                    if let Some((transport, beat, count_in)) = parse_progress(&label) {
                        state.transport = Some(transport);
                        state.beat = beat;
                        state.count_in_beats_left = count_in;
                    }
                }
                Role::Slider => {
                    if label.eq_ignore_ascii_case("Tempo") {
                        state.tempo_bpm = node.numeric_value().map(round_to_thousandths);
                    } else if label.eq_ignore_ascii_case("Volume") {
                        state.volume = node.numeric_value().map(round_to_thousandths);
                    }
                }
                Role::CheckBox => match label.as_str() {
                    "Loop" => state.loop_enabled = is_on(&node),
                    "Cycle tunes" => state.cycle_tunes = is_on(&node),
                    "Count-in" => state.count_in = is_on(&node),
                    "Speak calls" => state.speak_calls = is_on(&node),
                    _ => {}
                },
                Role::Button => {
                    if is_on(&node) == Some(true)
                        && let Some(screen) = screen_for_tab(&label)
                    {
                        tabs.push(screen);
                    }
                }
                _ => {}
            }
        });

        if let Some(screen) = tabs.first() {
            state.screen = Some((*screen).to_string());
        }
        Ok(state)
    }
}

/// A `Role::Button` is a tab when its label names a top-row screen.
fn screen_for_tab(label: &str) -> Option<&'static str> {
    match label {
        "Session" => Some("session"),
        "Select dance" | "Dance" => Some("dance"),
        "Select Tune" | "Tune" => Some("tune"),
        "Preferences" | "Prefs" => Some("preferences"),
        _ => None,
    }
}

fn is_on(node: &NodeRef) -> Option<bool> {
    match node.toggled()? {
        Toggled::True => Some(true),
        Toggled::False => Some(false),
        Toggled::Mixed => None,
    }
}

/// Slider values arrive as f32 widened to f64; three decimals is plenty for a
/// BPM or a 0–1 volume and keeps the JSON readable.
fn round_to_thousandths(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// Parse the progress-bar text written by the session screen.
///
/// The session tab names the state in front of the beat, for example
/// `Playing · beat 12 / 64`, so a paused run is visible in the tree even
/// though it sounds the same as a muted one.
pub fn parse_progress(text: &str) -> Option<(TransportState, Option<f64>, Option<u32>)> {
    let (name, rest) = text.split_once(" · ")?;
    let state = TransportState::parse(name)?;
    match state {
        TransportState::CountIn => {
            let left = rest.split_whitespace().next()?.parse().ok()?;
            Some((state, None, Some(left)))
        }
        TransportState::Ending => Some((state, None, None)),
        _ => {
            let beat = rest
                .strip_prefix("beat ")?
                .split(" /")
                .next()?
                .parse()
                .ok()?;
            Some((state, Some(beat), None))
        }
    }
}

fn visit<'a>(node: NodeRef<'a>, f: &mut impl FnMut(NodeRef<'a>)) {
    f(node);
    for child in node.children() {
        visit(child, f);
    }
}

/// A located control, reduced to plain data so the tree borrow ends before any
/// further inspection request.
#[derive(Debug, Clone, Copy)]
struct Found {
    center: Pos2,
    rect: egui::Rect,
    toggled: Option<bool>,
    value: Option<f64>,
    disabled: bool,
}

fn rect_of(node: &NodeRef, pixels_per_point: f32) -> Option<egui::Rect> {
    let bounds = node.bounding_box()?;
    let scale = if pixels_per_point > 0.0 {
        1.0 / f64::from(pixels_per_point)
    } else {
        1.0
    };
    #[expect(clippy::cast_possible_truncation)]
    let rect = egui::Rect::from_min_max(
        Pos2::new((bounds.x0 * scale) as f32, (bounds.y0 * scale) as f32),
        Pos2::new((bounds.x1 * scale) as f32, (bounds.y1 * scale) as f32),
    );
    rect.is_positive().then_some(rect)
}

/// Find exactly one visible control by role and accessible name.
fn find(
    tree: &Tree,
    pixels_per_point: f32,
    role: Role,
    description: &str,
    accepts: impl Fn(&str) -> bool,
) -> Result<Found, String> {
    let mut matches = Vec::new();
    visit(tree.state().root(), &mut |node| {
        if node.is_hidden() || node.role() != role {
            return;
        }
        let label = node.label().unwrap_or_default();
        if !accepts(&label) {
            return;
        }
        if let Some(rect) = rect_of(&node, pixels_per_point) {
            matches.push(Found {
                center: rect.center(),
                rect,
                toggled: is_on(&node),
                value: node.numeric_value(),
                disabled: node.is_disabled(),
            });
        }
    });

    match matches.len() {
        0 => Err(format!(
            "{description} is not on screen — switch to the Session tab first"
        )),
        1 => Ok(matches[0]),
        _ => Err(format!(
            "{description} matched {} controls; the session tab should have one",
            matches.len()
        )),
    }
}

fn fraction(value: f64, (min, max): (f64, f64)) -> f32 {
    let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
    #[expect(clippy::cast_possible_truncation)]
    {
        t as f32
    }
}

fn value_at(rect: egui::Rect, fraction: f32) -> Pos2 {
    // egui insets the rail by the handle radius (`height / 2.5`) at each end, so
    // the value runs between `left + radius` and `right - radius`, not the full
    // widget width.
    let radius = rect.height() / 2.5;
    let left = rect.left() + radius;
    let right = rect.right() - radius;
    Pos2::new(
        left + (right - left) * fraction.clamp(0.0, 1.0),
        rect.center().y,
    )
}

/// The length of a horizontal slider's value rail in points.
fn rail_length(rect: egui::Rect) -> f32 {
    (rect.width() - 2.0 * rect.height() / 2.5).max(1.0)
}

/// One session-tab action, mirroring the transport buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    Play,
    Pause,
    Restart,
    End,
    CancelEnding,
}

impl SessionAction {
    fn labels(self) -> &'static [&'static str] {
        match self {
            Self::Play => &["Play"],
            Self::Pause => &["Pause"],
            Self::Restart => &["Restart"],
            // The button flips to Cancel while an ending is pending, and reads
            // Ending… (disabled) while the ending plays.
            Self::End => &["End tune", "End", "Cancel ending", "Cancel", "Ending…"],
            Self::CancelEnding => &["Cancel ending", "Cancel"],
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Self::Play => "the Play button",
            Self::Pause => "the Pause button",
            Self::Restart => "the Restart button",
            Self::End => "the End tune button",
            Self::CancelEnding => "the Cancel ending button (no ending is pending)",
        }
    }
}

/// The slider a `session_set` request can move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionToggle {
    LoopEnabled,
    CycleTunes,
    CountIn,
    SpeakCalls,
}

impl SessionToggle {
    fn label(self) -> &'static str {
        match self {
            Self::LoopEnabled => "Loop",
            Self::CycleTunes => "Cycle tunes",
            Self::CountIn => "Count-in",
            Self::SpeakCalls => "Speak calls",
        }
    }
}

/// What `session_set` changed. Every field is optional; omitted fields are left
/// alone. The result names the value the app actually applied.
#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SessionSet {
    /// Tempo in BPM, 120–320. The slider snaps to a nearby value, so the
    /// result reports the tempo the app actually adopted.
    #[serde(default)]
    pub tempo_bpm: Option<f64>,

    /// Master volume, 0–1.
    #[serde(default)]
    pub volume: Option<f64>,

    /// **Loop**.
    #[serde(default)]
    pub loop_enabled: Option<bool>,

    /// **Cycle tunes**.
    #[serde(default)]
    pub cycle_tunes: Option<bool>,

    /// **Count-in**.
    #[serde(default)]
    pub count_in: Option<bool>,

    /// **Speak calls**.
    #[serde(default)]
    pub speak_calls: Option<bool>,
}

impl SessionSet {
    fn toggles(&self) -> [(SessionToggle, Option<bool>); 4] {
        [
            (SessionToggle::LoopEnabled, self.loop_enabled),
            (SessionToggle::CycleTunes, self.cycle_tunes),
            (SessionToggle::CountIn, self.count_in),
            (SessionToggle::SpeakCalls, self.speak_calls),
        ]
    }
}

/// A live session on the app's inspection connection.
///
/// The server opens a second inspection connection for this so it can read the
/// tree directly (for toggle state and slider values) while the generic tools
/// keep their own bridge. The app serves each inspection connection on its own
/// thread, so the second one is invisible to the UI.
pub struct SessionClient {
    bridge: Bridge,
}

impl SessionClient {
    pub fn new(bridge: Bridge) -> Self {
        Self { bridge }
    }

    /// Read the session tab's state.
    pub async fn state(&self) -> Result<SessionState, String> {
        let snap = self.bridge.fetch_tree().await?;
        SessionState::read(&snap)
    }

    /// Bring the session tab to the front if another screen is showing.
    pub async fn ensure_session(&self) -> Result<(), String> {
        let snap = self.bridge.fetch_tree().await?;
        let state = SessionState::read(&snap)?;
        if state.screen.as_deref() == Some("session") {
            return Ok(());
        }
        let tab = find(
            tree_of(&snap)?,
            snap.pixels_per_point,
            Role::Button,
            "the Session tab",
            |label| label == "Session",
        );
        let found = tab?;
        self.click(found.center).await
    }

    /// Click a transport button and return the state it produced.
    pub async fn action(&self, action: SessionAction) -> Result<SessionState, String> {
        self.ensure_session().await?;
        let snap = self.bridge.fetch_tree().await?;
        let found = find(
            tree_of(&snap)?,
            snap.pixels_per_point,
            Role::Button,
            action.describe(),
            |label| action.labels().contains(&label),
        )?;
        if found.disabled {
            return Err(format!(
                "{} is disabled right now — the music has not been prepared yet, or an \
                 ending is already playing",
                action.describe()
            ));
        }
        self.click(found.center).await?;
        self.state().await
    }

    /// Apply tempo, volume, and toggle changes, then report the resulting state.
    pub async fn apply(&self, set: &SessionSet) -> Result<SessionState, String> {
        self.ensure_session().await?;

        if let Some(tempo) = set.tempo_bpm {
            self.set_slider("Tempo", TEMPO_RANGE, tempo, TEMPO_TOLERANCE)
                .await?;
        }
        if let Some(volume) = set.volume {
            self.set_slider("Volume", VOLUME_RANGE, volume, VOLUME_TOLERANCE)
                .await?;
        }
        for (toggle, wanted) in set.toggles() {
            if let Some(wanted) = wanted {
                self.set_toggle(toggle, wanted).await?;
            }
        }
        self.state().await
    }

    /// Poll until the transport reaches a state or the beat crosses a bound.
    pub async fn wait(
        &self,
        transport: Option<TransportState>,
        min_beat: Option<f64>,
        max_beat: Option<f64>,
        timeout: Duration,
    ) -> Result<SessionState, String> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let state = self.state().await?;
            let transport_ok = transport.is_none_or(|wanted| state.transport == Some(wanted));
            let min_ok = min_beat.is_none_or(|min| state.beat.is_some_and(|beat| beat >= min));
            let max_ok = max_beat.is_none_or(|max| state.beat.is_some_and(|beat| beat <= max));
            if transport_ok && min_ok && max_ok {
                return Ok(state);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "session_wait timed out after {}s; last state: {state:?}",
                    timeout.as_secs(),
                ));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn set_toggle(&self, toggle: SessionToggle, wanted: bool) -> Result<(), String> {
        for _attempt in 0..3 {
            let snap = self.bridge.fetch_tree().await?;
            let found = find(
                tree_of(&snap)?,
                snap.pixels_per_point,
                Role::CheckBox,
                toggle.label(),
                |label| label == toggle.label(),
            )?;
            if found.toggled == Some(wanted) {
                return Ok(());
            }
            if found.disabled {
                return Err(format!("{} is disabled", toggle.label()));
            }
            self.click(found.center).await?;
            if self.toggle_is(toggle).await? == Some(wanted) {
                return Ok(());
            }
        }
        Err(format!(
            "could not turn {} {}",
            toggle.label(),
            if wanted { "on" } else { "off" }
        ))
    }

    async fn toggle_is(&self, toggle: SessionToggle) -> Result<Option<bool>, String> {
        let state = self.state().await?;
        Ok(match toggle {
            SessionToggle::LoopEnabled => state.loop_enabled,
            SessionToggle::CycleTunes => state.cycle_tunes,
            SessionToggle::CountIn => state.count_in,
            SessionToggle::SpeakCalls => state.speak_calls,
        })
    }

    async fn set_slider(
        &self,
        name: &str,
        (min, max): (f64, f64),
        wanted: f64,
        tolerance: f64,
    ) -> Result<(), String> {
        let wanted = wanted.clamp(min, max);
        let (mut current, mut rect) = self.slider_reading(name).await?;
        let start = current;
        let mut correction = 0.0f32;

        for _attempt in 0..6 {
            if (current - wanted).abs() <= tolerance {
                return Ok(());
            }
            let from = value_at(rect, fraction(current, (min, max)));
            let to = value_at(rect, fraction(wanted, (min, max)));
            let to = Pos2::new(to.x + correction, to.y);
            self.drag(from, to).await?;

            (current, rect) = self.slider_reading(name).await?;
            if (current - wanted).abs() <= tolerance {
                return Ok(());
            }
            // The pointer and the value are not perfectly proportional (the
            // rail is inset by the handle, and egui snaps to "aimed" values
            // within a few points), so nudge the target by the remaining
            // error rather than repeating the same drag.
            let per_point = (max - min) / f64::from(rail_length(rect));
            correction += ((wanted - current) / per_point) as f32;
        }

        // A slider that never moved is a failure; one that settled on a nearby
        // value was snapped by egui, and the caller reads the applied value.
        if (current - start).abs() > (max - min) / 200.0 {
            Ok(())
        } else {
            Err(format!(
                "could not move the {name} slider from {start}; it stayed at {current}"
            ))
        }
    }

    async fn slider_reading(&self, name: &str) -> Result<(f64, egui::Rect), String> {
        let snap = self.bridge.fetch_tree().await?;
        let found = find(
            tree_of(&snap)?,
            snap.pixels_per_point,
            Role::Slider,
            &format!("the {name} slider"),
            |label| label.eq_ignore_ascii_case(name),
        )?;
        let value = found
            .value
            .ok_or_else(|| format!("the {name} slider reports no numeric value"))?;
        Ok((value, found.rect))
    }

    async fn click(&self, pos: Pos2) -> Result<(), String> {
        self.bridge
            .apply_events(vec![
                Event::PointerMoved(pos),
                pointer_button(pos, true),
                pointer_button(pos, false),
            ])
            .await
    }

    async fn drag(&self, from: Pos2, to: Pos2) -> Result<(), String> {
        const STEPS: u32 = 8;
        self.bridge
            .apply_events(vec![Event::PointerMoved(from), pointer_button(from, true)])
            .await?;
        for step in 1..=STEPS {
            let t = step as f32 / STEPS as f32;
            let waypoint = Pos2::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            self.bridge
                .apply_events(vec![Event::PointerMoved(waypoint)])
                .await?;
        }
        self.bridge
            .apply_events(vec![pointer_button(to, false)])
            .await
    }
}

fn pointer_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn tree_of(snap: &egui_mcp::TreeSnapshot) -> Result<&Tree, String> {
    snap.tree
        .as_ref()
        .ok_or("no accessibility tree yet".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_progress_bar_names_the_transport_state() {
        assert_eq!(
            parse_progress("Ready · beat 0 / 64"),
            Some((TransportState::Ready, Some(0.0), None))
        );
        assert_eq!(
            parse_progress("Playing · beat 12 / 64"),
            Some((TransportState::Playing, Some(12.0), None))
        );
        assert_eq!(
            parse_progress("Paused · beat 64 / 64"),
            Some((TransportState::Paused, Some(64.0), None))
        );
        assert_eq!(
            parse_progress("Finished · beat 64 / 64"),
            Some((TransportState::Finished, Some(64.0), None))
        );
        assert_eq!(
            parse_progress("Count-in · 4 beats left"),
            Some((TransportState::CountIn, None, Some(4)))
        );
        assert_eq!(
            parse_progress("Count-in · 1 beat left"),
            Some((TransportState::CountIn, None, Some(1)))
        );
        assert_eq!(
            parse_progress("Ending · slow finish"),
            Some((TransportState::Ending, None, None))
        );
    }

    #[test]
    fn an_unrelated_label_is_not_mistaken_for_transport() {
        assert_eq!(parse_progress("Beat 12 / 64"), None);
        assert_eq!(parse_progress("Playing"), None);
        assert_eq!(parse_progress("Loading · 50%"), None);
    }

    #[test]
    fn slider_positions_map_the_range_onto_the_rail() {
        let rect = egui::Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(100.0, 40.0));
        // The handle radius is height / 2.5 = 16, so the rail runs 26..94.
        assert!((rail_length(rect) - 68.0).abs() < 1e-4);
        assert!((value_at(rect, fraction(120.0, TEMPO_RANGE)).x - 26.0).abs() < 1e-4);
        assert!((value_at(rect, fraction(320.0, TEMPO_RANGE)).x - 94.0).abs() < 1e-4);
        assert!((value_at(rect, fraction(220.0, TEMPO_RANGE)).x - 60.0).abs() < 1e-4);
        // Values outside the range pin to the ends rather than leaving the rail.
        assert!((value_at(rect, fraction(0.0, TEMPO_RANGE)).x - 26.0).abs() < 1e-4);
        assert!((value_at(rect, fraction(400.0, TEMPO_RANGE)).x - 94.0).abs() < 1e-4);
    }

    #[test]
    fn volume_uses_the_same_rail_maths() {
        let rect = egui::Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(100.0, 20.0));
        // height / 2.5 = 8, so the rail runs 8..92 and its middle is still 50.
        assert!((value_at(rect, fraction(0.5, VOLUME_RANGE)).x - 50.0).abs() < 1e-4);
        assert!((value_at(rect, fraction(0.0, VOLUME_RANGE)).x - 8.0).abs() < 1e-4);
        assert!((value_at(rect, fraction(1.0, VOLUME_RANGE)).x - 92.0).abs() < 1e-4);
    }

    #[test]
    fn tabs_map_to_screens() {
        assert_eq!(screen_for_tab("Session"), Some("session"));
        assert_eq!(screen_for_tab("Select dance"), Some("dance"));
        assert_eq!(screen_for_tab("Dance"), Some("dance"));
        assert_eq!(screen_for_tab("Select Tune"), Some("tune"));
        assert_eq!(screen_for_tab("Tune"), Some("tune"));
        assert_eq!(screen_for_tab("Preferences"), Some("preferences"));
        assert_eq!(screen_for_tab("Prefs"), Some("preferences"));
        assert_eq!(screen_for_tab("About"), None);
    }

    #[test]
    fn transport_actions_cover_both_layouts() {
        assert_eq!(
            SessionAction::End.labels(),
            &["End tune", "End", "Cancel ending", "Cancel", "Ending…"]
        );
        assert_eq!(
            SessionAction::CancelEnding.labels(),
            &["Cancel ending", "Cancel"]
        );
    }
}
