//! Call timing: when the caller should speak relative to the dance.
//!
//! This is the heart of the trainer. A call must land **before** the dancers
//! execute the figure, and the lead depends on how long the rendered words
//! take to say.

use serde::{Deserialize, Serialize};

use crate::alias::AliasTable;
use crate::dance::{Dance, DanceError};
use crate::roles::RoleTerms;
use crate::time::Tempo;

/// Flat safety padding added to every call, in seconds.
///
/// This is deliberately outside the min/max speech-lead clamp: the padding is
/// a constant buffer for every instruction, while the clamp keeps the speech
/// estimate itself within the caller's chosen range.
pub const LEAD_PADDING_SECONDS: f64 = 0.5;

/// User-tunable call-lead calculation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeadSettings {
    /// Automatic syllable-based lead. When false, `manual_beats` is used.
    #[serde(default = "default_true")]
    pub auto: bool,
    #[serde(default = "default_min_lead")]
    pub min_beats: f64,
    #[serde(default = "default_max_lead")]
    pub max_beats: f64,
    #[serde(default = "default_syllables_per_second")]
    pub syllables_per_second: f64,
    #[serde(default = "default_word_gap")]
    pub word_gap_seconds: f64,
    #[serde(default = "default_extra_pause")]
    pub extra_pause_seconds: f64,
    #[serde(default = "default_lead_scale")]
    pub lead_scale: f64,
    #[serde(default = "default_manual_lead")]
    pub manual_beats: f64,
}

fn default_true() -> bool {
    true
}
fn default_min_lead() -> f64 {
    1.0
}
fn default_max_lead() -> f64 {
    5.0
}
fn default_syllables_per_second() -> f64 {
    // Calibrated from a measured 2.25 s delivery of the nine-syllable call
    // "larks allemande left once and a half".
    4.0
}
fn default_word_gap() -> f64 {
    0.0
}
fn default_extra_pause() -> f64 {
    0.0
}
fn default_lead_scale() -> f64 {
    1.0
}
fn default_manual_lead() -> f64 {
    1.5
}

impl Default for LeadSettings {
    fn default() -> Self {
        Self {
            auto: true,
            min_beats: default_min_lead(),
            max_beats: default_max_lead(),
            syllables_per_second: default_syllables_per_second(),
            word_gap_seconds: default_word_gap(),
            extra_pause_seconds: default_extra_pause(),
            lead_scale: default_lead_scale(),
            manual_beats: default_manual_lead(),
        }
    }
}

impl LeadSettings {
    /// Clamp settings into a sane range after loading preferences.
    pub fn validate(&mut self) {
        if !self.min_beats.is_finite() {
            self.min_beats = default_min_lead();
        }
        if !self.max_beats.is_finite() {
            self.max_beats = default_max_lead();
        }
        self.min_beats = self.min_beats.clamp(0.25, 8.0);
        self.max_beats = self.max_beats.clamp(self.min_beats, 8.0);
        if !self.syllables_per_second.is_finite() || self.syllables_per_second < 0.5 {
            self.syllables_per_second = default_syllables_per_second();
        }
        self.syllables_per_second = self.syllables_per_second.clamp(0.5, 20.0);
        if !self.word_gap_seconds.is_finite() {
            self.word_gap_seconds = default_word_gap();
        }
        self.word_gap_seconds = self.word_gap_seconds.clamp(0.0, 1.0);
        if !self.extra_pause_seconds.is_finite() {
            self.extra_pause_seconds = default_extra_pause();
        }
        self.extra_pause_seconds = self.extra_pause_seconds.clamp(0.0, 2.0);
        if !self.lead_scale.is_finite() {
            self.lead_scale = default_lead_scale();
        }
        self.lead_scale = self.lead_scale.clamp(0.25, 4.0);
        if !self.manual_beats.is_finite() {
            self.manual_beats = default_manual_lead();
        }
        self.manual_beats = self.manual_beats.clamp(self.min_beats, self.max_beats);
    }

    /// Lead in dance beats for a rendered call, including the flat padding.
    pub fn lead_beats(&self, syllables: usize, words: usize, tempo: Tempo) -> f64 {
        let speech_lead = if !self.auto {
            self.manual_beats.clamp(self.min_beats, self.max_beats)
        } else {
            let speech_seconds = syllables as f64 / self.syllables_per_second
                + words.saturating_sub(1) as f64 * self.word_gap_seconds
                + self.extra_pause_seconds;
            let beats = speech_seconds / tempo.seconds_per_dance_beat().0 * self.lead_scale;
            beats.clamp(self.min_beats, self.max_beats)
        };

        let padding_beats = LEAD_PADDING_SECONDS / tempo.seconds_per_dance_beat().0;
        speech_lead + padding_beats
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineConfig {
    /// Number of accompaniment beats before the loop. Three is the product
    /// default.
    pub count_in_beats: f64,
    /// The music loop length. A standard contra tune is 64 dance beats.
    pub music_loop_beats: f64,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            count_in_beats: 3.0,
            music_loop_beats: 64.0,
        }
    }
}

/// A call placed on the practice timeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallEvent {
    pub figure_index: usize,
    pub phrase: Option<String>,
    /// When the caller should begin speaking. May be negative (count-in).
    pub call_start_beat: f64,
    /// When the dancers execute the figure.
    pub execute_beat: f64,
    /// The end of this sub-call / figure.
    pub end_beat: f64,
    pub text: String,
    pub original_text: String,
    pub syllables: usize,
    pub words: usize,
}

impl CallEvent {
    pub fn lead_beats(&self) -> f64 {
        self.execute_beat - self.call_start_beat
    }

    pub fn contains_beat(&self, beat: f64) -> bool {
        beat >= self.call_start_beat && beat < self.end_beat
    }
}

/// A call occurrence in a particular loop, with times adjusted for a lead that
/// wraps past the loop boundary.
///
/// The first call of a dance may need to start before beat 0 (during the
/// count-in). When the dance loops, that same lead has to happen at the end of
/// the previous loop so the first move is still called. A wrapped occurrence
/// therefore runs from `start_beat` to the loop boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledCall {
    /// Index into [`PracticeTimeline::calls`].
    pub index: usize,
    pub call: CallEvent,
    /// When the caller starts speaking, in current-loop beats.
    pub start_beat: f64,
    /// When the figure executes, in current-loop beats.
    pub execute_beat: f64,
    /// End of the figure in current-loop beats.
    pub end_beat: f64,
    /// True for the pre-loop occurrence of a call whose lead begins before 0.
    pub wrapped: bool,
}

impl ScheduledCall {
    /// True while the caller should be speaking this occurrence.
    pub fn is_calling(&self, beat: f64) -> bool {
        beat >= self.start_beat && beat < self.execute_beat
    }

    /// True while the figure is being called or executed.
    pub fn contains_beat(&self, beat: f64) -> bool {
        beat >= self.start_beat && beat < self.end_beat
    }
}

/// A labelled phrase span, e.g. A1 from beat 0 to 16.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhraseSpan {
    pub label: String,
    pub start_beat: f64,
    pub end_beat: f64,
}

/// The complete call sequence for one dance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PracticeTimeline {
    pub calls: Vec<CallEvent>,
    pub phrases: Vec<PhraseSpan>,
    pub total_beats: f64,
    pub music_loop_beats: f64,
    pub count_in_beats: f64,
}

impl PracticeTimeline {
    /// Build a timeline from a dance, alias table, and lead settings.
    pub fn build(
        dance: &Dance,
        aliases: &AliasTable,
        roles: &RoleTerms,
        lead: &LeadSettings,
        tempo: Tempo,
        config: TimelineConfig,
    ) -> Result<Self, TimelineError> {
        dance.validate().map_err(TimelineError::Dance)?;

        let mut calls = Vec::new();
        let mut phrases = Vec::new();
        let mut cursor = 0.0;
        let mut current_label: Option<String> = None;

        for (figure_index, figure) in dance.figures.iter().enumerate() {
            if let Some(label) = &figure.phrase
                && !label.trim().is_empty()
            {
                current_label = Some(label.trim().to_string());
            }

            let figure_start = cursor;
            let figure_end = figure_start + figure.beats;
            let parts = split_figure_text(&figure.text);
            let allocations = allocate_beats(&parts, figure.beats);

            for (part, allocated) in parts.iter().zip(allocations.iter()) {
                let rendered = aliases.render(part, roles);
                let lead_beats = lead.lead_beats(rendered.syllables, rendered.words, tempo);
                let execute_beat = figure_start + allocated.offset;
                calls.push(CallEvent {
                    figure_index,
                    phrase: current_label.clone(),
                    call_start_beat: execute_beat - lead_beats,
                    execute_beat,
                    end_beat: execute_beat + allocated.beats,
                    text: rendered.text,
                    original_text: rendered.original,
                    syllables: rendered.syllables,
                    words: rendered.words,
                });
            }

            phrases.push(PhraseSpan {
                label: current_label.clone().unwrap_or_else(|| "—".into()),
                start_beat: figure_start,
                end_beat: figure_end,
            });

            cursor = figure_end;
        }

        if calls.is_empty() {
            return Err(TimelineError::NoCalls);
        }

        calls.sort_by(|a, b| {
            a.call_start_beat
                .total_cmp(&b.call_start_beat)
                .then(a.execute_beat.total_cmp(&b.execute_beat))
        });

        Ok(Self {
            calls,
            phrases,
            total_beats: cursor,
            music_loop_beats: config.music_loop_beats.max(1.0),
            count_in_beats: config.count_in_beats.max(0.0),
        })
    }

    fn make_scheduled(&self, index: usize, wrapped: bool) -> ScheduledCall {
        let call = self.calls[index].clone();
        if wrapped {
            ScheduledCall {
                index,
                start_beat: call.call_start_beat + self.music_loop_beats,
                execute_beat: self.music_loop_beats,
                end_beat: self.music_loop_beats,
                wrapped: true,
                call,
            }
        } else {
            ScheduledCall {
                index,
                start_beat: call.call_start_beat,
                execute_beat: call.execute_beat,
                end_beat: call.end_beat,
                wrapped: false,
                call,
            }
        }
    }

    /// The call being spoken or executed at `beat`, taking loop wrapping into
    /// account.
    ///
    /// When `looping` is true, a call whose lead starts before beat 0 is also
    /// available at the end of the loop, so the first move is called before
    /// the next repetition.
    pub fn scheduled_call_at(&self, beat: f64, looping: bool) -> Option<ScheduledCall> {
        if looping && beat >= 0.0 {
            let loop_beats = self.music_loop_beats;
            let wrapped = self
                .calls
                .iter()
                .enumerate()
                .rev()
                .find(|(_, call)| {
                    call.call_start_beat < 0.0
                        && beat >= call.call_start_beat + loop_beats
                        && beat < loop_beats
                })
                .map(|(index, _)| index);
            if let Some(index) = wrapped {
                return Some(self.make_scheduled(index, true));
            }
        }

        self.calls
            .iter()
            .enumerate()
            .rev()
            .find(|(_, call)| beat >= call.call_start_beat && beat < call.end_beat)
            .map(|(index, _)| self.make_scheduled(index, false))
    }

    /// The `index`-th call as it occurs at `beat` in the current loop.
    ///
    /// This is used by the dance card, which lists every call but needs the
    /// wrapped phase for the first call near the loop boundary.
    pub fn scheduled_call_for_index(
        &self,
        index: usize,
        beat: f64,
        looping: bool,
    ) -> Option<ScheduledCall> {
        let call = self.calls.get(index)?;
        let wrapped = looping
            && call.call_start_beat < 0.0
            && beat >= call.call_start_beat + self.music_loop_beats
            && beat < self.music_loop_beats;
        Some(self.make_scheduled(index, wrapped))
    }

    /// The next call to begin after `beat`, taking loop wrapping into account.
    pub fn next_scheduled_call(&self, beat: f64, looping: bool) -> Option<ScheduledCall> {
        let normal = self
            .calls
            .iter()
            .enumerate()
            .find(|(_, call)| call.call_start_beat > beat)
            .map(|(index, _)| self.make_scheduled(index, false));

        if !looping {
            return normal;
        }

        let loop_beats = self.music_loop_beats;
        let wrapped = self
            .calls
            .iter()
            .enumerate()
            .filter(|(_, call)| call.call_start_beat < 0.0)
            .map(|(index, call)| (index, call.call_start_beat + loop_beats))
            .find(|(_, start)| *start > beat)
            .map(|(index, _)| self.make_scheduled(index, true));

        match (normal, wrapped) {
            (Some(normal), Some(wrapped)) => {
                if normal.start_beat <= wrapped.start_beat {
                    Some(normal)
                } else {
                    Some(wrapped)
                }
            }
            (Some(normal), None) => Some(normal),
            (None, Some(wrapped)) => Some(wrapped),
            (None, None) => None,
        }
    }

    /// The call that is currently being spoken or executed at `beat`.
    pub fn call_at(&self, beat: f64) -> Option<&CallEvent> {
        self.calls.iter().find(|call| call.contains_beat(beat))
    }

    pub fn next_call(&self, beat: f64) -> Option<&CallEvent> {
        self.calls.iter().find(|call| call.call_start_beat > beat)
    }

    /// The call whose figure begins at or after `beat`.
    pub fn upcoming_call(&self, beat: f64) -> Option<&CallEvent> {
        self.calls.iter().find(|call| call.execute_beat >= beat)
    }

    pub fn phrase_at(&self, beat: f64) -> Option<&PhraseSpan> {
        self.phrases
            .iter()
            .find(|phrase| beat >= phrase.start_beat && beat < phrase.end_beat)
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum TimelineError {
    #[error("invalid dance: {0}")]
    Dance(#[from] DanceError),
    #[error("dance produced no calls")]
    NoCalls,
}

#[derive(Clone, Copy, Debug)]
struct Allocation {
    offset: f64,
    beats: f64,
}

/// Split a figure row into explicit sub-calls. "&" is *not* a separator:
/// "balance & swing" is one call.
fn split_figure_text(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    for chunk in text
        .split([';', '\n'])
        .flat_map(|chunk| chunk.split(" - "))
        .flat_map(|chunk| chunk.split(" – "))
        .flat_map(|chunk| chunk.split(" — "))
    {
        let part = chunk.trim();
        if !part.is_empty() {
            parts.push(part.to_string());
        }
    }
    if parts.is_empty() {
        parts.push(text.trim().to_string());
    }
    parts
}

fn allocate_beats(parts: &[String], total: f64) -> Vec<Allocation> {
    if parts.is_empty() {
        return Vec::new();
    }
    if parts.len() == 1 {
        return vec![Allocation {
            offset: 0.0,
            beats: total,
        }];
    }

    // A trailing/standalone balance gets about two beats; the rest of the
    // figure is shared by the other sub-calls. If that does not fit, fall
    // back to an equal split.
    let mut fixed = vec![0.0_f64; parts.len()];
    if total >= 4.0 {
        for (index, part) in parts.iter().enumerate() {
            if index > 0 && part.to_lowercase().contains("balance") {
                fixed[index] = 2.0;
            }
        }
    }

    let fixed_sum: f64 = fixed.iter().sum();
    let flexible_count = fixed.iter().filter(|value| **value == 0.0).count();
    if fixed_sum <= 0.0 || flexible_count == 0 || fixed_sum >= total - 0.5 {
        return equal_split(parts.len(), total);
    }

    let remaining = total - fixed_sum;
    let per_flexible = remaining / flexible_count as f64;
    if per_flexible <= 0.0 {
        return equal_split(parts.len(), total);
    }

    let mut offset = 0.0;
    let mut out = Vec::with_capacity(parts.len());
    for value in fixed {
        let beats = if value > 0.0 { value } else { per_flexible };
        out.push(Allocation { offset, beats });
        offset += beats;
    }
    out
}

fn equal_split(count: usize, total: f64) -> Vec<Allocation> {
    let per = total / count as f64;
    (0..count)
        .map(|index| Allocation {
            offset: per * index as f64,
            beats: per,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alias::AliasTable;
    use crate::dance::{Dance, Figure, SourceRef};
    use crate::roles::RoleTerms;

    fn another_last_dance() -> Dance {
        Dance {
            source: SourceRef::new("contradb", "2409", "https://contradb.com/dances/2409"),
            title: "Another Last Dance".into(),
            choreographer: "Erik Hoffman".into(),
            formation: "improper".into(),
            preamble: String::new(),
            notes: String::new(),
            figures: vec![
                Figure::new(Some("A1".into()), 8.0, "neighbors do si do once"),
                Figure::new(None, 8.0, "ladles allemande left 1½"),
                Figure::new(Some("A2".into()), 16.0, "partners balance & swing"),
                Figure::new(Some("B1".into()), 8.0, "circle left 3 places"),
                Figure::new(None, 8.0, "neighbors swing"),
                Figure::new(Some("B2".into()), 8.0, "long lines forward & back"),
                Figure::new(None, 8.0, "star left 4 places"),
            ],
        }
    }

    #[test]
    fn default_dance_has_one_call_per_figure() {
        let timeline = PracticeTimeline::build(
            &another_last_dance(),
            &AliasTable::default(),
            &RoleTerms::default(),
            &LeadSettings::default(),
            Tempo::default(),
            TimelineConfig::default(),
        )
        .expect("timeline");
        assert_eq!(timeline.calls.len(), 7);
        assert_eq!(timeline.total_beats, 64.0);
        assert_eq!(timeline.count_in_beats, 3.0);
    }

    #[test]
    fn first_call_starts_in_the_count_in() {
        let timeline = PracticeTimeline::build(
            &another_last_dance(),
            &AliasTable::default(),
            &RoleTerms::default(),
            &LeadSettings::default(),
            Tempo::default(),
            TimelineConfig::default(),
        )
        .expect("timeline");
        let first = &timeline.calls[0];
        assert!(
            first.call_start_beat < 0.0,
            "first call should be shown/sung during the count-in, got {}",
            first.call_start_beat
        );
        assert_eq!(first.execute_beat, 0.0);
    }

    #[test]
    fn lead_is_between_two_and_six_beats() {
        let timeline = PracticeTimeline::build(
            &another_last_dance(),
            &AliasTable::default(),
            &RoleTerms::default(),
            &LeadSettings::default(),
            Tempo::default(),
            TimelineConfig::default(),
        )
        .expect("timeline");
        for call in &timeline.calls {
            assert!(
                (2.0..=6.0).contains(&call.lead_beats()),
                "lead for {:?} was {}",
                call.text,
                call.lead_beats()
            );
        }
    }

    #[test]
    fn measured_allemande_call_gets_padding() {
        let aliases = AliasTable::default();
        let call = aliases.render("ladles allemande left 1½", &RoleTerms::default());
        assert_eq!(call.text, "larks allemande left once and a half");
        assert_eq!(call.syllables, 9);

        let lead = LeadSettings::default().lead_beats(call.syllables, call.words, Tempo::default());
        // 9 syllables / 4.0 = 2.25 s = 4.5 beats, plus the flat 0.5 s = 5.5 beats.
        assert_eq!(lead, 5.5);
        let seconds = lead * Tempo::default().seconds_per_dance_beat().0;
        assert!((seconds - 2.75).abs() < 1e-9);
    }

    #[test]
    fn padding_is_half_a_second_at_any_tempo() {
        let lead = LeadSettings {
            auto: false,
            manual_beats: 1.25,
            ..LeadSettings::default()
        };
        for bpm in [120.0, 180.0, 240.0, 320.0] {
            let tempo = Tempo::new(bpm);
            let expected = 1.25 + LEAD_PADDING_SECONDS / tempo.seconds_per_dance_beat().0;
            assert!(
                (lead.lead_beats(0, 0, tempo) - expected).abs() < 1e-9,
                "padding was not flat at {bpm} BPM"
            );
        }
    }

    #[test]
    fn first_call_wraps_to_the_end_of_the_loop() {
        let timeline = PracticeTimeline::build(
            &another_last_dance(),
            &AliasTable::default(),
            &RoleTerms::default(),
            &LeadSettings::default(),
            Tempo::default(),
            TimelineConfig::default(),
        )
        .expect("timeline");
        let first = &timeline.calls[0];
        let wrapped_start = timeline.music_loop_beats - first.lead_beats();
        assert!(wrapped_start > 0.0);
        assert!(wrapped_start < timeline.music_loop_beats);

        let wrapped = timeline
            .scheduled_call_at(wrapped_start + 0.1, true)
            .expect("wrapped first call");
        assert!(wrapped.wrapped);
        assert_eq!(wrapped.index, 0);
        assert_eq!(wrapped.execute_beat, timeline.music_loop_beats);
        assert_eq!(wrapped.end_beat, timeline.music_loop_beats);
        assert!((wrapped.start_beat - wrapped_start).abs() < 1e-9);
        assert!(wrapped.is_calling(wrapped_start + 0.1));

        let not_wrapped = timeline.scheduled_call_at(wrapped_start + 0.1, false);
        assert!(not_wrapped.is_none_or(|call| !call.wrapped));
    }

    #[test]
    fn next_call_includes_the_wrapped_first_call() {
        let timeline = PracticeTimeline::build(
            &another_last_dance(),
            &AliasTable::default(),
            &RoleTerms::default(),
            &LeadSettings::default(),
            Tempo::default(),
            TimelineConfig::default(),
        )
        .expect("timeline");
        let wrapped_start = timeline.music_loop_beats - timeline.calls[0].lead_beats();
        let next = timeline
            .next_scheduled_call(wrapped_start - 0.5, true)
            .expect("next call");
        assert!(next.wrapped);
        assert_eq!(next.index, 0);
        assert!((next.start_beat - wrapped_start).abs() < 1e-9);
    }

    #[test]
    fn loop_boundary_hands_back_to_the_normal_first_call() {
        let timeline = PracticeTimeline::build(
            &another_last_dance(),
            &AliasTable::default(),
            &RoleTerms::default(),
            &LeadSettings::default(),
            Tempo::default(),
            TimelineConfig::default(),
        )
        .expect("timeline");
        let at_zero = timeline
            .scheduled_call_at(0.0, true)
            .expect("first call at the loop start");
        assert!(!at_zero.wrapped);
        assert_eq!(at_zero.index, 0);
        assert_eq!(at_zero.execute_beat, 0.0);
    }

    #[test]
    fn calls_are_sorted() {
        let timeline = PracticeTimeline::build(
            &another_last_dance(),
            &AliasTable::default(),
            &RoleTerms::default(),
            &LeadSettings::default(),
            Tempo::default(),
            TimelineConfig::default(),
        )
        .expect("timeline");
        let mut sorted = timeline.calls.clone();
        sorted.sort_by(|a, b| a.call_start_beat.total_cmp(&b.call_start_beat));
        assert_eq!(timeline.calls, sorted);
    }

    #[test]
    fn figure_with_dash_is_split_into_two_calls() {
        let mut dance = another_last_dance();
        dance.figures = vec![Figure::new(
            Some("A1".into()),
            8.0,
            "ladles dance in to a long wave in the center - balance the wave",
        )];
        let timeline = PracticeTimeline::build(
            &dance,
            &AliasTable::default(),
            &RoleTerms::default(),
            &LeadSettings::default(),
            Tempo::default(),
            TimelineConfig::default(),
        )
        .expect("timeline");
        assert_eq!(timeline.calls.len(), 2);
        assert_eq!(timeline.calls[0].execute_beat, 0.0);
        assert!(timeline.calls[1].execute_beat > 0.0);
    }

    #[test]
    fn manual_lead_also_gets_padding() {
        let lead = LeadSettings {
            auto: false,
            manual_beats: 1.25,
            ..LeadSettings::default()
        };
        assert_eq!(lead.lead_beats(100, 20, Tempo::default()), 2.25);
    }

    #[test]
    fn slower_tempo_needs_fewer_beats_for_the_same_words() {
        let lead = LeadSettings::default();
        let fast = lead.lead_beats(4, 2, Tempo::new(300.0));
        let slow = lead.lead_beats(4, 2, Tempo::new(180.0));
        assert!(
            slow <= fast,
            "a longer beat should fit the same words in fewer beats"
        );
    }
}
