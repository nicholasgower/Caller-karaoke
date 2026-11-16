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
//! | Transport buttons | `Play`/`Pause` (one toggle), `Restart`, `End tune`/`End`, `Cancel ending`/`Cancel` |
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

    /// The **Undo** button's enabled state. Present only while Preferences is
    /// open, because that is where the buttons live.
    pub undo_available: Option<bool>,

    /// The **Redo** button's enabled state. Present only while Preferences is
    /// open.
    pub redo_available: Option<bool>,
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
                    match label.as_str() {
                        "Undo" => state.undo_available = Some(!node.is_disabled()),
                        "Redo" => state.redo_available = Some(!node.is_disabled()),
                        _ => {}
                    }
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
    Undo,
    Redo,
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
            Self::Undo => &["Undo"],
            Self::Redo => &["Redo"],
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Self::Play => "the Play button",
            Self::Pause => "the Pause button",
            Self::Restart => "the Restart button",
            Self::End => "the End tune button",
            Self::CancelEnding => "the Cancel ending button (no ending is pending)",
            Self::Undo => "the Undo button",
            Self::Redo => "the Redo button",
        }
    }

    /// Undo and redo are app-wide keyboard shortcuts rather than transport
    /// buttons, so they are pressed on the keyboard and work from any screen.
    ///
    /// `Modifiers::COMMAND` is Cmd on macOS and Ctrl elsewhere, the same chord
    /// a user presses. `None` means the action clicks its button.
    fn shortcut(self) -> Option<(egui::Key, Modifiers)> {
        match self {
            Self::Undo => Some((egui::Key::Z, Modifiers::COMMAND)),
            Self::Redo => Some((egui::Key::Y, Modifiers::COMMAND)),
            _ => None,
        }
    }
}

/// The way a finger travels across the screen to change tabs.
///
/// A swipe left moves to the next tab and a swipe right to the previous one,
/// matching the app's own gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwipeDirection {
    /// The finger travels left: the next tab.
    Left,
    /// The finger travels right: the previous tab.
    Right,
}

impl SwipeDirection {
    /// How far the finger travels, in logical points.
    ///
    /// The app needs 60 points to see a swipe at all; 160 is a deliberate
    /// gesture rather than a flick that might be read as a tap.
    const TRAVEL: f32 = 160.0;

    fn travel(self) -> f32 {
        match self {
            Self::Left => -Self::TRAVEL,
            Self::Right => Self::TRAVEL,
        }
    }
}

/// Whether pressing the single Play/Pause button would move the transport.
///
/// Play and Pause share one toggle button whose accessible name follows the
/// action it offers. A request for the state the session is already in must be
/// a no-op rather than a press, which would flip the music.
fn needs_toggle(action: SessionAction, transport: Option<TransportState>) -> bool {
    let playing = matches!(
        transport,
        Some(TransportState::Playing | TransportState::CountIn | TransportState::Ending)
    );
    match action {
        SessionAction::Play => !playing,
        SessionAction::Pause => playing,
        _ => true,
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
        // Undo and redo are keyboard shortcuts, not session-tab buttons: they
        // work from whichever screen is in front, so do not switch tabs first.
        if let Some((key, modifiers)) = action.shortcut() {
            self.press_shortcut(key, modifiers).await?;
            return self.state().await;
        }
        self.ensure_session().await?;
        // Play and Pause are one toggle button. Asking for the state the
        // session is already in must not press it, or "pause" while paused
        // would start the music.
        if matches!(action, SessionAction::Play | SessionAction::Pause) {
            let state = self.state().await?;
            if !needs_toggle(action, state.transport) {
                return Ok(state);
            }
        }
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

    /// Press a chord the way a user does: hold the modifiers, tap the key, and
    /// release.
    async fn press_shortcut(&self, key: egui::Key, modifiers: Modifiers) -> Result<(), String> {
        self.bridge
            .apply_events(vec![
                Event::ModifiersChanged(modifiers),
                key_event(key, modifiers, true),
                key_event(key, modifiers, false),
                Event::ModifiersChanged(Modifiers::NONE),
            ])
            .await
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

    /// Swipe across the screen with one finger and report the resulting state.
    ///
    /// A tab swipe is a touch gesture: the app ignores a mouse drag over the
    /// same empty content. So this sends what a touch screen reports for a
    /// finger — the touch itself plus the primary pointer press, moves and
    /// release every backend derives from it — and tries a few points of empty
    /// content until the tab actually changes.
    pub async fn swipe(&self, direction: SwipeDirection) -> Result<SessionState, String> {
        let before = self.state().await?;
        let snap = self.bridge.fetch_tree().await?;
        let starts = swipe_starts(&snap, direction);
        if starts.is_empty() {
            return Err(format!(
                "found no empty content to swipe across on the {} screen — every point is a \
                 control; use `press_key` with ArrowLeft/ArrowRight or click a tab instead",
                before.screen.as_deref().unwrap_or("current")
            ));
        }
        for from in starts {
            let to = Pos2::new(from.x + direction.travel(), from.y);
            self.touch_swipe(from, to).await?;
            let after = self.state().await?;
            if after.screen != before.screen {
                return Ok(after);
            }
        }
        Err(format!(
            "the swipe did not change tabs from the {} screen; the screen may have a popup \
             open",
            before.screen.as_deref().unwrap_or("current")
        ))
    }

    /// Put one finger down at `from`, drag it to `to`, and lift it.
    ///
    /// One frame per step, so the app sees a press, a moving finger and a
    /// release rather than a teleport.
    async fn touch_swipe(&self, from: Pos2, to: Pos2) -> Result<(), String> {
        for frame in swipe_frames(from, to) {
            self.bridge.apply_events(frame).await?;
        }
        Ok(())
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

/// The frames of one finger swipe: the touch down, the moves, and the lift.
///
/// The app reads a swipe from a frame that carries both the touch and the
/// pointer press derived from it, the way `egui-winit` and the web backend
/// deliver one finger, so the two stay together in the first frame.
fn swipe_frames(from: Pos2, to: Pos2) -> Vec<Vec<Event>> {
    const STEPS: u32 = 6;
    let device_id = egui::TouchDeviceId(1);
    let id = egui::TouchId(1);
    let touch = |phase, pos| Event::Touch {
        device_id,
        id,
        phase,
        pos,
        force: None,
    };
    let mut frames = vec![vec![
        touch(egui::TouchPhase::Start, from),
        Event::PointerMoved(from),
        pointer_button(from, true),
    ]];
    for step in 1..=STEPS {
        let t = step as f32 / STEPS as f32;
        let pos = Pos2::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        frames.push(vec![
            touch(egui::TouchPhase::Move, pos),
            Event::PointerMoved(pos),
        ]);
    }
    frames.push(vec![
        touch(egui::TouchPhase::End, to),
        pointer_button(to, false),
        Event::PointerGone,
    ]);
    frames
}

fn key_event(key: egui::Key, modifiers: Modifiers, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn tree_of(snap: &egui_mcp::TreeSnapshot) -> Result<&Tree, String> {
    snap.tree
        .as_ref()
        .ok_or("no accessibility tree yet".to_owned())
}

/// Roles that take a press for themselves, so a swipe that starts on one is not
/// a tab change. They mirror the controls the app treats as owning the gesture;
/// labels and frames are not listed because a press on them is still a swipe.
fn takes_a_press(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::CheckBox
            | Role::RadioButton
            | Role::Slider
            | Role::SpinButton
            | Role::TextInput
            | Role::ComboBox
            | Role::Link
            | Role::MenuItem
            | Role::Tab
            | Role::Switch
            | Role::ScrollBar
    )
}

/// Points where a finger swipe starts on empty content, best first.
///
/// The app reads a swipe only from the screen content, below the tab row. The
/// grid walks down that area and across it in thirds: the app's screens stack
/// boxes with gaps between them, so a clear point is usually a row or two down.
fn swipe_starts(snap: &egui_mcp::TreeSnapshot, direction: SwipeDirection) -> Vec<Pos2> {
    let Some(tree) = snap.tree.as_ref() else {
        return Vec::new();
    };
    let root = tree.state().root();

    // The root node carries no geometry of its own, so the window is the union
    // of everything drawn in it. The tab row is the lowest tab button across
    // the top, which bounds the content without hard-coding a height that
    // changes between the desktop and phone layouts.
    let mut viewport = egui::Rect::NOTHING;
    let mut tab_row_bottom = None;
    let mut blocked = Vec::new();
    visit(root, &mut |node| {
        if node.is_hidden() {
            return;
        }
        let Some(rect) = rect_of(&node, snap.pixels_per_point) else {
            return;
        };
        viewport = viewport.union(rect);
        if node.role() == Role::Button
            && screen_for_tab(&node.label().unwrap_or_default()).is_some()
        {
            tab_row_bottom =
                Some(tab_row_bottom.map_or(rect.bottom(), |bottom: f32| bottom.max(rect.bottom())));
        }
        if takes_a_press(node.role()) {
            blocked.push(rect);
        }
    });
    if !viewport.is_positive() {
        return Vec::new();
    }
    let content = egui::Rect::from_min_max(
        Pos2::new(
            viewport.left(),
            tab_row_bottom.unwrap_or(viewport.top() + 64.0),
        ),
        viewport.max,
    );
    blank_points(viewport, content, direction, &blocked)
}

/// The candidate swipe starts for one direction, in the order to try them.
fn blank_points(
    viewport: egui::Rect,
    content: egui::Rect,
    direction: SwipeDirection,
    blocked: &[egui::Rect],
) -> Vec<Pos2> {
    let mut points = Vec::new();
    if !content.is_positive() {
        return points;
    }
    // Eighths down the content: contacts sit in hand-sized rows, so a point
    // between two boxes is usually within a row or two of the top.
    for row in 0..8 {
        let y = content.top() + content.height() * (0.06 + 0.12 * row as f32);
        // Thirds across it, middle first: a swipe that starts at the edge would
        // leave the window before it is long enough to be read.
        for column in [0.5, 0.25, 0.75] {
            let from = Pos2::new(content.left() + content.width() * column, y);
            let to = Pos2::new(from.x + direction.travel(), y);
            let clear = content.contains(from) && viewport.contains(to);
            if clear && !blocked.iter().any(|rect| rect.contains(from)) {
                points.push(from);
            }
        }
    }
    points
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

    #[test]
    fn play_and_pause_share_one_toggle_button() {
        assert_eq!(SessionAction::Play.labels(), &["Play"]);
        assert_eq!(SessionAction::Pause.labels(), &["Pause"]);
        // The label names the action, and only a press that changes the state
        // reaches the button.
        assert!(needs_toggle(
            SessionAction::Play,
            Some(TransportState::Ready)
        ));
        assert!(needs_toggle(
            SessionAction::Play,
            Some(TransportState::Paused)
        ));
        assert!(needs_toggle(
            SessionAction::Play,
            Some(TransportState::Finished)
        ));
        assert!(!needs_toggle(
            SessionAction::Play,
            Some(TransportState::Playing)
        ));
        assert!(!needs_toggle(
            SessionAction::Play,
            Some(TransportState::CountIn)
        ));
        assert!(needs_toggle(
            SessionAction::Pause,
            Some(TransportState::CountIn)
        ));
        assert!(needs_toggle(
            SessionAction::Pause,
            Some(TransportState::Ending)
        ));
        assert!(!needs_toggle(
            SessionAction::Pause,
            Some(TransportState::Paused)
        ));
        // The other actions press their own dedicated buttons every time.
        assert!(needs_toggle(
            SessionAction::Restart,
            Some(TransportState::Playing)
        ));
    }

    #[test]
    fn undo_and_redo_are_keyboard_shortcuts() {
        assert_eq!(
            SessionAction::Undo.shortcut(),
            Some((egui::Key::Z, Modifiers::COMMAND))
        );
        assert_eq!(
            SessionAction::Redo.shortcut(),
            Some((egui::Key::Y, Modifiers::COMMAND))
        );
        assert_eq!(SessionAction::Play.shortcut(), None);
        // Both are named in the tool schema as actions.
        assert_eq!(
            serde_json::from_str::<SessionAction>(r#""undo""#).expect("deserialize"),
            SessionAction::Undo
        );
        assert_eq!(
            serde_json::from_str::<SessionAction>(r#""redo""#).expect("deserialize"),
            SessionAction::Redo
        );
    }

    #[test]
    fn a_swipe_left_goes_forward_and_a_swipe_right_goes_back() {
        // The finger travel, not the tab, names the direction: left is the
        // next tab.
        assert!(SwipeDirection::Left.travel() < 0.0);
        assert!(SwipeDirection::Right.travel() > 0.0);
        assert_eq!(
            serde_json::from_str::<SwipeDirection>(r#""left""#).expect("deserialize"),
            SwipeDirection::Left
        );
        assert_eq!(
            serde_json::from_str::<SwipeDirection>(r#""right""#).expect("deserialize"),
            SwipeDirection::Right
        );
    }

    #[test]
    fn a_swipe_starts_on_empty_content_and_stays_inside_the_window() {
        let viewport = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(390.0, 844.0));
        let content = egui::Rect::from_min_max(Pos2::new(0.0, 64.0), viewport.max);
        // A full-width row, like a saved-item list, in the middle of the screen.
        let row = egui::Rect::from_min_size(Pos2::new(0.0, 300.0), egui::vec2(390.0, 44.0));

        for direction in [SwipeDirection::Left, SwipeDirection::Right] {
            let points = blank_points(viewport, content, direction, &[row]);
            assert!(!points.is_empty(), "empty content still offers a start");
            for point in points {
                assert!(content.contains(point), "{point:?} is above the content");
                assert!(!row.contains(point), "{point:?} starts on a control");
                let end = Pos2::new(point.x + direction.travel(), point.y);
                assert!(
                    viewport.contains(end),
                    "{point:?} swiping {direction:?} leaves the window"
                );
            }
        }
    }

    #[test]
    fn controls_take_the_press_but_labels_do_not() {
        assert!(takes_a_press(Role::Slider));
        assert!(takes_a_press(Role::Button));
        assert!(takes_a_press(Role::TextInput));
        // A press on a plain label or a frame is still a tab swipe.
        assert!(!takes_a_press(Role::Label));
        assert!(!takes_a_press(Role::GenericContainer));
        assert!(!takes_a_press(Role::Image));
    }

    #[test]
    fn a_swipe_frame_pairs_the_finger_with_its_pointer_press() {
        let from = Pos2::new(300.0, 400.0);
        let to = Pos2::new(140.0, 400.0);
        let frames = swipe_frames(from, to);

        // The app only counts a swipe when the frame that presses the pointer
        // also carries the touch, so they must not be split apart.
        let first = &frames[0];
        assert!(first.iter().any(|event| matches!(
            event,
            Event::Touch {
                phase: egui::TouchPhase::Start,
                ..
            }
        )));
        assert!(
            first
                .iter()
                .any(|event| matches!(event, Event::PointerButton { pressed: true, .. }))
        );
        assert!(
            first
                .iter()
                .any(|event| matches!(event, Event::PointerMoved(_)))
        );

        // The release lifts the finger, and the gesture is purely horizontal:
        // a vertical component would be read as a scroll.
        let last = frames.last().expect("a swipe releases");
        assert!(
            last.iter()
                .any(|event| matches!(event, Event::PointerButton { pressed: false, .. }))
        );
        assert!(last.iter().any(|event| matches!(
            event,
            Event::Touch {
                phase: egui::TouchPhase::End,
                ..
            }
        )));
        for frame in &frames[1..] {
            for event in frame {
                if let Event::Touch { pos, .. } | Event::PointerMoved(pos) = event {
                    assert!((pos.y - from.y).abs() < f32::EPSILON, "a swipe stays level");
                    assert!(
                        pos.x <= from.x && pos.x >= to.x,
                        "the finger only moves left"
                    );
                }
            }
        }
    }
}
