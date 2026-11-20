use super::about::about;
use super::{
    apply_phone_spacing, compact, filled_button_row, form_row, inline_field, number_slider,
    responsive_combo, slider_row,
};
use crate::app::history::HistoryAction;
use crate::app::*;
use crate::updates::{self, UpdateStatus};
use caller_core::{
    CardPadding, ChorusPreset, EchoPreset, MAX_HUMANIZE_AMOUNT, MAX_PID_KD, MAX_PID_KI, MAX_PID_KP,
    ReverbPreset,
};
use eframe::egui;

/// The top-level groups of Preferences.
///
/// A desktop shows the list in a left frame and the selected group's settings
/// beside it; a phone shows the list first and drills into one group, so a
/// long page is not one three-thousand-point scroll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum PreferencesCategory {
    Calling,
    BandAndSound,
    Playback,
    Appearance,
    DataAndStorage,
    About,
}

impl PreferencesCategory {
    pub(in crate::app) const ALL: [Self; 6] = [
        Self::Calling,
        Self::BandAndSound,
        Self::Playback,
        Self::Appearance,
        Self::DataAndStorage,
        Self::About,
    ];

    pub(in crate::app) fn label(self) -> &'static str {
        match self {
            Self::Calling => "Calling",
            Self::BandAndSound => "Band & Sound",
            Self::Playback => "Playback",
            Self::Appearance => "Appearance",
            Self::DataAndStorage => "Data & Storage",
            Self::About => "About",
        }
    }

    pub(in crate::app) fn summary(self) -> &'static str {
        match self {
            Self::Calling => "Role terms, lead timing, and spoken calls",
            Self::BandAndSound => "Band list, soundfont, effects, and mix",
            Self::Playback => "Loop, count-in, tempo range, and variations",
            Self::Appearance => "Theme, motion, dance card padding, shortcuts, undo",
            Self::DataAndStorage => "Web proxy, cache, MIDI export, and memory",
            Self::About => "Version, credits, and project links",
        }
    }
}

/// Which Preferences page is in front.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) enum PreferencesPage {
    /// The category list. A desktop also shows the selected group's settings
    /// beside it; a phone shows only the list until a category is chosen.
    #[default]
    Categories,
    Category(PreferencesCategory),
}

impl PreferencesPage {
    /// The category whose settings are shown on the desktop.
    pub(in crate::app) fn selected(self) -> PreferencesCategory {
        match self {
            Self::Categories => PreferencesCategory::Calling,
            Self::Category(category) => category,
        }
    }

    /// Where Back and the Android system back button lead inside Preferences.
    ///
    /// A phone drills down and can step back to the list; a desktop has the
    /// list on screen already, so Back leaves Preferences entirely.
    pub(in crate::app) fn back_target(self, compact: bool) -> Option<Self> {
        match self {
            Self::Category(_) if compact => Some(Self::Categories),
            _ => None,
        }
    }
}

/// One preferences group: where it belongs and what it draws.
#[derive(Clone, Copy)]
pub(in crate::app) struct PreferencesSection {
    pub category: PreferencesCategory,
    /// Stable name a search result uses to reveal this section. It is not a
    /// user-facing label; the section draws its own heading.
    pub anchor: &'static str,
    pub draw: fn(&mut CallerApp, &mut egui::Ui, bool),
}

impl PreferencesSection {
    const fn new(
        category: PreferencesCategory,
        anchor: &'static str,
        draw: fn(&mut CallerApp, &mut egui::Ui, bool),
    ) -> Self {
        Self {
            category,
            anchor,
            draw,
        }
    }
}

/// Every preferences section, in the order it is drawn inside its category.
///
/// A new setting belongs in the matching section function, so unrelated
/// branches edit different functions instead of the same 200-line body, and
/// gets an entry in [`SETTINGS`] so the search can find it.
pub(in crate::app) const SECTIONS: &[PreferencesSection] = &[
    PreferencesSection::new(PreferencesCategory::Calling, "role-terms", role_terms),
    PreferencesSection::new(PreferencesCategory::Calling, "calling-lead", calling_lead),
    PreferencesSection::new(PreferencesCategory::Calling, "speech", speech),
    PreferencesSection::new(
        PreferencesCategory::BandAndSound,
        "instruments",
        instruments,
    ),
    PreferencesSection::new(PreferencesCategory::BandAndSound, "soundfont", soundfont),
    PreferencesSection::new(PreferencesCategory::BandAndSound, "effects", effects),
    PreferencesSection::new(PreferencesCategory::Playback, "playback", playback),
    PreferencesSection::new(PreferencesCategory::Playback, "tempo-range", tempo_range),
    PreferencesSection::new(PreferencesCategory::Appearance, "appearance", appearance),
    PreferencesSection::new(
        PreferencesCategory::DataAndStorage,
        "data-sources",
        data_sources,
    ),
    PreferencesSection::new(PreferencesCategory::DataAndStorage, "web-proxy", web_proxy),
    PreferencesSection::new(
        PreferencesCategory::DataAndStorage,
        "maintenance",
        maintenance,
    ),
    PreferencesSection::new(PreferencesCategory::About, "about", about),
];

/// One setting the Preferences search can find.
///
/// The list is hand-written rather than derived from the widgets: a result
/// must jump to a real section, and a new control is not searchable until it
/// has an entry here. `keywords` carries the words a user is likely to try but
/// that are not in the title or category name.
struct PreferenceSetting {
    title: &'static str,
    category: PreferencesCategory,
    /// The [`PreferencesSection::anchor`] that draws this setting.
    section: &'static str,
    keywords: &'static str,
}

impl PreferenceSetting {
    /// Whether every word in `query` appears in the title, category, or
    /// keywords. `query` is already lower-case.
    fn matches(&self, query: &str) -> bool {
        let text =
            format!("{} {} {}", self.title, self.category.label(), self.keywords).to_lowercase();
        query.split_whitespace().all(|word| text.contains(word))
    }
}

/// Every setting a user can find through the search box, in category order.
///
/// Sections without individual controls (About) still get one entry so their
/// content is reachable by name.
const SETTINGS: &[PreferenceSetting] = &[
    PreferenceSetting {
        title: "Role terms",
        category: PreferencesCategory::Calling,
        section: "role-terms",
        keywords: "larks robins ladies gentlemen women men leads follows lefts rights preset",
    },
    PreferenceSetting {
        title: "Custom role terms",
        category: PreferencesCategory::Calling,
        section: "role-terms",
        keywords: "left singular plural right singular plural custom",
    },
    PreferenceSetting {
        title: "Dance move aliases",
        category: PreferencesCategory::Calling,
        section: "role-terms",
        keywords: "alias editor wording alternative calls phrase",
    },
    PreferenceSetting {
        title: "Automatic syllable-based lead",
        category: PreferencesCategory::Calling,
        section: "calling-lead",
        keywords: "lead timing auto speech",
    },
    PreferenceSetting {
        title: "Minimum speech lead",
        category: PreferencesCategory::Calling,
        section: "calling-lead",
        keywords: "beats timing minimum lead",
    },
    PreferenceSetting {
        title: "Maximum speech lead",
        category: PreferencesCategory::Calling,
        section: "calling-lead",
        keywords: "beats timing maximum lead",
    },
    PreferenceSetting {
        title: "Speech rate",
        category: PreferencesCategory::Calling,
        section: "calling-lead",
        keywords: "syllables per second pace delivery",
    },
    PreferenceSetting {
        title: "Manual speech lead",
        category: PreferencesCategory::Calling,
        section: "calling-lead",
        keywords: "beats fixed timing",
    },
    PreferenceSetting {
        title: "Safety padding",
        category: PreferencesCategory::Calling,
        section: "calling-lead",
        keywords: "buffer seconds delay lead",
    },
    PreferenceSetting {
        title: "Rebuild call timing",
        category: PreferencesCategory::Calling,
        section: "calling-lead",
        keywords: "refresh timeline recompute",
    },
    PreferenceSetting {
        title: "Speak calls",
        category: PreferencesCategory::Calling,
        section: "speech",
        keywords: "tts text to speech voice narration",
    },
    PreferenceSetting {
        title: "Voice",
        category: PreferencesCategory::Calling,
        section: "speech",
        keywords: "tts neural kokoro voice speaker",
    },
    PreferenceSetting {
        title: "Text to speech rate",
        category: PreferencesCategory::Calling,
        section: "speech",
        keywords: "tts speed pace rate",
    },
    PreferenceSetting {
        title: "Tone",
        category: PreferencesCategory::Calling,
        section: "speech",
        keywords: "delivery tts style",
    },
    PreferenceSetting {
        title: "Deliveries per call",
        category: PreferencesCategory::Calling,
        section: "speech",
        keywords: "tts versions repeats variation",
    },
    PreferenceSetting {
        title: "Shuffle alternative call aliases",
        category: PreferencesCategory::Calling,
        section: "speech",
        keywords: "wording alternatives random variation",
    },
    PreferenceSetting {
        title: "Instrument",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "band part voice row section",
    },
    PreferenceSetting {
        title: "Players",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "count section size band",
    },
    PreferenceSetting {
        title: "Instrument volume",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "band mix level loudness",
    },
    PreferenceSetting {
        title: "Instrument direction",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "pan front rear placement stereo surround",
    },
    PreferenceSetting {
        title: "Add instrument",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "band row new",
    },
    PreferenceSetting {
        title: "Dancer sounds",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "stomps claps dancer",
    },
    PreferenceSetting {
        title: "Dancer sound volume",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "stomps claps level",
    },
    PreferenceSetting {
        title: "Percussion rhythm",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "foot taps drumset one and a gallop",
    },
    PreferenceSetting {
        title: "Accompaniment rhythm",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "chords rhythm strum",
    },
    PreferenceSetting {
        title: "Caller volume",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "speech level mix volume",
    },
    PreferenceSetting {
        title: "Band volume",
        category: PreferencesCategory::BandAndSound,
        section: "instruments",
        keywords: "master music level mix volume",
    },
    PreferenceSetting {
        title: "Soundfont",
        category: PreferencesCategory::BandAndSound,
        section: "soundfont",
        keywords: "sf2 bank instruments fluid r3 load",
    },
    PreferenceSetting {
        title: "Load .sf2",
        category: PreferencesCategory::BandAndSound,
        section: "soundfont",
        keywords: "soundfont bank file instruments",
    },
    PreferenceSetting {
        title: "Reverb",
        category: PreferencesCategory::BandAndSound,
        section: "effects",
        keywords: "room hall effects",
    },
    PreferenceSetting {
        title: "Echo",
        category: PreferencesCategory::BandAndSound,
        section: "effects",
        keywords: "delay slapback effects",
    },
    PreferenceSetting {
        title: "Chorus",
        category: PreferencesCategory::BandAndSound,
        section: "effects",
        keywords: "shimmer widen effects",
    },
    PreferenceSetting {
        title: "Loop the dance and music",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "repeat loop",
    },
    PreferenceSetting {
        title: "Four-beat count-in",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "potatoes intro count in",
    },
    PreferenceSetting {
        title: "Vary the music on each loop",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "variations ornaments loop",
    },
    PreferenceSetting {
        title: "Variation options",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "articulation double stops grace notes octave chord stabs melody alone",
    },
    PreferenceSetting {
        title: "Fiddle bowing",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "bow accent tie eighth notes slur stroke fiddle",
    },
    PreferenceSetting {
        title: "Show original card text",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "card figures wording",
    },
    PreferenceSetting {
        title: "Humanize the band",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "drift spread human timing pitch intonation fiddle unfretted",
    },
    PreferenceSetting {
        title: "Timing amount",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "drift amount human tempo",
    },
    PreferenceSetting {
        title: "Tuning amount",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "pitch amount intonation out of tune fiddle",
    },
    PreferenceSetting {
        title: "Tempo controller",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "proportional integral derivative gains kp ki kd pid",
    },
    PreferenceSetting {
        title: "Gradual ending",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "final bar fade ending",
    },
    PreferenceSetting {
        title: "Resolve the final chord a fifth lower",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "ending final chord",
    },
    PreferenceSetting {
        title: "Output mode",
        category: PreferencesCategory::Playback,
        section: "playback",
        keywords: "mono stereo surround channels",
    },
    PreferenceSetting {
        title: "Tempo range",
        category: PreferencesCategory::Playback,
        section: "tempo-range",
        keywords: "minimum maximum bpm slider window",
    },
    PreferenceSetting {
        title: "Reset to 200–300",
        category: PreferencesCategory::Playback,
        section: "tempo-range",
        keywords: "restore default tempo bpm",
    },
    PreferenceSetting {
        title: "Theme",
        category: PreferencesCategory::Appearance,
        section: "appearance",
        keywords: "dark light colour color",
    },
    PreferenceSetting {
        title: "Smooth animations",
        category: PreferencesCategory::Appearance,
        section: "appearance",
        keywords: "motion reduce animation",
    },
    PreferenceSetting {
        title: "Dance card padding",
        category: PreferencesCategory::Appearance,
        section: "appearance",
        keywords: "compact sparse spacing row",
    },
    PreferenceSetting {
        title: "Show dance glossary",
        category: PreferencesCategory::Appearance,
        section: "appearance",
        keywords: "glossary terms definitions underline",
    },
    PreferenceSetting {
        title: "Undo",
        category: PreferencesCategory::Appearance,
        section: "appearance",
        keywords: "history ctrl z revert",
    },
    PreferenceSetting {
        title: "Redo",
        category: PreferencesCategory::Appearance,
        section: "appearance",
        keywords: "history ctrl y repeat",
    },
    PreferenceSetting {
        title: "Data sources",
        category: PreferencesCategory::DataAndStorage,
        section: "data-sources",
        keywords: "contradb source site download",
    },
    PreferenceSetting {
        title: "Web data proxy",
        category: PreferencesCategory::DataAndStorage,
        section: "web-proxy",
        keywords: "proxy cors web browser",
    },
    PreferenceSetting {
        title: "Proxy URL",
        category: PreferencesCategory::DataAndStorage,
        section: "web-proxy",
        keywords: "proxy token web address",
    },
    PreferenceSetting {
        title: "Proxy token",
        category: PreferencesCategory::DataAndStorage,
        section: "web-proxy",
        keywords: "proxy token web secret",
    },
    PreferenceSetting {
        title: "Export MIDI",
        category: PreferencesCategory::DataAndStorage,
        section: "maintenance",
        keywords: "midi file export save",
    },
    PreferenceSetting {
        title: "Clear source cache",
        category: PreferencesCategory::DataAndStorage,
        section: "maintenance",
        keywords: "cache offline storage clear",
    },
    PreferenceSetting {
        title: "Memory use",
        category: PreferencesCategory::DataAndStorage,
        section: "maintenance",
        keywords: "ram memory report refresh",
    },
    PreferenceSetting {
        title: "About",
        category: PreferencesCategory::About,
        section: "about",
        keywords: "version credits author project links license",
    },
];

/// The settings whose title, category, or keywords contain every word.
fn matching_settings(query: &str) -> Vec<&'static PreferenceSetting> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    SETTINGS
        .iter()
        .filter(|setting| setting.matches(&query))
        .collect()
}

/// The About section, named so the shell tests can assert that the version,
/// author, and project links are drawn from Preferences and nowhere else.
#[cfg(test)]
pub(in crate::app) const ABOUT_SECTION: fn(&mut CallerApp, &mut egui::Ui, bool) = about;

/// The "Preferences" title, with the feedback and update actions on its right.
///
/// A wide header keeps all three on one row, with **Check for updates** in the
/// top-right corner and **Submit feedback** to its left. At phone width the
/// buttons stack under the title and stay right-aligned, so the two long labels
/// never force the page wider than the 360-point minimum window.
fn preferences_header(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    if compact {
        preferences_title(ui);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                preferences_actions(app, ui);
            });
        });
    } else {
        ui.horizontal(|ui| {
            preferences_title(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                preferences_actions(app, ui);
            });
        });
    }
    update_status(app, ui);
}

/// The page title, with an accessible name.
///
/// `Ui::heading` paints the word but leaves the accessibility node's label
/// empty, so a screen reader (and the agent UI tests) cannot name the page from
/// the title node. Naming it here also gives the test a stable anchor for the
/// "same row as the title" layout check.
fn preferences_title(ui: &mut egui::Ui) {
    ui.heading("Preferences")
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, "Preferences"));
}

/// The two header actions, added right to left.
fn preferences_actions(app: &mut CallerApp, ui: &mut egui::Ui) {
    let checking = app.updates.is_checking();
    let check = ui
        .add_enabled(!checking, egui::Button::new("Check for updates"))
        .on_hover_text("Ask GitHub whether a newer release has been published")
        .accessible_name("Check for updates");
    if check.clicked() {
        app.updates.start();
    }
    // Added second, so it sits to the left of the update button.
    let feedback = ui
        .button("Submit feedback")
        .on_hover_text("Open the GitHub issue tracker in your browser")
        .accessible_name("Submit feedback");
    if feedback.clicked() {
        ui.ctx()
            .open_url(egui::OpenUrl::new_tab(updates::ISSUES_URL));
    }
}

/// The outcome of the last update check, under the title row.
///
/// This is deliberately inline rather than a modal: a check is a background
/// question, and its answer must not interrupt a running session. Nothing is
/// downloaded or installed; a newer release is shown as a link to its page.
fn update_status(app: &CallerApp, ui: &mut egui::Ui) {
    let muted = theme::muted(ui);
    match app.updates.status() {
        UpdateStatus::Idle => {}
        UpdateStatus::Checking => {
            ui.label(
                egui::RichText::new("Checking for updates…")
                    .small()
                    .color(muted),
            );
        }
        UpdateStatus::UpToDate { latest } => {
            ui.label(
                egui::RichText::new(format!("You are running the latest version ({latest})."))
                    .small()
                    .color(muted),
            );
        }
        UpdateStatus::Available { latest, url } => {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Version {latest} is available. This build is {}.",
                        updates::CURRENT_VERSION
                    ))
                    .small(),
                );
                ui.hyperlink_to(
                    egui::RichText::new("Open the release page")
                        .small()
                        .underline(),
                    url,
                );
            });
        }
        UpdateStatus::NoRelease => {
            ui.label(
                egui::RichText::new("No GitHub release has been published yet.")
                    .small()
                    .color(muted),
            );
        }
        UpdateStatus::Failed { message } => {
            ui.label(
                egui::RichText::new(format!("Could not check for updates: {message}"))
                    .small()
                    .color(theme::error(ui)),
            );
        }
    }
}

impl CallerApp {
    pub(in crate::app) fn preferences_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        let before = self.prefs.clone();
        // Recomputed by the custom role fields each frame. While a field is
        // focused the timeline rebuild is deferred; see `role_terms`.
        self.role_edit_active = false;
        preferences_header(self, ui, compact);
        ui.label(
            egui::RichText::new("Changes are saved automatically. Playback settings are applied without restarting the music; role and alias changes rebuild the call timeline.")
                .color(theme::muted(ui)),
        );
        ui.add_space(8.0);

        preferences_search_box(self, ui, compact);
        ui.add_space(4.0);

        if !self.prefs_query.trim().is_empty() {
            self.preferences_search_results(ui, compact);
        } else if compact {
            self.preferences_phone_ui(ui, compact);
        } else {
            self.preferences_desktop_ui(ui, compact);
        }

        // One place applies a change to the subsystem it belongs to; see
        // Preferences::changes_since. A value changed and changed back before
        // the queued render runs leaves no pending work.
        self.reconcile_preferences(before);
    }

    /// The settings that match the search field, across every category.
    ///
    /// Selecting a result switches Preferences to the category that owns it
    /// and asks that category to scroll the matching section into view. The
    /// query stays in the field while the list is shown, so typing keeps
    /// narrowing it; **Clear search** returns to the normal category layout.
    fn preferences_search_results(&mut self, ui: &mut egui::Ui, compact: bool) {
        let matches = matching_settings(&self.prefs_query);
        if matches.is_empty() {
            ui.add_space(8.0);
            ui.label(format!(
                "No settings match \"{}\".",
                self.prefs_query.trim()
            ));
            ui.label(
                egui::RichText::new("Try a word such as volume, tempo, theme, lead, or cache.")
                    .small()
                    .color(theme::muted(ui)),
            );
            return;
        }
        ui.label(
            egui::RichText::new(match matches.len() {
                1 => "1 setting".to_string(),
                count => format!("{count} settings"),
            })
            .small()
            .color(theme::muted(ui)),
        );
        egui::ScrollArea::vertical()
            .id_salt("preferences-search-results")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for setting in matches {
                    if setting_row(ui, setting, compact) {
                        self.prefs_page = PreferencesPage::Category(setting.category);
                        self.prefs_reveal = Some(setting.section);
                        self.prefs_query.clear();
                    }
                }
            });
    }

    /// Desktop Preferences: a category list on the left, settings on the right.
    fn preferences_desktop_ui(&mut self, ui: &mut egui::Ui, compact: bool) {
        let selected = self.prefs_page.selected();
        ui.horizontal_top(|ui| {
            let nav_height = ui.available_height();
            ui.allocate_ui_with_layout(
                egui::vec2(PREFERENCES_NAV_WIDTH, nav_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("preferences-categories")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for category in PreferencesCategory::ALL {
                                let response =
                                    ui.selectable_label(category == selected, category.label());
                                if response.clicked() {
                                    self.prefs_page = PreferencesPage::Category(category);
                                }
                            }
                        });
                },
            );
            ui.separator();
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("preferences-settings")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.heading(selected.label());
                        ui.label(
                            egui::RichText::new(selected.summary())
                                .small()
                                .color(theme::muted(ui)),
                        );
                        ui.add_space(4.0);
                        for section in SECTIONS
                            .iter()
                            .filter(|section| section.category == selected)
                        {
                            let response =
                                ui.push_id(section.anchor, |ui| (section.draw)(self, ui, compact));
                            if self.prefs_reveal == Some(section.anchor) {
                                ui.scroll_to_rect(response.response.rect, Some(egui::Align::TOP));
                                self.prefs_reveal = None;
                            }
                        }
                    });
            });
        });
    }

    /// Phone Preferences: the category list, then one category with a Back
    /// control. The Android system back button uses the same target.
    fn preferences_phone_ui(&mut self, ui: &mut egui::Ui, compact: bool) {
        match self.prefs_page {
            PreferencesPage::Categories => {
                for category in PreferencesCategory::ALL {
                    if category_row(ui, category) {
                        self.prefs_page = PreferencesPage::Category(category);
                    }
                }
            }
            PreferencesPage::Category(category) => {
                if ui
                    .add_sized(
                        [ui.available_width().min(200.0), theme::TOUCH_TARGET],
                        egui::Button::new("< Categories"),
                    )
                    .on_hover_text("Back to the settings categories (Esc)")
                    .accessible_name("Back to settings categories")
                    .clicked()
                {
                    self.prefs_page = PreferencesPage::Categories;
                }
                ui.heading(category.label());
                ui.label(
                    egui::RichText::new(category.summary())
                        .small()
                        .color(theme::muted(ui)),
                );
                egui::ScrollArea::vertical()
                    .id_salt("preferences-settings")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for section in SECTIONS
                            .iter()
                            .filter(|section| section.category == category)
                        {
                            let response =
                                ui.push_id(section.anchor, |ui| (section.draw)(self, ui, compact));
                            if self.prefs_reveal == Some(section.anchor) {
                                ui.scroll_to_rect(response.response.rect, Some(egui::Align::TOP));
                                self.prefs_reveal = None;
                            }
                        }
                    });
            }
        }
    }
}

/// The Preferences search field.
///
/// The visible label names the action; the accessible name is "Search
/// settings", so an agent can find the field without guessing the prompt.
fn preferences_search_box(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    if compact {
        ui.label(
            egui::RichText::new("Find a setting")
                .small()
                .color(theme::muted(ui)),
        );
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.prefs_query)
                .hint_text("Search by name, such as volume or lead")
                .desired_width(f32::INFINITY),
        );
        response.accessible_name("Search settings");
        if !app.prefs_query.is_empty()
            && ui
                .add_sized(
                    [ui.available_width(), theme::TOUCH_TARGET],
                    egui::Button::new("Clear search"),
                )
                .clicked()
        {
            app.prefs_query.clear();
        }
    } else {
        ui.horizontal(|ui| {
            ui.label("Find a setting");
            let response = ui.add(
                egui::TextEdit::singleline(&mut app.prefs_query)
                    .hint_text("Search by name, such as volume or lead")
                    .desired_width(PREFERENCES_SEARCH_WIDTH),
            );
            response.accessible_name("Search settings");
            if !app.prefs_query.is_empty() && ui.button("Clear search").clicked() {
                app.prefs_query.clear();
            }
        });
    }
}

/// Width of the desktop Preferences search field.
const PREFERENCES_SEARCH_WIDTH: f32 = 360.0;

/// Height of one search result row. A phone row leaves room for a title that
/// wraps to two lines above the category name.
const SEARCH_ROW_HEIGHT: f32 = 40.0;
const SEARCH_ROW_HEIGHT_COMPACT: f32 = 64.0;

/// Draw one tappable search result: the setting name over its category.
///
/// The row is painted rather than assembled from labels so the title wraps
/// within the row and the whole row stays one click target. Returns true when
/// the row is clicked or activated from the keyboard.
fn setting_row(ui: &mut egui::Ui, setting: &PreferenceSetting, compact: bool) -> bool {
    let width = ui.available_width();
    let height = if compact {
        SEARCH_ROW_HEIGHT_COMPACT
    } else {
        SEARCH_ROW_HEIGHT
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let visuals = ui.style().interact(&response);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect(
        rect,
        egui::CornerRadius::same(6),
        visuals.weak_bg_fill,
        if response.has_focus() {
            ui.visuals().selection.stroke
        } else {
            visuals.bg_stroke
        },
        egui::StrokeKind::Inside,
    );

    let left = rect.left() + PHONE_CATEGORY_INSET;
    let top = rect.top() + if compact { 8.0 } else { 4.0 };
    let wrap = (rect.width() - 2.0 * PHONE_CATEGORY_INSET).max(1.0);
    let title = painter.layout(
        setting.title.to_owned(),
        egui::FontId::proportional(15.0),
        visuals.text_color(),
        wrap,
    );
    let title_height = title.size().y;
    painter.galley(egui::pos2(left, top), title, visuals.text_color());
    painter.text(
        egui::pos2(left, top + title_height + 2.0),
        egui::Align2::LEFT_TOP,
        setting.category.label(),
        egui::FontId::proportional(12.0),
        theme::muted(ui),
    );

    let label = format!("{}. {}", setting.title, setting.category.label());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
        || (response.has_focus()
            && ui.input(|input| {
                input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space)
            }))
}

/// Width of the desktop Preferences category column.
const PREFERENCES_NAV_WIDTH: f32 = 170.0;

/// Height of one phone Preferences category row.
///
/// Every row uses this height and the same left inset, so the titles and
/// summaries line up in a single column instead of each short title drifting
/// toward the centre of its own button.
const PHONE_CATEGORY_ROW_HEIGHT: f32 = 60.0;

/// Horizontal inset of the title and summary inside a phone category row.
const PHONE_CATEGORY_INSET: f32 = 14.0;

/// Draw one tappable phone Preferences category row.
///
/// A plain `Button` centres each line independently, which makes a list of
/// two-line labels look ragged: "Calling" can start well right of "Band &
/// Sound". Painting the row ourselves keeps every title and summary on the
/// same left edge while preserving the touch target, hover/press feedback,
/// focus ring, and accessible name.
fn category_row(ui: &mut egui::Ui, category: PreferencesCategory) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, PHONE_CATEGORY_ROW_HEIGHT),
        egui::Sense::click(),
    );
    let visuals = ui.style().interact(&response);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect(
        rect,
        egui::CornerRadius::same(6),
        visuals.weak_bg_fill,
        if response.has_focus() {
            ui.visuals().selection.stroke
        } else {
            visuals.bg_stroke
        },
        egui::StrokeKind::Inside,
    );

    let left = rect.left() + PHONE_CATEGORY_INSET;
    let top = rect.top() + 8.0;
    painter.text(
        egui::pos2(left, top),
        egui::Align2::LEFT_TOP,
        category.label(),
        egui::FontId::proportional(15.0),
        visuals.text_color(),
    );
    let wrap = (rect.width() - 2.0 * PHONE_CATEGORY_INSET).max(1.0);
    let summary = painter.layout(
        category.summary().to_owned(),
        egui::FontId::proportional(13.0),
        theme::muted(ui),
        wrap,
    );
    painter.galley(egui::pos2(left, top + 20.0), summary, theme::muted(ui));

    let label = format!("{}. {}", category.label(), category.summary());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
        || (response.has_focus()
            && ui.input(|input| {
                input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space)
            }))
}

fn role_terms(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Role terms");
    // The stored selection, not an inference from the terms: choosing Custom
    // keeps the editable fields on screen even when the words still match a
    // named preset.
    let mut preset = app.prefs.role_preset;
    let combo = if compact {
        egui::ComboBox::from_id_salt("role-terms")
    } else {
        egui::ComboBox::from_label("Role terms")
    };
    responsive_combo(combo, ui, compact)
        .selected_text(preset.label())
        .show_ui(ui, |ui| {
            for option in RolePreset::ALL {
                ui.selectable_value(&mut preset, option, option.label());
            }
        });
    if preset != app.prefs.role_preset {
        app.prefs.select_role_preset(preset);
        // A named preset replaces the terms, so the change is complete now.
        // Custom keeps the current words and waits for an edit. Role terms
        // change the calls and the spoken text, never the music; the frame's
        // timeline reconciliation is the whole effect.
        app.role_edit_pending = false;
    }
    if app.prefs.role_preset == RolePreset::Custom {
        egui::Grid::new("custom-roles")
            .num_columns(if compact { 1 } else { 2 })
            .show(ui, |ui| {
                form_row(ui, compact, "Left singular", |ui| {
                    edit_role_term(app, ui, RoleField::LeftSingular);
                });
                ui.end_row();
                form_row(ui, compact, "Left plural", |ui| {
                    edit_role_term(app, ui, RoleField::LeftPlural);
                });
                ui.end_row();
                form_row(ui, compact, "Right singular", |ui| {
                    edit_role_term(app, ui, RoleField::RightSingular);
                });
                ui.end_row();
                form_row(ui, compact, "Right plural", |ui| {
                    edit_role_term(app, ui, RoleField::RightPlural);
                });
                ui.end_row();
            });
        // The example below and the next session frame read the edited terms
        // immediately, but the deferred rebuild waits until the field loses
        // focus so typing a long word does not rebuild the timeline and
        // restart speech on every keystroke.
        if app.role_edit_pending && !app.role_edit_active {
            app.commit_role_terms();
        }
    }
    ui.label(
        egui::RichText::new(
            app.prefs
                .role_terms
                .apply("ladles allemande left 1½; gentlespoons do si do once"),
        )
        .italics()
        .color(theme::muted(ui)),
    );
    if ui.button("Edit dance move aliases…").clicked() {
        app.screen = Screen::Aliases;
    }
    ui.separator();
}

/// One editable custom role term.
///
/// Editing a term always means Custom: it marks the selection, remembers that
/// a rebuild is pending, and lets the field lose focus before the timeline is
/// rebuilt. The active flag is cleared at the start of every preferences
/// frame, so a stale focus state cannot defer the rebuild forever.
fn edit_role_term(app: &mut CallerApp, ui: &mut egui::Ui, field: RoleField) {
    let terms = &mut app.prefs.role_terms;
    let buffer = match field {
        RoleField::LeftSingular => &mut terms.left_singular,
        RoleField::LeftPlural => &mut terms.left_plural,
        RoleField::RightSingular => &mut terms.right_singular,
        RoleField::RightPlural => &mut terms.right_plural,
    };
    let response = ui.text_edit_singleline(buffer);
    if response.changed() {
        app.prefs.role_preset = RolePreset::Custom;
        app.role_edit_pending = true;
        app.role_edit_active = true;
    }
    if response.has_focus() || response.lost_focus() {
        app.role_edit_active = true;
    }
    if response.lost_focus() {
        app.commit_role_terms();
    }
}

#[derive(Clone, Copy)]
enum RoleField {
    LeftSingular,
    LeftPlural,
    RightSingular,
    RightPlural,
}

fn instruments(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Instruments");
    ui.label(
        egui::RichText::new(
            "The band is a list. Add any number of instruments, choose what each one plays, \
             how many players stand in that section, and where it sits. The part chooses the \
             notes; the instrument chooses the voice. A second piano is its own row with its \
             own level and place, not a copy of the first.",
        )
        .small()
        .color(theme::muted(ui)),
    );

    ui.label(egui::RichText::new("Band").strong());
    let mut remove = None;
    let mut move_up = None;
    let mut move_down = None;
    let pitched_count = app
        .prefs
        .band
        .iter()
        .filter(|slot| slot.is_pitched())
        .count();
    let count = app.prefs.band.len();
    for (index, slot) in app.prefs.band.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{}. {} · {}",
                        index + 1,
                        slot.part.label(),
                        slot.voice.label()
                    ))
                    .strong(),
                );
                ui.label(
                    egui::RichText::new(slot.direction_label())
                        .small()
                        .color(theme::muted(ui)),
                );
            });
            if compact {
                ui.horizontal_wrapped(|ui| {
                    part_combo(ui, true, index, slot);
                    voice_combo(ui, true, index, slot);
                });
                ui.horizontal_wrapped(|ui| {
                    players_field(ui, true, index, slot);
                    direction_combo(ui, true, index, slot);
                });
                ui.horizontal(|ui| {
                    ui.label("Volume");
                    volume_slider(ui, compact, index, slot);
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    part_combo(ui, false, index, slot);
                    voice_combo(ui, false, index, slot);
                    players_field(ui, false, index, slot);
                    direction_combo(ui, false, index, slot);
                    ui.label("Volume");
                    volume_slider(ui, compact, index, slot);
                });
            }
            let mut add_actions = |ui: &mut egui::Ui, size: egui::Vec2| {
                let can_remove = if slot.is_pitched() {
                    pitched_count > 1
                } else {
                    count > 1
                };
                let remove_button = ui
                    .add_enabled(can_remove, egui::Button::new("Remove").min_size(size))
                    .on_hover_text(if can_remove {
                        "Remove this instrument from the band"
                    } else {
                        "A band needs at least one pitched instrument"
                    })
                    .accessible_name(format!("Remove instrument {}", index + 1));
                if remove_button.clicked() {
                    remove = Some(index);
                }
                let up = ui
                    .add_enabled(index > 0, egui::Button::new("Up").min_size(size))
                    .accessible_name(format!("Move instrument {} up", index + 1));
                if up.clicked() {
                    move_up = Some(index);
                }
                let down = ui
                    .add_enabled(index + 1 < count, egui::Button::new("Down").min_size(size))
                    .accessible_name(format!("Move instrument {} down", index + 1));
                if down.clicked() {
                    move_down = Some(index);
                }
            };
            if compact {
                // Every button in the instrument card's action row splits the
                // row, so the set leaves no empty space on the right.
                filled_button_row(ui, 3, |ui, width| {
                    add_actions(ui, egui::vec2(width, theme::TOUCH_TARGET));
                });
            } else {
                ui.horizontal_wrapped(|ui| add_actions(ui, egui::Vec2::ZERO));
            }
        });
    }
    if let Some(index) = remove {
        app.prefs.band.remove(index);
    }
    if let Some(index) = move_up
        && index > 0
    {
        app.prefs.band.swap(index, index - 1);
    }
    if let Some(index) = move_down
        && index + 1 < app.prefs.band.len()
    {
        app.prefs.band.swap(index, index + 1);
    }
    if ui
        .button("Add instrument")
        .on_hover_text("Add another instrument to the band")
        .accessible_name("Add instrument")
        .clicked()
    {
        app.prefs.band.push(BandSlot::default());
    }
    app.prefs.validate();

    ui.label(
        egui::RichText::new(
            "Percussion is optional: add a foot-tap or drumset row for a one-and-a gallop or a \
             simple kit. The rhythm menu changes the pattern; it starts with the count-in. \
             Removing every percussion row leaves the band silent of percussion.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.add_enabled_ui(false, |ui| {
        ui.checkbox(
            &mut app.prefs.dancer_sounds,
            "Dancer sounds (stomps and claps)",
        )
        .on_hover_text("Temporarily disabled while the dancer sound rhythms are corrected");
        slider_row(
            ui,
            compact,
            "Dancer sound volume",
            &mut app.prefs.dancer_sound_volume,
            |ui, value| number_slider(ui, egui::Slider::new(value, 0.0..=1.5)),
        );
    });
    ui.label(
        egui::RichText::new(
            "Dancer sounds are temporarily disabled while their rhythms are corrected. The music, \
             calls, and dance timing are unchanged.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    inline_field(ui, compact, "Percussion rhythm", |ui| {
        percussion_rhythm_combo(ui, compact, &mut app.prefs);
    });
    inline_field(ui, compact, "Accompaniment rhythm", |ui| {
        rhythm_combo(ui, compact, &mut app.prefs);
    });
    ui.label(
        egui::RichText::new(
            "The accompaniment rhythm changes how an accompaniment row realises the chords; it \
             never moves the dance grid or the calls.",
        )
        .small()
        .color(theme::muted(ui)),
    );

    ui.separator();
    ui.label(egui::RichText::new("Mix").strong());
    slider_row(
        ui,
        compact,
        "Caller volume",
        &mut app.prefs.caller_volume,
        |ui, value| number_slider(ui, egui::Slider::new(value, 0.0..=1.5)),
    );
    ui.label(
        egui::RichText::new(
            "The spoken calls have their own level. Turning the band down never hides a call, \
             and turning the caller down never changes the music.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    slider_row(
        ui,
        compact,
        "Band volume",
        &mut app.prefs.master_volume,
        |ui, value| number_slider(ui, egui::Slider::new(value, 0.0..=1.5)),
    );
    ui.label(
        egui::RichText::new(
            "The band master sits on top of each row's volume. Both levels apply immediately.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.separator();
}

fn part_combo(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    ui.label(format!("Part {}", index + 1))
        .on_hover_text("What this instrument plays");
    let mut part = slot.part;
    let combo = egui::ComboBox::from_id_salt(("band-part", index));
    responsive_combo(combo, ui, compact)
        .selected_text(part.label())
        .show_ui(ui, |ui| {
            for option in BandPart::ALL {
                ui.selectable_value(&mut part, option, option.label());
            }
        });
    if part != slot.part {
        slot.part = part;
        if part == BandPart::Percussion && !slot.voice.is_percussion() {
            slot.voice = BandVoice::FootTaps;
        } else if part != BandPart::Percussion && slot.voice.is_percussion() {
            slot.voice = BandVoice::Fiddle;
        }
    }
}

fn voice_combo(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    ui.label(format!("Instrument {}", index + 1))
        .on_hover_text("The voice this instrument uses");
    let percussion = slot.part == BandPart::Percussion;
    let combo = egui::ComboBox::from_id_salt(("band-voice", index));
    responsive_combo(combo, ui, compact)
        .selected_text(slot.voice.label())
        .show_ui(ui, |ui| {
            for option in BandVoice::ALL {
                if option.is_percussion() == percussion {
                    ui.selectable_value(&mut slot.voice, option, option.label());
                }
            }
        });
}

fn players_field(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    let max = if slot.voice.is_percussion() {
        BandSlot::MAX_PERCUSSION_PLAYERS
    } else {
        BandSlot::MAX_PLAYERS
    };
    ui.label("Players");
    players_stepper(ui, compact, index, &mut slot.players, max);
}

/// A players stepper whose buttons carry the instrument number, so an agent
/// can tell the rows apart in the widget tree.
fn players_stepper(ui: &mut egui::Ui, compact: bool, index: usize, players: &mut u8, max: u8) {
    let name = format!("instrument {}", index + 1);
    let fewer = format!("Fewer players on {name}");
    let more = format!("More players on {name}");
    let size = if compact {
        egui::vec2(theme::TOUCH_TARGET, theme::TOUCH_TARGET)
    } else {
        egui::vec2(28.0, 20.0)
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let decrease = ui
            .add_enabled(
                *players > BandSlot::MIN_PLAYERS,
                egui::Button::new("−").min_size(size),
            )
            .accessible_name(&fewer);
        if decrease.clicked() {
            *players = (*players).saturating_sub(1).max(BandSlot::MIN_PLAYERS);
        }
        ui.add_sized(
            [size.x, size.y],
            egui::Label::new(egui::RichText::new(players.to_string()).monospace()),
        )
        .accessible_name(format!("Players on {name}: {players}"));
        let increase = ui
            .add_enabled(*players < max, egui::Button::new("+").min_size(size))
            .accessible_name(&more);
        if increase.clicked() {
            *players = (*players + 1).min(max);
        }
    });
}

/// The six places a slot can sit in.
const DIRECTIONS: [(&str, f32, f32); 6] = [
    ("Left front", -0.6, 0.0),
    ("Centre front", 0.0, 0.0),
    ("Right front", 0.6, 0.0),
    ("Left rear", -0.6, 0.6),
    ("Centre rear", 0.0, 0.6),
    ("Right rear", 0.6, 0.6),
];

fn direction_combo(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    let combo = egui::ComboBox::from_id_salt(("band-direction", index));
    responsive_combo(combo, ui, compact)
        .selected_text(slot.direction_label())
        .show_ui(ui, |ui| {
            for (label, pan, depth) in DIRECTIONS {
                if ui
                    .selectable_label(slot.direction_label() == label, label)
                    .clicked()
                {
                    slot.pan = pan;
                    slot.depth = depth;
                }
            }
        });
    ui.label(format!("Direction {}", index + 1))
        .on_hover_text("Left/right and front/rear in the stereo or surround image");
}

fn volume_slider(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    if compact {
        ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(96.0);
    }
    number_slider(
        ui,
        egui::Slider::new(&mut slot.volume, 0.0..=1.5)
            .max_decimals(2)
            .show_value(true),
    )
    .accessible_name(format!("Volume for instrument {}", index + 1));
}

fn percussion_rhythm_combo(ui: &mut egui::Ui, compact: bool, prefs: &mut Preferences) {
    let combo = egui::ComboBox::from_id_salt("percussion_rhythm");
    responsive_combo(combo, ui, compact)
        .selected_text(prefs.percussion_rhythm.label())
        .show_ui(ui, |ui| {
            for option in PercussionRhythm::ALL {
                ui.selectable_value(&mut prefs.percussion_rhythm, option, option.label());
            }
        });
}

fn rhythm_combo(ui: &mut egui::Ui, compact: bool, prefs: &mut Preferences) {
    let combo = egui::ComboBox::from_id_salt("accompaniment_rhythm");
    responsive_combo(combo, ui, compact)
        .selected_text(prefs.accompaniment_rhythm.label())
        .show_ui(ui, |ui| {
            for option in AccompanimentRhythm::ALL {
                ui.selectable_value(&mut prefs.accompaniment_rhythm, option, option.label());
            }
        });
}

fn appearance(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Theme");
    if compact {
        // A set of buttons on one phone line splits the width so the row ends
        // at the content edge.
        filled_button_row(ui, Theme::ALL.len(), |ui, width| {
            for option in Theme::ALL {
                let selected = app.prefs.theme == option;
                if ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::selectable(selected, option.label()),
                    )
                    .clicked()
                {
                    app.prefs.theme = option;
                }
            }
        });
    } else {
        ui.horizontal(|ui| {
            for option in Theme::ALL {
                ui.selectable_value(&mut app.prefs.theme, option, option.label());
            }
        });
    }
    ui.heading("Interface");
    ui.checkbox(&mut app.prefs.animations_enabled, "Smooth animations");
    ui.label(
                egui::RichText::new(
                    "Fade and slide between screens and animate tabs and highlights. Turn off to reduce motion.",
                )
                .small()
                .color(theme::muted(ui)),
            );
    form_row(ui, compact, "Dance card padding", |ui| {
        if compact {
            filled_button_row(ui, CardPadding::ALL.len(), |ui, width| {
                for option in CardPadding::ALL {
                    let selected = app.prefs.card_padding == option;
                    if ui
                        .add_sized(
                            [width, theme::TOUCH_TARGET],
                            egui::Button::selectable(selected, option.label()),
                        )
                        .clicked()
                    {
                        app.prefs.card_padding = option;
                    }
                }
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                for option in CardPadding::ALL {
                    ui.selectable_value(&mut app.prefs.card_padding, option, option.label());
                }
            });
        }
    });
    ui.label(
        egui::RichText::new(
            "Compact packs the figures in the session card and previews closer together; Sparse keeps the roomier rows.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.checkbox(&mut app.prefs.glossary_enabled, "Show dance glossary");
    ui.label(
        egui::RichText::new(
            "Underline dance terms in calls and cards, show their definition tooltips, and keep the Glossary tab and Dance card button. Turning this off removes all of them.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    undo_redo_buttons(app, ui, compact);
    if let Some(message) = &app.history_status {
        ui.label(egui::RichText::new(message).small().color(theme::muted(ui)));
    }
    ui.label(
                egui::RichText::new(
                    "Keyboard: Space play/pause · R restart · Esc back · 1-4 switch tabs · 5 opens Preferences · ←/→ previous or next tab · Ctrl+Z undo · Ctrl+Y or Ctrl+Shift+Z redo · Enter starts a search, creates a folder, or adds an alias.",
                )
                .small()
                .color(theme::muted(ui)),
            );
    ui.separator();
}

/// Undo and redo buttons for the last preference or picker change.
///
/// The keyboard chords do the same thing; the buttons exist so the feature is
/// discoverable and works on a touch screen, where there is no Ctrl key. Both
/// carry an accessible name and are disabled when their stack is empty.
fn undo_redo_buttons(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    let mut undo = false;
    let mut redo = false;
    let mut add = |ui: &mut egui::Ui, undo_size: egui::Vec2, redo_size: egui::Vec2| {
        let undo_response = ui
            .add_enabled(
                app.history.can_undo(),
                egui::Button::new("Undo").min_size(undo_size),
            )
            .on_hover_text("Undo the last change (Ctrl+Z)")
            .accessible_name("Undo");
        undo = undo_response.clicked();
        let redo_response = ui
            .add_enabled(
                app.history.can_redo(),
                egui::Button::new("Redo").min_size(redo_size),
            )
            .on_hover_text("Redo the change (Ctrl+Y or Ctrl+Shift+Z)")
            .accessible_name("Redo");
        redo = redo_response.clicked();
    };
    if compact {
        // Both buttons split the phone row so the pair ends at the content
        // edge instead of leaving empty space on the right.
        filled_button_row(ui, 2, |ui, width| {
            let size = egui::vec2(width, theme::TOUCH_TARGET);
            add(ui, size, size);
        });
    } else {
        let size = egui::vec2(96.0, 24.0);
        ui.horizontal_wrapped(|ui| add(ui, size, size));
    }
    if undo {
        app.pending_history = Some(HistoryAction::Undo);
    }
    if redo {
        app.pending_history = Some(HistoryAction::Redo);
    }
}

fn calling_lead(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Calling lead");
    ui.checkbox(&mut app.prefs.lead.auto, "Automatic syllable-based lead");
    if app.prefs.lead.auto {
        slider_row(
            ui,
            compact,
            "Minimum speech lead",
            &mut app.prefs.lead.min_beats,
            |ui, value| number_slider(ui, egui::Slider::new(value, 0.5..=4.0).text("beats")),
        );
        slider_row(
            ui,
            compact,
            "Maximum speech lead",
            &mut app.prefs.lead.max_beats,
            |ui, value| number_slider(ui, egui::Slider::new(value, 1.0..=6.0).text("beats")),
        );
        slider_row(
            ui,
            compact,
            "Speech rate",
            &mut app.prefs.lead.syllables_per_second,
            |ui, value| number_slider(ui, egui::Slider::new(value, 3.0..=10.0).text("syllables/s")),
        );
    } else {
        slider_row(
            ui,
            compact,
            "Manual speech lead",
            &mut app.prefs.lead.manual_beats,
            |ui, value| number_slider(ui, egui::Slider::new(value, 0.5..=6.0).text("beats")),
        );
    }
    slider_row(
        ui,
        compact,
        "Safety padding",
        &mut app.prefs.lead.padding_seconds,
        |ui, value| {
            number_slider(
                ui,
                egui::Slider::new(
                    value,
                    caller_core::MIN_PADDING_SECONDS..=caller_core::MAX_PADDING_SECONDS,
                )
                .step_by(0.1)
                .fixed_decimals(1)
                .text("s"),
            )
        },
    );
    ui.label(
        egui::RichText::new(
            "Added to every call after the speech-lead clamp. Zero removes the buffer.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    if ui.button("Rebuild call timing").clicked() {
        app.rebuild_timeline();
        app.needs_render = true;
        app.save_prefs();
    }
    ui.separator();
}

fn speech(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Text to speech");
    if ui
        .checkbox(&mut app.prefs.tts_enabled, "Speak calls")
        .changed()
    {
        app.speech.set_enabled(app.prefs.tts_enabled);
        if !app.prefs.tts_enabled {
            app.speech.stop();
        }
    }
    ui.label(
        egui::RichText::new(
            "Desktop and Android builds use the local Kokoro neural voice and download the \
                     model and selected voice once, then cache them. Web uses the platform \
                     voice; Android falls back to it if the neural runtime is unavailable.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let neural = crate::platform() != "web";
    inline_field(ui, compact, "Voice", |ui| {
        ui.add_enabled_ui(neural, |ui| {
            let selected = crate::tts::voice(&app.prefs.tts_voice);
            let combo = egui::ComboBox::from_id_salt("tts_voice");
            responsive_combo(combo, ui, compact)
                .selected_text(selected.label)
                .show_ui(ui, |ui| {
                    for voice in crate::tts::VOICES {
                        if ui
                            .selectable_label(app.prefs.tts_voice == voice.id, voice.label)
                            .clicked()
                        {
                            app.prefs.tts_voice = voice.id.to_string();
                            app.speech.set_voice(&app.prefs.tts_voice);
                        }
                    }
                });
        });
    });
    let selected = crate::tts::voice(&app.prefs.tts_voice);
    ui.label(
        egui::RichText::new(if neural {
            selected.detail
        } else {
            "The platform voice is used in this build."
        })
        .small()
        .color(theme::muted(ui)),
    );
    let rate_response = slider_row(ui, compact, "Rate", &mut app.prefs.tts_rate, |ui, rate| {
        number_slider(
            ui,
            egui::Slider::new(rate, caller_core::MIN_TTS_RATE..=caller_core::MAX_TTS_RATE)
                .step_by(0.1)
                .fixed_decimals(1),
        )
    });
    if rate_response.changed() {
        app.speech.set_rate(app.prefs.speech_rate());
    }
    ui.horizontal(|ui| {
        ui.label("Tone");
        egui::ComboBox::from_id_salt("tts_tone")
            .selected_text(app.prefs.tts_tone.label())
            .show_ui(ui, |ui| {
                for tone in caller_core::CallTone::ALL {
                    ui.selectable_value(&mut app.prefs.tts_tone, tone, tone.label());
                }
            });
    });
    ui.label(
        egui::RichText::new(app.prefs.tts_tone.detail())
            .small()
            .color(theme::muted(ui)),
    );
    ui.add_enabled_ui(neural, |ui| {
        slider_row(
            ui,
            compact,
            "Deliveries per call",
            &mut app.prefs.tts_deliveries,
            |ui, deliveries| {
                number_slider(
                    ui,
                    egui::Slider::new(
                        deliveries,
                        caller_core::MIN_TTS_DELIVERIES..=caller_core::MAX_TTS_DELIVERIES,
                    )
                    .integer(),
                )
                .accessible_name("Deliveries per call")
            },
        );
    });
    ui.label(
        egui::RichText::new(
            "Prepares up to four versions of every call, each a little higher or lower, and \
             rotates them when a line repeats, so a looping dance does not sound like one \
             recording. The first version is ready before the rest, which prepare quietly in \
             the background. More deliveries add preparation time and memory; 1 keeps a \
             single version. Desktop and Android neural voice only.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.checkbox(
        &mut app.prefs.shuffle_alternatives,
        "Shuffle alternative call aliases",
    );
    ui.label(
                egui::RichText::new(
                    "Randomly uses an alternative wording, such as \"partner swing\" instead of \"swing your partner\", for moves that have one. The chosen wording is shown and spoken, and its timing is used.",
                )
                .small()
                .color(theme::muted(ui)),
            );
    let status = app.speech.status().clone();
    if let Some(progress) = status.progress {
        ui.add(
            egui::ProgressBar::new(progress)
                .show_percentage()
                .text(status.message.clone()),
        );
    } else {
        let color = if status.error.is_some() {
            theme::error(ui)
        } else {
            theme::muted(ui)
        };
        ui.label(egui::RichText::new(&status.message).small().color(color));
    }
    if let Some(error) = &status.error {
        ui.label(
            egui::RichText::new(format!("Using the platform voice: {error}"))
                .small()
                .color(theme::muted(ui)),
        );
    } else if !app.speech.available()
        && let Some(error) = app.speech.error()
    {
        ui.label(
            egui::RichText::new(format!("TTS unavailable: {error}"))
                .small()
                .color(theme::muted(ui)),
        );
    }
    if status.error.is_some() && app.speech.can_retry() && ui.button("Retry neural voice").clicked()
    {
        app.speech.retry();
    }
}

fn playback(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.checkbox(&mut app.prefs.loop_enabled, "Loop the dance and music");
    ui.checkbox(&mut app.prefs.count_in_enabled, "Four-beat count-in");
    ui.checkbox(
        &mut app.prefs.accompaniment_variations,
        "Vary the music on each loop",
    );
    ui.label(
        egui::RichText::new(
            "Adds melody ornaments, emphasis notes, and the occasional bar with melody alone.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let variations = app.prefs.accompaniment_variations;
    ui.add_enabled_ui(variations, |ui| {
        ui.indent("variation_options", |ui| {
            let options = &mut app.prefs.variation_options;
            ui.checkbox(&mut options.articulation, "Vary note lengths");
            ui.checkbox(&mut options.double_stops, "Add chord-tone double stops");
            ui.checkbox(&mut options.grace_notes, "Add grace notes");
            ui.checkbox(&mut options.octave_lift, "Lift held notes an octave");
            ui.checkbox(&mut options.chord_stabs, "Add chord stabs");
            ui.checkbox(
                &mut options.melody_alone_bar,
                "Let the melody play a bar alone",
            );
        });
    });
    ui.add_space(4.0);
    ui.label("Fiddle bowing");
    ui.label(
        egui::RichText::new(
            "Shape the melody and countermelody as a fiddler would. Both rules follow the written \
             rhythm, so no note moves off the beat.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.checkbox(
        &mut app.prefs.bowing_options.bow_change,
        "Accent and re-strike the bow-change figure",
    );
    ui.checkbox(
        &mut app.prefs.bowing_options.tied_runs,
        "Tie four to eight running eighths",
    );
    ui.checkbox(&mut app.prefs.show_original, "Show original card text");
    ui.checkbox(&mut app.prefs.humanize_timing, "Humanize the band");
    ui.label(
        egui::RichText::new(
            "Each player drifts by a fraction of a percent and a controller gently settles it back on \
             the tempo; chord notes start a few milliseconds apart; a fiddle's fingers land a few \
             cents off the nominal pitch. The count-in, the 64-beat grid, and the calls stay exact.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let humanize = app.prefs.humanize_timing;
    ui.add_enabled_ui(humanize, |ui| {
        ui.indent("humanize_options", |ui| {
            let options = &mut app.prefs.humanize_options;
            ui.checkbox(&mut options.tempo_drift, "Tempo drift per player");
            ui.checkbox(&mut options.chord_spread, "Spread chord notes");
            ui.checkbox(&mut options.pitch_drift, "Pitch drift (unfretted strings)");
            slider_row(
                ui,
                compact,
                "Timing amount",
                &mut options.amount,
                |ui, value| {
                    number_slider(
                        ui,
                        egui::Slider::new(value, 0.0..=MAX_HUMANIZE_AMOUNT).fixed_decimals(2),
                    )
                    .accessible_name("Timing amount")
                },
            );
            slider_row(
                ui,
                compact,
                "Tuning amount",
                &mut options.pitch_amount,
                |ui, value| {
                    number_slider(
                        ui,
                        egui::Slider::new(value, 0.0..=MAX_HUMANIZE_AMOUNT).fixed_decimals(2),
                    )
                    .accessible_name("Tuning amount")
                },
            );
            ui.label(
                egui::RichText::new(
                    "One is the calibrated amount for each control: timing scales the drift and \
                     the chord spread, tuning scales the fiddle's intonation. The bounds scale \
                     with each slider, so both stay even rather than saturating.",
                )
                .small()
                .color(theme::muted(ui)),
            );
            ui.add_space(4.0);
            ui.label("Tempo controller");
            ui.label(
                egui::RichText::new(
                    "The controller pulls each player's accumulated timing error back to the \
                     beat. The defaults are 0.5, 0.05, and 0.02. Raise a gain to correct sooner, \
                     or lower it to let a player wander for longer.",
                )
                .small()
                .color(theme::muted(ui)),
            );
            slider_row(
                ui,
                compact,
                "Proportional gain (Kp)",
                &mut options.pid_kp,
                |ui, value| {
                    number_slider(
                        ui,
                        egui::Slider::new(value, 0.0..=MAX_PID_KP).fixed_decimals(2),
                    )
                    .accessible_name("Proportional gain (Kp)")
                },
            );
            slider_row(
                ui,
                compact,
                "Integral gain (Ki)",
                &mut options.pid_ki,
                |ui, value| {
                    number_slider(
                        ui,
                        egui::Slider::new(value, 0.0..=MAX_PID_KI).fixed_decimals(3),
                    )
                    .accessible_name("Integral gain (Ki)")
                },
            );
            slider_row(
                ui,
                compact,
                "Derivative gain (Kd)",
                &mut options.pid_kd,
                |ui, value| {
                    number_slider(
                        ui,
                        egui::Slider::new(value, 0.0..=MAX_PID_KD).fixed_decimals(3),
                    )
                    .accessible_name("Derivative gain (Kd)")
                },
            );
        });
    });
    ui.checkbox(&mut app.prefs.ending_options.gradual, "Gradual ending");
    ui.checkbox(
        &mut app.prefs.ending_options.final_chord_down_fifth,
        "Resolve the final chord a fifth lower",
    );
    ui.label(
        egui::RichText::new(
            "The last four dance beats ease and thin into a held final chord. Turning gradual ending \
             off replaces only the final bar. The final chord and its closing melody note are a \
             perfect fifth below the written ones unless the second option is off.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.heading("Output");
    inline_field(ui, compact, "Output mode", |ui| {
        let combo = egui::ComboBox::from_id_salt("output_mode");
        responsive_combo(combo, ui, compact)
            .selected_text(app.prefs.output_mode.label())
            .show_ui(ui, |ui| {
                for option in OutputMode::ALL {
                    ui.selectable_value(&mut app.prefs.output_mode, option, option.label());
                }
            });
    });
    ui.label(
        egui::RichText::new(
            "Mono folds the band to one channel. Stereo is the default left/right pair. \
             Surround renders five channels: front left, front right, centre, rear left, rear \
             right. An instrument's direction sets its side and whether it sits at the front or \
             behind the dancers; a surround render played on a stereo device is folded down, and a \
             mono or stereo render on a multichannel device keeps its channels and leaves the \
             extras silent.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.separator();
}

/// Clamp a tempo window to the global limits and keep it at least one BPM wide.
///
/// The two controls are edited independently, so the helper is what turns an
/// out-of-range or inverted pair into a usable slider range without silently
/// snapping the caller's current tempo.
fn normalize_tempo_range(min_bpm: &mut f64, max_bpm: &mut f64) {
    *min_bpm = min_bpm.clamp(caller_core::Tempo::MIN_BPM, caller_core::Tempo::MAX_BPM);
    *max_bpm = max_bpm.clamp(caller_core::Tempo::MIN_BPM, caller_core::Tempo::MAX_BPM);
    if *min_bpm > *max_bpm {
        std::mem::swap(min_bpm, max_bpm);
    }
    if *min_bpm >= *max_bpm {
        if *max_bpm >= caller_core::Tempo::MAX_BPM {
            *max_bpm = caller_core::Tempo::MAX_BPM;
            *min_bpm = caller_core::Tempo::MAX_BPM - 1.0;
        } else {
            *max_bpm = *min_bpm + 1.0;
        }
    }
}

/// The session tempo slider's default window.
const DEFAULT_TEMPO_MIN_BPM: f64 = 200.0;
const DEFAULT_TEMPO_MAX_BPM: f64 = 300.0;

fn tempo_range(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Tempo range");
    ui.label(
        egui::RichText::new(
            "The session tempo slider starts at this window, 200–300 BPM by default. \
             The full range is 120–320 BPM, and a tempo outside the window stays selectable.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let mut min = app.prefs.tempo_min_bpm;
    let mut max = app.prefs.tempo_max_bpm;
    inline_field(ui, compact, "Minimum", |ui| {
        ui.add(
            egui::DragValue::new(&mut min)
                .range(caller_core::Tempo::MIN_BPM..=caller_core::Tempo::MAX_BPM)
                .speed(1.0)
                .suffix(" BPM")
                // A typed number applies when editing finishes. Parsing every
                // digit would clamp a partial "1" to the minimum and let
                // `normalize_tempo_range` swap the pair while the user is still
                // typing, rewriting the text under the keyboard.
                .update_while_editing(false),
        )
        .on_hover_text("Lowest tempo on the session slider")
        .accessible_name("Minimum tempo");
    });
    inline_field(ui, compact, "Maximum", |ui| {
        ui.add(
            egui::DragValue::new(&mut max)
                .range(caller_core::Tempo::MIN_BPM..=caller_core::Tempo::MAX_BPM)
                .speed(1.0)
                .suffix(" BPM")
                .update_while_editing(false),
        )
        .on_hover_text("Highest tempo on the session slider")
        .accessible_name("Maximum tempo");
    });
    normalize_tempo_range(&mut min, &mut max);
    if min != app.prefs.tempo_min_bpm || max != app.prefs.tempo_max_bpm {
        app.prefs.tempo_min_bpm = min;
        app.prefs.tempo_max_bpm = max;
    }
    if ui.button("Reset to 200–300").clicked() {
        app.prefs.tempo_min_bpm = DEFAULT_TEMPO_MIN_BPM;
        app.prefs.tempo_max_bpm = DEFAULT_TEMPO_MAX_BPM;
    }
    ui.separator();
}

fn soundfont(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Soundfont");
    let current = app
        .prefs
        .soundfont
        .as_ref()
        .map(|setting| setting.name.clone())
        .unwrap_or_else(|| "Bundled: Fluid R3 GM".into());
    ui.label(current);
    ui.label(
        egui::RichText::new(
            "Sampled instruments from Fluid R3 GM (MIT, Frank Wen). Load an .sf2 to use a different bank.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    if let Some(error) = &app.soundfont_error {
        ui.colored_label(theme::error(ui), error);
    }
    if compact {
        // The visible actions split the phone row, so the set ends at the
        // content edge whether or not a user font can be removed.
        let buttons = if app.prefs.soundfont.is_some() { 2 } else { 1 };
        let mut load = false;
        let mut use_bundled = false;
        filled_button_row(ui, buttons, |ui, width| {
            let size = egui::vec2(width, theme::TOUCH_TARGET);
            load = ui
                .add_sized(size, egui::Button::new("Load .sf2…"))
                .clicked();
            if app.prefs.soundfont.is_some() {
                use_bundled = ui
                    .add_sized(size, egui::Button::new("Use bundled"))
                    .clicked();
            }
        });
        if load {
            app.load_soundfont();
        }
        if use_bundled {
            app.remove_soundfont();
        }
    } else {
        ui.horizontal(|ui| {
            if ui.button("Load .sf2…").clicked() {
                app.load_soundfont();
            }
            if app.prefs.soundfont.is_some() && ui.button("Use bundled").clicked() {
                app.remove_soundfont();
            }
        });
    }
    ui.separator();
}

fn effects(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Effects");
    ui.label(
        egui::RichText::new(
            "Applied to playback without restarting the music, and written into exported MIDI \
             files. Exported MIDI carries General MIDI sends, so its sound also depends on the player.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    inline_field(ui, compact, "Reverb", |ui| {
        let combo = egui::ComboBox::from_id_salt("effect_reverb");
        responsive_combo(combo, ui, compact)
            .selected_text(app.prefs.effects.reverb.label())
            .show_ui(ui, |ui| {
                for option in ReverbPreset::ALL {
                    ui.selectable_value(&mut app.prefs.effects.reverb, option, option.label());
                }
            });
    });
    ui.label(
        egui::RichText::new(app.prefs.effects.reverb.detail())
            .small()
            .color(theme::muted(ui)),
    );
    inline_field(ui, compact, "Echo", |ui| {
        let combo = egui::ComboBox::from_id_salt("effect_echo");
        responsive_combo(combo, ui, compact)
            .selected_text(app.prefs.effects.echo.label())
            .show_ui(ui, |ui| {
                for option in EchoPreset::ALL {
                    ui.selectable_value(&mut app.prefs.effects.echo, option, option.label());
                }
            });
    });
    ui.label(
        egui::RichText::new(
            "Subtle is the default: a short slapback that widens the band without smearing the \
             dance pulse. Strong is a longer echo; Off is completely dry. Exported MIDI uses the \
             General MIDI delay send; players that do not implement it ignore the echo.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    inline_field(ui, compact, "Chorus", |ui| {
        let combo = egui::ComboBox::from_id_salt("effect_chorus");
        responsive_combo(combo, ui, compact)
            .selected_text(app.prefs.effects.chorus.label())
            .show_ui(ui, |ui| {
                for option in ChorusPreset::ALL {
                    ui.selectable_value(&mut app.prefs.effects.chorus, option, option.label());
                }
            });
    });
    ui.label(
        egui::RichText::new("A light shimmer or a wide chorus on playback and the exported file.")
            .small()
            .color(theme::muted(ui)),
    );
    ui.separator();
}

/// Where the **Download dance** source lives, and where the cache is.
///
/// The source menu moved to the Download dance page, next to the search form it
/// controls. Saved dances keep their own source, so switching it there never
/// rewrites the library; it only changes where the next download comes from.
fn data_sources(app: &mut CallerApp, ui: &mut egui::Ui, _compact: bool) {
    ui.heading("Data sources");
    ui.label(
        egui::RichText::new(
            "Choose the search source for new downloads on the Download dance page, next to \
             the search form. Saved dances keep the source they came from, and their Open on… \
             link returns there.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let selected = app.dance_source();
    // An id from an older or newer build is replaced with the provider this
    // build actually resolved, so the file and the menu agree.
    if app.prefs.dance_source != selected.id() {
        app.prefs.dance_source = selected.id().to_string();
    }
    ui.label(
        egui::RichText::new(format!("Current source: {}", selected.display_name()))
            .small()
            .color(theme::muted(ui)),
    );
    ui.hyperlink_to("Open the source site", selected.base_url());
    ui.separator();
}

fn web_proxy(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Web data proxy");
    ui.label(
        egui::RichText::new(
            "Only needed by the web build. Desktop and Android fetch sources directly.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let mut proxy = app.prefs.web_proxy_url.clone().unwrap_or_default();
    let mut token = app.prefs.web_proxy_token.clone().unwrap_or_default();
    let mut proxy_changed = false;
    egui::Grid::new("proxy")
        .num_columns(if compact { 1 } else { 2 })
        .show(ui, |ui| {
            form_row(ui, compact, "Proxy URL", |ui| {
                if ui.text_edit_singleline(&mut proxy).changed() {
                    proxy_changed = true;
                }
            });
            ui.end_row();
            form_row(ui, compact, "Token", |ui| {
                if ui.text_edit_singleline(&mut token).changed() {
                    proxy_changed = true;
                }
            });
            ui.end_row();
        });
    if proxy_changed {
        app.prefs.web_proxy_url = (!proxy.trim().is_empty()).then_some(proxy);
        app.prefs.web_proxy_token = (!token.trim().is_empty()).then_some(token);
        app.source.set_proxy(
            app.prefs.web_proxy_url.clone(),
            app.prefs.web_proxy_token.clone(),
        );
        app.save_prefs();
    }
    ui.separator();
}

fn maintenance(app: &mut CallerApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Maintenance");
    let buttons = if app.cache_clear_confirm { 3 } else { 2 };
    if compact {
        // The three maintenance actions split one phone row, so it ends at the
        // content edge.
        filled_button_row(ui, buttons, |ui, width| {
            let size = egui::vec2(width, theme::TOUCH_TARGET);
            if ui
                .add_sized(size, egui::Button::new("Export MIDI…"))
                .clicked()
            {
                app.export_midi();
            }
            if app.cache_clear_confirm {
                if ui
                    .add_sized(size, egui::Button::new("Confirm clear cache").wrap())
                    .clicked()
                {
                    app.source.clear_cache();
                    app.cache_clear_confirm = false;
                    app.status = "Cache cleared".into();
                }
                if ui.add_sized(size, egui::Button::new("Cancel")).clicked() {
                    app.cache_clear_confirm = false;
                }
            } else if ui
                .add_sized(size, egui::Button::new("Clear source cache").wrap())
                .clicked()
            {
                app.cache_clear_confirm = true;
            }
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Export MIDI…").clicked() {
                app.export_midi();
            }
            if app.cache_clear_confirm {
                if ui.button("Confirm clear cache").clicked() {
                    app.source.clear_cache();
                    app.cache_clear_confirm = false;
                    app.status = "Cache cleared".into();
                }
                if ui.button("Cancel").clicked() {
                    app.cache_clear_confirm = false;
                }
            } else if ui.button("Clear source cache").clicked() {
                app.cache_clear_confirm = true;
            }
        });
    }
    ui.label(
        egui::RichText::new(&app.status)
            .small()
            .color(theme::muted(ui)),
    );
    ui.add_space(8.0);
    memory_use(app, ui);
}

/// The measured memory table, with an explicit refresh.
///
/// The report walks the libraries and buffers, so it is built on demand rather
/// than every frame. It is a diagnostic: nothing in the session depends on it.
fn memory_use(app: &mut CallerApp, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Memory use").strong());
    ui.label(
        egui::RichText::new(
            "Measured and estimated resident memory by category. Numbers vary by platform and \
             by which optional features have loaded.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let mut refresh = app.memory_report.is_none();
    if ui.button("Refresh memory use").clicked() {
        refresh = true;
    }
    if refresh {
        let ctx = ui.ctx().clone();
        app.memory_report = Some(app.memory_report(Some(&ctx)));
    }
    if let Some(report) = &app.memory_report {
        let mut total = 0u64;
        // One category per block rather than a two-column grid: at 360 points
        // the category name and the long source note would otherwise force the
        // grid wider than the page.
        for (name, bytes, estimated, note) in report.rows() {
            total = total.saturating_add(bytes);
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(name.replace('_', " ")).small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} MiB{}",
                        crate::memory::mebibytes(bytes),
                        if estimated { " (est.)" } else { "" }
                    ))
                    .small()
                    .color(theme::muted(ui)),
                );
            });
            if !note.is_empty() {
                ui.label(egui::RichText::new(note).small().color(theme::muted(ui)));
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Total").strong());
            ui.label(
                egui::RichText::new(format!("{:.1} MiB", crate::memory::mebibytes(total))).strong(),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tempo_range_is_validated_and_inverted_pairs_are_read_the_same() {
        let (mut min, mut max) = (200.0, 300.0);
        normalize_tempo_range(&mut min, &mut max);
        assert_eq!((min, max), (200.0, 300.0));

        // Out of the global limits.
        (min, max) = (50.0, 900.0);
        normalize_tempo_range(&mut min, &mut max);
        assert_eq!((min, max), (120.0, 320.0));

        // Inverted input becomes the same usable window.
        (min, max) = (300.0, 200.0);
        normalize_tempo_range(&mut min, &mut max);
        assert_eq!((min, max), (200.0, 300.0));

        // Equal values keep a one-BPM window instead of a one-point slider.
        (min, max) = (250.0, 250.0);
        normalize_tempo_range(&mut min, &mut max);
        assert_eq!((min, max), (250.0, 251.0));
        (min, max) = (320.0, 320.0);
        normalize_tempo_range(&mut min, &mut max);
        assert_eq!((min, max), (319.0, 320.0));
    }

    #[test]
    fn the_reset_action_restores_the_default_tempo_window() {
        assert_eq!(
            (DEFAULT_TEMPO_MIN_BPM, DEFAULT_TEMPO_MAX_BPM),
            (200.0, 300.0)
        );
    }

    #[test]
    fn a_phone_drills_into_a_category_and_back_to_the_list() {
        let page = PreferencesPage::Categories;
        assert_eq!(page.back_target(true), None);
        assert_eq!(page.back_target(false), None);

        let page = PreferencesPage::Category(PreferencesCategory::Playback);
        assert_eq!(page.back_target(true), Some(PreferencesPage::Categories));
        // A desktop shows the list beside the settings, so it has nothing to
        // step back to inside Preferences.
        assert_eq!(page.back_target(false), None);
        assert_eq!(page.selected(), PreferencesCategory::Playback);
        assert_eq!(
            PreferencesPage::Categories.selected(),
            PreferencesCategory::Calling
        );
    }

    #[test]
    fn the_search_matches_titles_synonyms_and_every_word() {
        let titles = |query: &str| -> Vec<&'static str> {
            matching_settings(query)
                .into_iter()
                .map(|setting| setting.title)
                .collect()
        };

        assert!(titles("volume").contains(&"Caller volume"));
        assert!(titles("volume").contains(&"Band volume"));
        // A synonym that is not in the title finds the same setting.
        assert!(titles("bpm").contains(&"Tempo range"));
        assert!(titles("tts").contains(&"Speak calls"));
        // Every word must match, not just one of them. "band volume" still
        // matches the other band levels because "band" is one of their
        // keywords, so use a narrower pair to prove the AND.
        assert_eq!(titles("caller volume"), vec!["Caller volume"]);
        assert!(titles("band nothing").is_empty());
        // Case and punctuation around the query do not matter.
        assert_eq!(titles("  Volume  "), titles("volume"));
        assert!(matching_settings("   ").is_empty());
        assert!(titles("no such setting").is_empty());
    }

    #[test]
    fn the_search_lists_the_whole_band_volume_set() {
        let titles: Vec<&str> = matching_settings("volume")
            .into_iter()
            .map(|setting| setting.title)
            .collect();
        for expected in [
            "Instrument volume",
            "Dancer sound volume",
            "Caller volume",
            "Band volume",
        ] {
            assert!(titles.contains(&expected), "{expected} is not searchable");
        }
    }

    #[test]
    fn every_searchable_setting_points_at_a_real_section() {
        for setting in SETTINGS {
            let section = SECTIONS
                .iter()
                .find(|section| section.anchor == setting.section)
                .unwrap_or_else(|| {
                    panic!(
                        "{} points at unknown section {}",
                        setting.title, setting.section
                    )
                });
            assert_eq!(
                section.category,
                setting.category,
                "{} is listed under {} but drawn in {}",
                setting.title,
                setting.category.label(),
                section.category.label()
            );
        }
        // A section with no individual control still needs one way in, so the
        // search can never land on a category with nothing to reveal.
        for section in SECTIONS {
            assert!(
                SETTINGS
                    .iter()
                    .any(|setting| setting.section == section.anchor),
                "{} has no searchable setting",
                section.anchor
            );
        }
    }

    #[test]
    fn every_category_owns_at_least_one_section_and_about_is_last() {
        for category in PreferencesCategory::ALL {
            assert!(
                SECTIONS.iter().any(|section| section.category == category),
                "{} has no settings",
                category.label()
            );
        }
        let last = SECTIONS.last().expect("sections");
        assert_eq!(last.category, PreferencesCategory::About);
        assert_eq!(SECTIONS.len(), 13, "every existing section is preserved");
    }
}
