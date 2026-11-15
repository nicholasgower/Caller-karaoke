//! Native window handling that egui does not provide.
//!
//! [`LaunchFocus`] brings the window to the front when the app starts, and
//! [`ScaleGuard`] keeps the window the same size when it moves between
//! monitors.
//!
//! Windows and winit try to preserve the *logical* window size when a window
//! crosses onto a monitor with a different scale factor, but a drag between
//! two such monitors can leave the window at its old *physical* size. The
//! logical size then changes by the ratio between the two displays: a 1120 pt
//! window becomes 1680 pt wide when it moves from a 150% display to a 100% one,
//! so the app suddenly grows to more than a screen. [`ScaleGuard`] remembers
//! the last settled logical size and undoes a size change that is that ratio.
//!
//! The correction waits until the window has stopped moving. Resizing a window
//! while the user is still dragging it makes the frame slide out from under the
//! cursor, because the operating system keeps positioning the window from the
//! cursor and the grab point it recorded when the drag began.

use eframe::egui;

/// How long the window must hold still before its size is corrected.
///
/// A drag keeps changing the window position, so the correction only runs once
/// the user has released the window (or paused long enough) and the frame can
/// be resized without moving away from the cursor.
const MOVE_SETTLE_SECONDS: f64 = 0.5;

/// How long after the window last moved a resize may still be the display
/// ratio.
///
/// The stray resize can arrive seconds after the scale-factor event, when the
/// drag finishes on the other display, so this horizon covers the whole move.
const MOVE_HORIZON_SECONDS: f64 = 10.0;

/// Logical size differences below this are rounding noise, not a resize.
const SIZE_EPSILON: f32 = 1.0;

/// Position differences below this are rounding noise, not a move.
const POSITION_EPSILON: f32 = 0.5;

/// How closely a size must match the display ratio to be treated as the bug.
///
/// The ratio is applied to the window's *outer* size, so the inner size drifts
/// by the border width: a 1120 pt inner window grows by 1.51, not 1.50. Five
/// percent covers that (and fractional scale factors such as 1.25 vs 1.5)
/// without matching an ordinary resize.
const RATIO_TOLERANCE: f32 = 0.05;

/// Frames for which the app keeps asking for the launch focus.
///
/// The window is shown at the end of the frame that paints it for the first
/// time, and that frame's focus request reaches the window manager right after,
/// so one request is normally enough. The extra frames cover a window manager
/// that needs another pass before the window is on screen.
const LAUNCH_FOCUS_FRAMES: u64 = 3;

/// Takes the keyboard focus when the app starts.
///
/// eframe creates the window hidden and shows it only once it has painted a
/// frame, so the user does not see a white flash. A window shown that way does
/// not necessarily come to the front: when the app is started from a terminal
/// or another application, Windows refuses to move the new window to the
/// foreground and leaves it behind the window that launched it. An explicit
/// focus request is allowed to do that (winit retries it with
/// `SetForegroundWindow` after taking the foreground permission), so the app
/// asks for the focus itself instead of relying on the plain show request.
///
/// The request is limited to the first frames: after that the user may have
/// picked another window, and the app must not pull the focus back.
#[derive(Debug, Default)]
pub struct LaunchFocus {
    /// The frame counter when the first frame ran; `None` before that.
    start_frame: Option<u64>,
    /// The window has had the focus, or the launch frames are over.
    done: bool,
}

impl LaunchFocus {
    /// Ask for the focus until the window is focused or the launch frames end.
    pub fn update(&mut self, ctx: &egui::Context) {
        if self.done {
            return;
        }

        if ctx.input(|input| input.viewport().focused.unwrap_or(false)) {
            // The window manager already gave us the focus: never ask again.
            self.done = true;
            return;
        }

        let frame = ctx.cumulative_frame_nr();
        let start = *self.start_frame.get_or_insert(frame);
        if frame.saturating_sub(start) >= LAUNCH_FOCUS_FRAMES {
            self.done = true;
            return;
        }

        // The web canvas cannot take focus from the browser, and the backend
        // would only log an unhandled command.
        if !cfg!(target_arch = "wasm32") {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }
}

/// Display scales remembered while the window is moving.
///
/// Four slots cover a laptop with three external displays; the oldest scale is
/// dropped when another one appears.
const MAX_SCALES: usize = 4;

/// Preserves the window's logical size across monitor changes.
///
/// The app is laid out in points, so its size must not change just because the
/// window moved to a monitor with a different pixel density. Once the window
/// has stopped moving, egui is told to restore the last settled size when a
/// monitor change left it at the ratio between two displays.
#[derive(Debug)]
pub struct ScaleGuard {
    pixels_per_point: Option<f32>,
    expected_size: Option<egui::Vec2>,
    /// Distinct display scales seen since the window started moving.
    scales: [f32; MAX_SCALES],
    scale_count: usize,
    /// Last window position, for noticing that the window is being dragged.
    last_position: Option<egui::Pos2>,
    /// When the window last moved or changed scale.
    last_change: f64,
}

impl Default for ScaleGuard {
    fn default() -> Self {
        Self {
            pixels_per_point: None,
            expected_size: None,
            scales: [0.0; MAX_SCALES],
            scale_count: 0,
            last_position: None,
            last_change: 0.0,
        }
    }
}

impl ScaleGuard {
    /// Watch the window for scale-factor changes.
    ///
    /// Returns `true` when the scale factor changed this frame, so the caller
    /// can drop resources that were rasterized for the previous density.
    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let ppp = ctx.pixels_per_point();
        let scale_changed = self.pixels_per_point != Some(ppp);
        let (inner_rect, outer_rect, maximized) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.inner_rect,
                viewport.outer_rect,
                viewport.maximized == Some(true) || viewport.fullscreen == Some(true),
            )
        });

        let Some(inner_rect) = inner_rect else {
            // Minimized or not laid out yet: there is no size to compare.
            self.pixels_per_point = Some(ppp);
            return scale_changed;
        };

        let position = outer_rect.unwrap_or(inner_rect).min;
        let now = ctx.input(|input| input.time);
        if let Some(size) = self.observe(ppp, inner_rect.size(), position, maximized, now) {
            // Only the desktop backend has a window the user can drag between
            // monitors; the web canvas and Android activity fill their
            // container, so they must not be resized from here.
            #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            #[cfg(any(target_arch = "wasm32", target_os = "android"))]
            let _ = size;
        }

        scale_changed
    }

    /// Record this frame's window state and return the logical size to
    /// restore, if the window was resized by a monitor change.
    fn observe(
        &mut self,
        ppp: f32,
        size: egui::Vec2,
        position: egui::Pos2,
        maximized: bool,
        now: f64,
    ) -> Option<egui::Vec2> {
        if self
            .last_position
            .is_none_or(|last| (last - position).length() > POSITION_EPSILON)
        {
            // The window is being dragged (or moved by the window manager).
            self.last_position = Some(position);
            self.last_change = now;
        }

        let Some(expected) = self.expected_size else {
            // First frame: learn the window's size.
            self.pixels_per_point = Some(ppp);
            self.expected_size = Some(size);
            self.reset_scales(ppp);
            return None;
        };

        if self.pixels_per_point != Some(ppp) {
            // The window crossed onto a monitor with a different density. Keep
            // the size it had and watch for a stray resize.
            self.pixels_per_point = Some(ppp);
            self.record_scale(ppp);
            self.last_change = now;
            return None;
        }

        // A maximized or fullscreen window is the size of its monitor by
        // definition; resizing it back would undo the maximize.
        if maximized {
            return None;
        }

        if (size - expected).length() > SIZE_EPSILON
            && is_display_ratio(expected, size, &self.scales[..self.scale_count])
            && now - self.last_change <= MOVE_HORIZON_SECONDS
        {
            // The window kept its old physical size instead of its old logical
            // size. Wait until the drag ends, then put the logical size back:
            // resizing a moving window slides it out from under the cursor.
            if now - self.last_change >= MOVE_SETTLE_SECONDS {
                return Some(expected);
            }
            return None;
        }

        // A stable frame: this is the size the user wants. Follow it, so a
        // deliberate resize is never undone, and forget the old scales once
        // the move is over.
        self.expected_size = Some(size);
        if now - self.last_change > MOVE_HORIZON_SECONDS {
            self.reset_scales(ppp);
        }
        None
    }

    fn reset_scales(&mut self, ppp: f32) {
        self.scales = [0.0; MAX_SCALES];
        self.scales[0] = ppp;
        self.scale_count = 1;
    }

    fn record_scale(&mut self, ppp: f32) {
        if self.scales[..self.scale_count].contains(&ppp) {
            return;
        }
        if self.scale_count < MAX_SCALES {
            self.scales[self.scale_count] = ppp;
            self.scale_count += 1;
        } else {
            self.scales.rotate_left(1);
            self.scales[MAX_SCALES - 1] = ppp;
        }
    }
}

/// Whether `size` is `expected` multiplied (or divided) by the ratio between
/// two of the displays the window has visited.
fn is_display_ratio(expected: egui::Vec2, size: egui::Vec2, scales: &[f32]) -> bool {
    for (index, first) in scales.iter().enumerate() {
        for second in &scales[index + 1..] {
            let ratio = first.max(*second) / first.min(*second);
            if ratio > 1.0 + f32::EPSILON
                && (matches_ratio(expected, size, ratio)
                    || matches_ratio(expected, size, 1.0 / ratio))
            {
                return true;
            }
        }
    }
    false
}

fn matches_ratio(expected: egui::Vec2, size: egui::Vec2, scale: f32) -> bool {
    let target = expected * scale;
    let tolerance = target * RATIO_TOLERANCE;
    (size.x - target.x).abs() <= tolerance.x.abs() + SIZE_EPSILON
        && (size.y - target.y).abs() <= tolerance.y.abs() + SIZE_EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs one frame and returns the viewport commands it sent.
    ///
    /// These tests render no widgets, so a frame is a single pass and the frame
    /// counter advances once per call. `focused` reports the window as focused
    /// in that frame's input, as the window manager does once the window has
    /// the keyboard.
    fn run_frame(
        focus: &mut LaunchFocus,
        ctx: &egui::Context,
        focused: bool,
    ) -> Vec<egui::ViewportCommand> {
        let mut input = egui::RawInput::default();
        if focused {
            input.viewports.insert(
                egui::ViewportId::ROOT,
                egui::ViewportInfo {
                    focused: Some(true),
                    ..Default::default()
                },
            );
        }
        let output = ctx.run_pass(input, |ctx| focus.update(ctx));
        let commands = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|viewport| viewport.commands.clone())
            .unwrap_or_default();
        // The test has no renderer to upload the font atlas to.
        output.drop_without_applying_deltas();
        commands
    }

    /// The bug: a window that eframe shows after its first frame stays behind
    /// the program that launched it unless the app asks for the focus itself.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn asks_for_the_focus_when_the_window_opens() {
        let ctx = egui::Context::default();
        let mut focus = LaunchFocus::default();
        assert!(run_frame(&mut focus, &ctx, false).contains(&egui::ViewportCommand::Focus));
    }

    /// A window that is focused from the start needs no request.
    #[test]
    fn does_not_ask_for_a_focus_it_already_has() {
        let ctx = egui::Context::default();
        let mut focus = LaunchFocus::default();
        let commands = run_frame(&mut focus, &ctx, true);
        assert!(
            !commands.contains(&egui::ViewportCommand::Focus),
            "{commands:?}"
        );
    }

    /// Once the window has the focus, the app must not keep asking for it.
    #[test]
    fn stops_asking_once_the_window_has_the_focus() {
        let ctx = egui::Context::default();
        let mut focus = LaunchFocus::default();
        run_frame(&mut focus, &ctx, false);
        for _ in 0..2 {
            let commands = run_frame(&mut focus, &ctx, true);
            assert!(
                !commands.contains(&egui::ViewportCommand::Focus),
                "{commands:?}"
            );
        }
    }

    /// If the window never gets the focus, the requests stop after the launch
    /// frames: the user may have chosen another window, and the app must not
    /// pull the focus back from it.
    #[test]
    fn stops_asking_after_the_launch_frames() {
        let ctx = egui::Context::default();
        let mut focus = LaunchFocus::default();
        for _ in 0..LAUNCH_FOCUS_FRAMES {
            run_frame(&mut focus, &ctx, false);
        }
        for _ in 0..5 {
            let commands = run_frame(&mut focus, &ctx, false);
            assert!(
                !commands.contains(&egui::ViewportCommand::Focus),
                "the app asked for the focus after its launch frames: {commands:?}"
            );
        }
    }

    fn size(x: f32, y: f32) -> egui::Vec2 {
        egui::vec2(x, y)
    }

    fn pos(x: f32, y: f32) -> egui::Pos2 {
        egui::pos2(x, y)
    }

    fn still(guard: &mut ScaleGuard, ppp: f32, size: egui::Vec2, now: f64) {
        guard.observe(ppp, size, pos(0.0, 0.0), false, now);
    }

    /// A fresh guard learns the first size it sees and never restores it.
    #[test]
    fn first_frame_learns_the_window_size() {
        let mut guard = ScaleGuard::default();
        assert_eq!(
            guard.observe(1.5, size(1120.0, 780.0), pos(0.0, 0.0), false, 0.0),
            None
        );
        assert_eq!(
            guard.observe(1.5, size(1120.0, 780.0), pos(0.0, 0.0), false, 0.1),
            None
        );
    }

    /// A normal resize (no monitor change) is the user's choice and must stick.
    #[test]
    fn follows_a_deliberate_resize() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        assert_eq!(
            guard.observe(1.5, size(900.0, 700.0), pos(0.0, 0.0), false, 2.0),
            None
        );
        assert_eq!(
            guard.observe(1.5, size(1000.0, 720.0), pos(0.0, 0.0), false, 2.1),
            None
        );
    }

    /// Winit keeps the logical size on most moves; the guard must not touch it.
    #[test]
    fn leaves_a_preserved_size_alone() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        assert_eq!(
            guard.observe(1.0, size(1120.0, 780.0), pos(0.0, 0.0), false, 2.0),
            None
        );
    }

    /// The regression: crossing a monitor boundary multiplies the size by the
    /// scale ratio (1120 * 1.5) instead of keeping the points.
    #[test]
    fn restores_a_size_scaled_by_the_monitor() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        assert_eq!(
            guard.observe(1.0, size(1680.0, 1170.0), pos(0.0, 0.0), false, 1.6),
            Some(size(1120.0, 780.0))
        );
        // The restore is a request: the size only changes later, so the guard
        // keeps asking until the window matches.
        assert_eq!(
            guard.observe(1.0, size(1680.0, 1170.0), pos(0.0, 0.0), false, 1.7),
            Some(size(1120.0, 780.0))
        );
        assert_eq!(
            guard.observe(1.0, size(1120.0, 780.0), pos(0.0, 0.0), false, 1.8),
            None
        );
    }

    /// The true bug scales the outer size, so the inner size grows by slightly
    /// more than the ratio (the window border is scaled too).
    #[test]
    fn restores_the_border_adjusted_ratio() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        assert_eq!(
            guard.observe(1.0, size(1687.3, 1188.7), pos(0.0, 0.0), false, 1.6),
            Some(size(1120.0, 780.0))
        );
    }

    /// The correction must not run while the window is still moving: resizing
    /// a dragged window slides it out from under the cursor.
    #[test]
    fn waits_until_the_window_stops_moving() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        // The drag is still in progress.
        assert_eq!(
            guard.observe(1.0, size(1680.0, 1170.0), pos(100.0, 0.0), false, 1.2),
            None
        );
        assert_eq!(
            guard.observe(1.0, size(1680.0, 1170.0), pos(200.0, 0.0), false, 1.3),
            None
        );
        // The window is released and holds still long enough to resize safely.
        assert_eq!(
            guard.observe(1.0, size(1680.0, 1170.0), pos(200.0, 0.0), false, 1.9),
            Some(size(1120.0, 780.0))
        );
    }

    /// A slow drag can outlast the move horizon; as long as the window keeps
    /// moving, the pending correction must not expire.
    #[test]
    fn survives_a_long_drag() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        let mut now = 1.0;
        for step in 1..=20 {
            now += 1.0;
            assert_eq!(
                guard.observe(
                    1.0,
                    size(1680.0, 1170.0),
                    pos(step as f32 * 40.0, 0.0),
                    false,
                    now
                ),
                None
            );
        }
        assert_eq!(
            guard.observe(1.0, size(1680.0, 1170.0), pos(800.0, 0.0), false, now + 1.0),
            Some(size(1120.0, 780.0))
        );
    }

    /// A window that shrank by the scale ratio is the same bug in reverse.
    #[test]
    fn restores_a_size_shrunk_by_the_monitor() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.0, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.5, size(1120.0, 780.0), 1.0);
        assert_eq!(
            guard.observe(1.5, size(747.0, 520.0), pos(0.0, 0.0), false, 1.6),
            Some(size(1120.0, 780.0))
        );
    }

    /// Maximizing makes the window the size of the monitor; the guard must not
    /// fight it.
    #[test]
    fn ignores_maximized_windows() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        assert_eq!(
            guard.observe(1.0, size(1920.0, 1040.0), pos(0.0, 0.0), true, 1.6),
            None
        );
    }

    /// Long after a move, a resize is the user's choice even at the old ratio.
    #[test]
    fn does_not_fight_a_later_resize() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        assert_eq!(
            guard.observe(1.0, size(1680.0, 1170.0), pos(0.0, 0.0), false, 30.0),
            None
        );
        // ...and the new size becomes the expected one.
        assert_eq!(
            guard.observe(1.0, size(1120.0, 780.0), pos(0.0, 0.0), false, 30.1),
            None
        );
    }

    /// Several scale changes in one drag (150% -> 100% -> 150%) only have to
    /// settle once, and the correction is still based on the original size.
    #[test]
    fn survives_a_back_and_forth_move() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        still(&mut guard, 1.5, size(1120.0, 780.0), 1.2);
        assert_eq!(
            guard.observe(1.5, size(1680.0, 1170.0), pos(0.0, 0.0), false, 1.8),
            Some(size(1120.0, 780.0))
        );
    }

    /// Monitors with the same scale never produce this bug, so an ordinary
    /// 1.5x resize is left alone.
    #[test]
    fn ignores_ratios_when_the_displays_match() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        assert_eq!(
            guard.observe(1.5, size(1680.0, 1170.0), pos(0.0, 0.0), false, 1.0),
            None
        );
    }

    /// A third display adds a second ratio; the stray resize may use either.
    #[test]
    fn restores_any_ratio_between_visited_displays() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.0, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.5, size(1120.0, 780.0), 1.0);
        still(&mut guard, 1.75, size(1120.0, 780.0), 1.5);
        // 1120 * 1.5, the ratio to the middle display rather than the first.
        assert_eq!(
            guard.observe(1.75, size(1680.0, 1170.0), pos(0.0, 0.0), false, 2.1),
            Some(size(1120.0, 780.0))
        );
    }

    /// A deliberate resize while the window is being dragged is followed when
    /// it does not look like the display ratio.
    #[test]
    fn follows_a_resize_that_is_not_the_display_ratio() {
        let mut guard = ScaleGuard::default();
        still(&mut guard, 1.5, size(1120.0, 780.0), 0.0);
        still(&mut guard, 1.0, size(1120.0, 780.0), 1.0);
        assert_eq!(
            guard.observe(1.0, size(900.0, 700.0), pos(100.0, 0.0), false, 1.2),
            None
        );
        assert_eq!(
            guard.observe(1.0, size(900.0, 700.0), pos(100.0, 0.0), false, 1.9),
            None
        );
    }
}
