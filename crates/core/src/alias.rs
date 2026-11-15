//! Dance-move classification and caller alias rendering.
//!
//! Card text is brief ("ladles allemande left 1½"); callers say it in many
//! different ways. The alias table maps a recognised move type to a template
//! such as `"{actor} allemande {direction} {count}"`, then `{actor}` is
//! rendered with the caller's chosen role vocabulary. Templates can force a
//! number with `{actor_singular}` or `{actor_plural}`; the object forms add
//! "your" ("your neighbor", "your neighbors").

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::roles::RoleTerms;
use crate::syllables::{count_syllables, word_count};

/// Move types the alias editor understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoveKind {
    Swing,
    DoSiDo,
    Allemande,
    AllemandeOrbit,
    Balance,
    BalanceTheRing,
    BalanceAndSwing,
    Chain,
    Circle,
    Star,
    StarPromenade,
    LongLines,
    Hey,
    DolphinHey,
    Gyre,
    Promenade,
    RightLeftThrough,
    PassThrough,
    PassBy,
    RollAway,
    Petronella,
    BoxTheGnat,
    SwatTheFlea,
    MadRobin,
    RoryOMore,
    Slide,
    SquareThrough,
    GiveAndTake,
    ButterflyWhirl,
    CaliforniaTwirl,
    Gate,
    Poussette,
    ArchAndDive,
    CrossTrails,
    FigureEight,
    TurnAlone,
    DownTheHall,
    UpTheHall,
    FormLongWaves,
    FormOceanWave,
    ContraCorners,
    MeltdownSwing,
    RevolvingDoor,
    SeeSaw,
    Other,
}

impl MoveKind {
    /// All built-in kinds, in the order shown by the alias editor.
    pub const ALL: [MoveKind; 45] = [
        MoveKind::Swing,
        MoveKind::DoSiDo,
        MoveKind::Allemande,
        MoveKind::AllemandeOrbit,
        MoveKind::Balance,
        MoveKind::BalanceTheRing,
        MoveKind::BalanceAndSwing,
        MoveKind::Chain,
        MoveKind::Circle,
        MoveKind::Star,
        MoveKind::StarPromenade,
        MoveKind::LongLines,
        MoveKind::Hey,
        MoveKind::DolphinHey,
        MoveKind::Gyre,
        MoveKind::Promenade,
        MoveKind::RightLeftThrough,
        MoveKind::PassThrough,
        MoveKind::PassBy,
        MoveKind::RollAway,
        MoveKind::Petronella,
        MoveKind::BoxTheGnat,
        MoveKind::SwatTheFlea,
        MoveKind::MadRobin,
        MoveKind::RoryOMore,
        MoveKind::Slide,
        MoveKind::SquareThrough,
        MoveKind::GiveAndTake,
        MoveKind::ButterflyWhirl,
        MoveKind::CaliforniaTwirl,
        MoveKind::Gate,
        MoveKind::Poussette,
        MoveKind::ArchAndDive,
        MoveKind::CrossTrails,
        MoveKind::FigureEight,
        MoveKind::TurnAlone,
        MoveKind::DownTheHall,
        MoveKind::UpTheHall,
        MoveKind::FormLongWaves,
        MoveKind::FormOceanWave,
        MoveKind::ContraCorners,
        MoveKind::MeltdownSwing,
        MoveKind::RevolvingDoor,
        MoveKind::SeeSaw,
        MoveKind::Other,
    ];

    pub fn id(self) -> &'static str {
        match self {
            MoveKind::Swing => "swing",
            MoveKind::DoSiDo => "do-si-do",
            MoveKind::Allemande => "allemande",
            MoveKind::AllemandeOrbit => "allemande-orbit",
            MoveKind::Balance => "balance",
            MoveKind::BalanceTheRing => "balance-the-ring",
            MoveKind::BalanceAndSwing => "balance-and-swing",
            MoveKind::Chain => "chain",
            MoveKind::Circle => "circle",
            MoveKind::Star => "star",
            MoveKind::StarPromenade => "star-promenade",
            MoveKind::LongLines => "long-lines",
            MoveKind::Hey => "hey",
            MoveKind::DolphinHey => "dolphin-hey",
            MoveKind::Gyre => "gyre",
            MoveKind::Promenade => "promenade",
            MoveKind::RightLeftThrough => "right-left-through",
            MoveKind::PassThrough => "pass-through",
            MoveKind::PassBy => "pass-by",
            MoveKind::RollAway => "roll-away",
            MoveKind::Petronella => "petronella",
            MoveKind::BoxTheGnat => "box-the-gnat",
            MoveKind::SwatTheFlea => "swat-the-flea",
            MoveKind::MadRobin => "mad-robin",
            MoveKind::RoryOMore => "rory-o-more",
            MoveKind::Slide => "slide",
            MoveKind::SquareThrough => "square-through",
            MoveKind::GiveAndTake => "give-and-take",
            MoveKind::ButterflyWhirl => "butterfly-whirl",
            MoveKind::CaliforniaTwirl => "california-twirl",
            MoveKind::Gate => "gate",
            MoveKind::Poussette => "poussette",
            MoveKind::ArchAndDive => "arch-and-dive",
            MoveKind::CrossTrails => "cross-trails",
            MoveKind::FigureEight => "figure-eight",
            MoveKind::TurnAlone => "turn-alone",
            MoveKind::DownTheHall => "down-the-hall",
            MoveKind::UpTheHall => "up-the-hall",
            MoveKind::FormLongWaves => "form-long-waves",
            MoveKind::FormOceanWave => "form-ocean-wave",
            MoveKind::ContraCorners => "contra-corners",
            MoveKind::MeltdownSwing => "meltdown-swing",
            MoveKind::RevolvingDoor => "revolving-door",
            MoveKind::SeeSaw => "see-saw",
            MoveKind::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MoveKind::Swing => "Swing",
            MoveKind::DoSiDo => "Do-si-do",
            MoveKind::Allemande => "Allemande",
            MoveKind::AllemandeOrbit => "Allemande orbit",
            MoveKind::Balance => "Balance",
            MoveKind::BalanceTheRing => "Balance the ring",
            MoveKind::BalanceAndSwing => "Balance and swing",
            MoveKind::Chain => "Chain",
            MoveKind::Circle => "Circle",
            MoveKind::Star => "Star",
            MoveKind::StarPromenade => "Star promenade",
            MoveKind::LongLines => "Long lines",
            MoveKind::Hey => "Hey",
            MoveKind::DolphinHey => "Dolphin hey",
            MoveKind::Gyre => "Gyre / gypsy",
            MoveKind::Promenade => "Promenade",
            MoveKind::RightLeftThrough => "Right and left through",
            MoveKind::PassThrough => "Pass through",
            MoveKind::PassBy => "Pass by",
            MoveKind::RollAway => "Roll away",
            MoveKind::Petronella => "Petronella",
            MoveKind::BoxTheGnat => "Box the gnat",
            MoveKind::SwatTheFlea => "Swat the flea",
            MoveKind::MadRobin => "Mad robin",
            MoveKind::RoryOMore => "Rory O'More",
            MoveKind::Slide => "Slide",
            MoveKind::SquareThrough => "Square through",
            MoveKind::GiveAndTake => "Give and take",
            MoveKind::ButterflyWhirl => "Butterfly whirl",
            MoveKind::CaliforniaTwirl => "California twirl",
            MoveKind::Gate => "Gate",
            MoveKind::Poussette => "Poussette",
            MoveKind::ArchAndDive => "Arch and dive",
            MoveKind::CrossTrails => "Cross trails",
            MoveKind::FigureEight => "Figure eight",
            MoveKind::TurnAlone => "Turn alone",
            MoveKind::DownTheHall => "Down the hall",
            MoveKind::UpTheHall => "Up the hall",
            MoveKind::FormLongWaves => "Form long waves",
            MoveKind::FormOceanWave => "Form an ocean wave",
            MoveKind::ContraCorners => "Contra corners",
            MoveKind::MeltdownSwing => "Meltdown swing",
            MoveKind::RevolvingDoor => "Revolving door",
            MoveKind::SeeSaw => "See saw",
            MoveKind::Other => "Other / custom",
        }
    }

    /// Default caller template. Empty placeholders are removed cleanly.
    pub fn default_template(self) -> &'static str {
        match self {
            MoveKind::Swing => "swing your {actor_singular}",
            MoveKind::DoSiDo => "{actor} do-si-do {count}",
            MoveKind::Allemande => "{actor} allemande {direction} {count}",
            MoveKind::AllemandeOrbit => "{actor} allemande orbit",
            MoveKind::Balance => "{actor} balance",
            MoveKind::BalanceTheRing => "balance the ring",
            MoveKind::BalanceAndSwing => "{actor} balance and swing",
            MoveKind::Chain => "{actor} chain",
            MoveKind::Circle => "circle {direction} {count}",
            MoveKind::Star => "star {direction} {count}",
            MoveKind::StarPromenade => "star promenade",
            MoveKind::LongLines => "long lines forward and back",
            MoveKind::Hey => "hey",
            MoveKind::DolphinHey => "dolphin hey",
            MoveKind::Gyre => "right shoulder round",
            MoveKind::Promenade => "promenade",
            MoveKind::RightLeftThrough => "right and left through",
            MoveKind::PassThrough => "pass through",
            MoveKind::PassBy => "pass by",
            MoveKind::RollAway => "roll away",
            MoveKind::Petronella => "petronella",
            MoveKind::BoxTheGnat => "box the gnat",
            MoveKind::SwatTheFlea => "swat the flea",
            MoveKind::MadRobin => "mad robin",
            MoveKind::RoryOMore => "Rory O'More",
            MoveKind::Slide => "slide {direction}",
            MoveKind::SquareThrough => "square through",
            MoveKind::GiveAndTake => "give and take",
            MoveKind::ButterflyWhirl => "butterfly whirl",
            MoveKind::CaliforniaTwirl => "California twirl",
            MoveKind::Gate => "gate",
            MoveKind::Poussette => "poussette",
            MoveKind::ArchAndDive => "arch and dive",
            MoveKind::CrossTrails => "cross trails",
            MoveKind::FigureEight => "figure eight",
            MoveKind::TurnAlone => "turn alone",
            MoveKind::DownTheHall => "down the hall",
            MoveKind::UpTheHall => "up the hall",
            MoveKind::FormLongWaves => "form long waves",
            MoveKind::FormOceanWave => "form an ocean wave",
            MoveKind::ContraCorners => "contra corners",
            MoveKind::MeltdownSwing => "meltdown swing",
            MoveKind::RevolvingDoor => "revolving door",
            MoveKind::SeeSaw => "see saw",
            MoveKind::Other => "",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }
}

/// A dance role as it appears on the card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Actor {
    Neighbor,
    Partner,
    Lark,
    Robin,
    Shadow,
    FirstCorner,
    SecondCorner,
    Other(String),
}

/// An actor plus whether the card phrase used a plural form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorRef {
    pub actor: Actor,
    pub plural: bool,
}

impl ActorRef {
    pub fn singular(&self) -> bool {
        !self.plural
    }

    /// The actor in the number used on the card.
    pub fn display(&self, roles: &RoleTerms) -> String {
        if self.plural {
            self.plural_display(roles)
        } else {
            self.singular_display(roles)
        }
    }

    /// The actor in the singular, regardless of the card phrase.
    pub fn singular_display(&self, roles: &RoleTerms) -> String {
        match &self.actor {
            Actor::Lark => roles.left_singular.clone(),
            Actor::Robin => roles.right_singular.clone(),
            Actor::Neighbor => "neighbor".into(),
            Actor::Partner => "partner".into(),
            Actor::Shadow => "shadow".into(),
            Actor::FirstCorner => "first corner".into(),
            Actor::SecondCorner => "second corner".into(),
            Actor::Other(value) => value.clone(),
        }
    }

    /// The actor in the plural, regardless of the card phrase.
    pub fn plural_display(&self, roles: &RoleTerms) -> String {
        match &self.actor {
            Actor::Lark => roles.left_plural.clone(),
            Actor::Robin => roles.right_plural.clone(),
            Actor::Neighbor => "neighbors".into(),
            Actor::Partner => "partners".into(),
            Actor::Shadow => "shadows".into(),
            Actor::FirstCorner => "first corners".into(),
            Actor::SecondCorner => "second corners".into(),
            Actor::Other(value) => value.clone(),
        }
    }
}

/// Parameters extracted from a card phrase.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MoveParams {
    pub actor: Option<ActorRef>,
    pub direction: Option<String>,
    pub count: Option<String>,
}

/// The result of classifying one card phrase.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassifiedMove {
    pub kind: MoveKind,
    pub params: MoveParams,
    pub original: String,
    pub normalized: String,
}

impl ClassifiedMove {
    pub fn actor(&self) -> Option<&ActorRef> {
        self.params.actor.as_ref()
    }
}

/// A rendered call, ready for the timeline and TTS.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedCall {
    pub kind: MoveKind,
    pub text: String,
    pub original: String,
    pub syllables: usize,
    pub words: usize,
    pub params: MoveParams,
    pub used_custom_alias: bool,
}

/// Caller alias overrides, persisted with preferences.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasTable {
    /// Keyed by [`MoveKind::id`].
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    /// Keyed by the normalised lowercase card phrase.
    #[serde(default)]
    pub custom: BTreeMap<String, String>,
}

impl AliasTable {
    pub fn template(&self, kind: MoveKind) -> &str {
        self.overrides
            .get(kind.id())
            .map(String::as_str)
            .unwrap_or_else(|| kind.default_template())
    }

    pub fn set_override(&mut self, kind: MoveKind, template: impl Into<String>) {
        let template = template.into();
        if template.trim().is_empty() {
            self.overrides.remove(kind.id());
        } else {
            self.overrides.insert(kind.id().to_string(), template);
        }
    }

    pub fn reset(&mut self, kind: MoveKind) {
        self.overrides.remove(kind.id());
    }

    pub fn is_customised(&self, kind: MoveKind) -> bool {
        self.overrides.contains_key(kind.id())
    }

    pub fn set_custom(&mut self, phrase: impl AsRef<str>, alias: impl Into<String>) {
        let key = phrase.as_ref().trim().to_lowercase();
        if key.is_empty() {
            return;
        }
        self.custom.insert(key, alias.into());
    }

    pub fn remove_custom(&mut self, phrase: &str) {
        self.custom.remove(&phrase.trim().to_lowercase());
    }

    /// Classify and render one figure phrase.
    pub fn render(&self, text: &str, roles: &RoleTerms) -> RenderedCall {
        let classified = classify(text);
        let mut used_custom_alias = false;

        let rendered = if classified.kind == MoveKind::Other {
            if let Some(alias) = self.custom.get(&classified.normalized.to_lowercase()) {
                used_custom_alias = true;
                alias.clone()
            } else {
                classified.normalized.clone()
            }
        } else {
            render_template(
                self.template(classified.kind),
                &classified.params,
                roles,
                &classified.normalized,
            )
        };

        let rendered = roles.apply(&rendered);
        let rendered = clean_spaces(&rendered);
        let rendered = if rendered.is_empty() {
            roles.apply(&classified.normalized)
        } else {
            rendered
        };

        RenderedCall {
            kind: classified.kind,
            syllables: count_syllables(&rendered),
            words: word_count(&rendered),
            text: rendered,
            original: classified.original,
            params: classified.params,
            used_custom_alias,
        }
    }
}

/// Classify a card phrase without rendering it.
pub fn classify(text: &str) -> ClassifiedMove {
    let original = text.trim().to_string();
    let normalized = normalize_figure(text);
    let lower = normalized.to_lowercase();
    let kind = detect_kind(&lower);
    let params = MoveParams {
        actor: find_actor(&lower),
        direction: find_direction(&lower),
        count: find_count(&lower),
    };
    ClassifiedMove {
        kind,
        params,
        original,
        normalized,
    }
}

fn detect_kind(lower: &str) -> MoveKind {
    // Specific combinations first.
    if lower.contains("balance and swing") || lower.contains("balance & swing") {
        return MoveKind::BalanceAndSwing;
    }
    if lower.contains("balance the ring") {
        return MoveKind::BalanceTheRing;
    }
    if lower.contains("meltdown swing") {
        return MoveKind::MeltdownSwing;
    }
    if lower.contains("star promenade") {
        return MoveKind::StarPromenade;
    }
    if lower.contains("allemande orbit") {
        return MoveKind::AllemandeOrbit;
    }
    if lower.contains("dolphin hey") {
        return MoveKind::DolphinHey;
    }

    const PATTERNS: &[(&str, MoveKind)] = &[
        ("right left through", MoveKind::RightLeftThrough),
        ("right and left through", MoveKind::RightLeftThrough),
        ("long lines", MoveKind::LongLines),
        ("form a long wave", MoveKind::FormLongWaves),
        ("form long waves", MoveKind::FormLongWaves),
        ("form an ocean wave", MoveKind::FormOceanWave),
        ("form a wave", MoveKind::FormOceanWave),
        ("down the hall", MoveKind::DownTheHall),
        ("up the hall", MoveKind::UpTheHall),
        ("cross trails", MoveKind::CrossTrails),
        ("figure 8", MoveKind::FigureEight),
        ("figure eight", MoveKind::FigureEight),
        ("turn alone", MoveKind::TurnAlone),
        ("pass by", MoveKind::PassBy),
        ("pass through", MoveKind::PassThrough),
        ("roll away", MoveKind::RollAway),
        ("box the gnat", MoveKind::BoxTheGnat),
        ("swat the flea", MoveKind::SwatTheFlea),
        ("mad robin", MoveKind::MadRobin),
        ("rory o'more", MoveKind::RoryOMore),
        ("rory o more", MoveKind::RoryOMore),
        ("california twirl", MoveKind::CaliforniaTwirl),
        ("butterfly whirl", MoveKind::ButterflyWhirl),
        ("square through", MoveKind::SquareThrough),
        ("give and take", MoveKind::GiveAndTake),
        ("give & take", MoveKind::GiveAndTake),
        ("arch and dive", MoveKind::ArchAndDive),
        ("arch & dive", MoveKind::ArchAndDive),
        ("contra corners", MoveKind::ContraCorners),
        ("revolving door", MoveKind::RevolvingDoor),
        ("see saw", MoveKind::SeeSaw),
        ("poussette", MoveKind::Poussette),
        ("petronella", MoveKind::Petronella),
        ("promenade", MoveKind::Promenade),
        ("gyre", MoveKind::Gyre),
        ("gypsy", MoveKind::Gyre),
        ("hey", MoveKind::Hey),
        ("chain", MoveKind::Chain),
        ("circle", MoveKind::Circle),
        ("star", MoveKind::Star),
        ("swing", MoveKind::Swing),
        ("do si do", MoveKind::DoSiDo),
        ("do-si-do", MoveKind::DoSiDo),
        ("dosido", MoveKind::DoSiDo),
        ("dos-a-dos", MoveKind::DoSiDo),
        ("allemande", MoveKind::Allemande),
        ("balance", MoveKind::Balance),
        ("gate", MoveKind::Gate),
        ("slide", MoveKind::Slide),
    ];

    PATTERNS
        .iter()
        .find(|(pattern, _)| lower.contains(pattern))
        .map(|(_, kind)| *kind)
        .unwrap_or(MoveKind::Other)
}

static ACTOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(first corners?|second corners?|neighbou?rs?|partners?|shadows?|gentlespoons?|gentlemen?|gents?|robins?|ladles?|ladies?|larks?|leads?|follows?|lefts|rights|women|men)\b",
    )
    .expect("actor regex is valid")
});

fn find_actor(lower: &str) -> Option<ActorRef> {
    let caps = ACTOR_RE.captures(lower)?;
    let matched = caps.get(0)?.as_str().to_lowercase();
    let plural = matches!(
        matched.as_str(),
        "women" | "men" | "gentlemen" | "gentlespoons" | "ladies" | "larks" | "robins"
    ) || (matched.ends_with('s') && !matched.ends_with("ss"));
    let actor = match matched.as_str() {
        "neighbor" | "neighbors" | "neighbour" | "neighbours" => Actor::Neighbor,
        "partner" | "partners" => Actor::Partner,
        "shadow" | "shadows" => Actor::Shadow,
        "first corner" | "first corners" => Actor::FirstCorner,
        "second corner" | "second corners" => Actor::SecondCorner,
        "gentlespoon" | "gentlespoons" | "gentleman" | "gentlemen" | "gent" | "gents" => {
            Actor::Robin
        }
        "robin" | "robins" | "follow" | "follows" | "right" | "rights" => Actor::Robin,
        "ladle" | "ladles" | "lady" | "ladies" | "lark" | "larks" | "lead" | "leads" | "left"
        | "lefts" | "woman" | "women" => Actor::Lark,
        "man" | "men" => Actor::Robin,
        other => Actor::Other(other.to_string()),
    };
    Some(ActorRef { actor, plural })
}

fn find_direction(lower: &str) -> Option<String> {
    // Longer/ambiguous forms first.
    const DIRECTIONS: &[(&str, &str)] = &[
        ("counterclockwise", "counterclockwise"),
        ("counter-clockwise", "counterclockwise"),
        ("ccw", "counterclockwise"),
        ("clockwise", "clockwise"),
        ("forward", "forward"),
        ("back", "back"),
        ("left", "left"),
        ("right", "right"),
        ("out", "out"),
        ("in", "in"),
        ("up", "up"),
        ("down", "down"),
    ];
    DIRECTIONS
        .iter()
        .find(|(pattern, _)| has_word(lower, pattern))
        .map(|(_, canonical)| (*canonical).to_string())
}

fn find_count(lower: &str) -> Option<String> {
    const COUNTS: &[(&str, &str)] = &[
        ("once and a half", "once and a half"),
        ("one and a half", "once and a half"),
        ("1 1/2", "once and a half"),
        ("1.5", "once and a half"),
        ("twice and a half", "twice and a half"),
        ("two and a half", "twice and a half"),
        ("2 1/2", "twice and a half"),
        ("2.5", "twice and a half"),
        ("three times", "three times"),
        ("3 times", "three times"),
        ("four times", "four times"),
        ("4 times", "four times"),
        ("three places", "three places"),
        ("3 places", "three places"),
        ("four places", "four places"),
        ("4 places", "four places"),
        ("two places", "two places"),
        ("2 places", "two places"),
        ("once", "once"),
        ("one time", "once"),
        ("twice", "twice"),
        ("two times", "twice"),
        ("half", "half"),
    ];
    COUNTS
        .iter()
        .find(|(pattern, _)| lower.contains(pattern))
        .map(|(_, canonical)| (*canonical).to_string())
}

fn has_word(haystack: &str, needle: &str) -> bool {
    if needle.contains(' ') {
        return haystack.contains(needle);
    }
    haystack
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word == needle)
}

fn render_template(
    template: &str,
    params: &MoveParams,
    roles: &RoleTerms,
    fallback: &str,
) -> String {
    let actor = params.actor.as_ref();
    let actor_display = actor.map(|a| a.display(roles)).unwrap_or_default();
    let actor_singular = actor.map(|a| a.singular_display(roles)).unwrap_or_default();
    let actor_plural = actor.map(|a| a.plural_display(roles)).unwrap_or_default();
    let actor_object = actor
        .map(|a| match a.actor {
            Actor::Neighbor | Actor::Partner | Actor::Lark | Actor::Robin => {
                format!("your {}", a.singular_display(roles))
            }
            _ => a.display(roles),
        })
        .unwrap_or_default();
    let actor_object_plural = actor
        .map(|a| match a.actor {
            Actor::Neighbor | Actor::Partner | Actor::Lark | Actor::Robin => {
                format!("your {}", a.plural_display(roles))
            }
            _ => a.plural_display(roles),
        })
        .unwrap_or_default();

    let mut out = template.to_string();
    out = out.replace("{actor}", &actor_display);
    out = out.replace("{actor_singular}", &actor_singular);
    out = out.replace("{actor_plural}", &actor_plural);
    out = out.replace("{actor_object}", &actor_object);
    out = out.replace("{actor_object_plural}", &actor_object_plural);
    out = out.replace(
        "{direction}",
        params.direction.as_deref().unwrap_or_default(),
    );
    out = out.replace("{count}", params.count.as_deref().unwrap_or_default());
    out = out.replace("{text}", fallback);
    clean_spaces(&out)
}

fn normalize_figure(text: &str) -> String {
    let mut text = text.replace("<u>", "").replace("</u>", "");
    text = text.replace("&amp;", " and ");
    text = text.replace('&', " and ");
    text = text.replace("1½", "once and a half");
    text = text.replace("2½", "twice and a half");
    text = text.replace('½', " half ");
    text = text.replace('⁋', " ");
    text = text.replace('→', " ");
    let text = clean_spaces(&text);
    text.trim_matches(|c: char| c == ',' || c == ';' || c == '.' || c == ':')
        .to_string()
}

fn clean_spaces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !last_space && !out.is_empty() {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_default_dance_figures() {
        assert_eq!(classify("neighbors do si do once").kind, MoveKind::DoSiDo);
        assert_eq!(
            classify("ladles allemande left 1½").kind,
            MoveKind::Allemande
        );
        assert_eq!(
            classify("partners balance & swing").kind,
            MoveKind::BalanceAndSwing
        );
        assert_eq!(classify("circle left 3 places").kind, MoveKind::Circle);
        assert_eq!(classify("neighbors swing").kind, MoveKind::Swing);
        assert_eq!(
            classify("long lines forward & back").kind,
            MoveKind::LongLines
        );
        assert_eq!(classify("star left 4 places").kind, MoveKind::Star);
    }

    #[test]
    fn extracts_actor_direction_and_count() {
        let classified = classify("ladles allemande left 1½");
        assert_eq!(classified.kind, MoveKind::Allemande);
        let actor = classified.actor().expect("actor");
        assert_eq!(actor.actor, Actor::Lark);
        assert!(actor.plural);
        assert_eq!(classified.params.direction.as_deref(), Some("left"));
        assert_eq!(classified.params.count.as_deref(), Some("once and a half"));
    }

    #[test]
    fn renders_with_default_roles() {
        let aliases = AliasTable::default();
        let roles = RoleTerms::default();
        let call = aliases.render("ladles allemande left 1½", &roles);
        assert_eq!(call.text, "larks allemande left once and a half");
        assert_eq!(call.kind, MoveKind::Allemande);
    }

    #[test]
    fn renders_swing_with_object() {
        let aliases = AliasTable::default();
        let roles = RoleTerms::default();
        assert_eq!(
            aliases.render("neighbors swing", &roles).text,
            "swing your neighbor"
        );
        assert_eq!(
            aliases.render("partners swing", &roles).text,
            "swing your partner"
        );
    }

    #[test]
    fn gypsy_renders_as_right_shoulder_round() {
        let aliases = AliasTable::default();
        let roles = RoleTerms::default();
        assert_eq!(
            aliases.render("neighbors gypsy", &roles).text,
            "right shoulder round"
        );
        assert_eq!(classify("neighbors gypsy").kind, MoveKind::Gyre);
    }

    #[test]
    fn balance_and_swing_stays_one_call() {
        let call = AliasTable::default().render("partners balance & swing", &RoleTerms::default());
        assert_eq!(call.text, "partners balance and swing");
        assert_eq!(call.kind, MoveKind::BalanceAndSwing);
    }

    #[test]
    fn custom_alias_overrides_a_kind() {
        let mut aliases = AliasTable::default();
        aliases.set_override(MoveKind::Swing, "swing that {actor}");
        let call = aliases.render("neighbors swing", &RoleTerms::default());
        assert_eq!(call.text, "swing that neighbors");
    }

    #[test]
    fn plural_placeholders_ignore_the_card_number() {
        let mut aliases = AliasTable::default();
        aliases.set_override(
            MoveKind::Swing,
            "{actor_singular} and {actor_plural} {actor_object} {actor_object_plural}",
        );
        let roles = RoleTerms::default();
        assert_eq!(
            aliases.render("neighbor swing", &roles).text,
            "neighbor and neighbors your neighbor your neighbors"
        );
        assert_eq!(
            aliases.render("neighbors swing", &roles).text,
            "neighbor and neighbors your neighbor your neighbors"
        );
    }

    #[test]
    fn plural_placeholders_use_configured_role_terms() {
        let mut aliases = AliasTable::default();
        aliases.set_override(MoveKind::Allemande, "{actor_plural} allemande {direction}");
        let roles = RoleTerms::preset(crate::roles::RolePreset::LadiesGentlemen);
        assert_eq!(
            aliases.render("ladle allemande left", &roles).text,
            "ladies allemande left"
        );
        assert_eq!(
            aliases.render("ladles allemande left", &roles).text,
            "ladies allemande left"
        );
    }

    #[test]
    fn plural_placeholders_are_empty_without_an_actor() {
        let mut aliases = AliasTable::default();
        aliases.set_override(
            MoveKind::Circle,
            "circle {actor_plural} {actor_object_plural}",
        );
        assert_eq!(
            aliases.render("circle left", &RoleTerms::default()).text,
            "circle"
        );
    }

    #[test]
    fn custom_alias_for_unknown_figure() {
        let mut aliases = AliasTable::default();
        aliases.set_custom("strange thing", "do the strange thing");
        let call = aliases.render("strange thing", &RoleTerms::default());
        assert_eq!(call.text, "do the strange thing");
        assert!(call.used_custom_alias);
    }

    #[test]
    fn role_terms_are_applied_to_alias() {
        let aliases = AliasTable::default();
        let roles = RoleTerms::preset(crate::roles::RolePreset::LadiesGentlemen);
        let call = aliases.render("ladles chain", &roles);
        assert_eq!(call.text, "ladies chain");
    }
}
