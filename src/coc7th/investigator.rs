use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    fmt::Debug,
    iter::{Iterator, once},
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{f64, i8, i16, i32, str, u8, u16, u32, usize},
};

use arbitrary_int::{i10, traits::Integer, u9};

use crate::{
    Input, Language, Output,
    bit::{self, Zero},
    coc7th::Kind,
    host::dom::{Id, Tag},
    object::Body,
    take,
};

pub const SKILL_COUNT: usize = 47;
pub const BACKSTORY_COUNT: usize = 10;
pub const MEMO_SLOTS: usize = 8;

pub type Dice = (i8, u8, i8); // (count, sides, modifier)

pub mod dice {
    use alloc::{
        format,
        string::{String, ToString},
    };

    use rand::{RngExt as _, rand_core::UnwrapErr, rngs::SysRng};

    use super::Dice;

    pub fn display(dice: &[Dice]) -> String {
        let s = dice
            .iter()
            .map(|&(count, sides, modifier)| {
                let dice_str = if count == 0 || sides == 0 {
                    String::new()
                } else {
                    format!("{count}D{sides}")
                };
                let modifier_str = match modifier {
                    0 => String::new(),
                    m if m > 0 => format!("+{m}"),
                    m => format!("{m}"),
                };
                format!("{dice_str}{modifier_str}")
            })
            .collect::<String>();
        s.trim_start_matches('+').to_string()
    }
    pub fn roll(dice: &[Dice]) -> i32 {
        dice.iter()
            .map(|&(count, sides, modifier)| {
                let rolled = if count != 0 && sides > 0 {
                    let mut rng = UnwrapErr(SysRng);
                    let sum: i32 =
                        (0..count.unsigned_abs()).map(|_| rng.random_range(1..=sides as i32)).sum();
                    if count < 0 { -sum } else { sum }
                } else {
                    0
                };
                rolled + modifier as i32
            })
            .sum()
    }
    pub fn percent_roll(level: i8) -> u8 {
        let mut rng = UnwrapErr(SysRng);
        let ones: u8 = rng.random_range(0..=9u8);
        let offset: u8 = if ones == 0 { 1 } else { 0 };
        let rolls_count = 1 + level.unsigned_abs() as usize;
        let tens = if level > 0 {
            (0..rolls_count).map(|_| rng.random_range(0..=9u8) + offset).min().unwrap()
        } else if level < 0 {
            (0..rolls_count).map(|_| rng.random_range(0..=9u8) + offset).max().unwrap()
        } else {
            rng.random_range(0..=9u8) + offset
        };
        tens * 10 + ones
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Profile,
    Characteristic,
    SecondaryAttribute,
    Skill,
    Possession,
    Backstory,
    Memo,
}

impl Section {
    pub const fn number(&self) -> u32 {
        match self {
            Self::Profile => 10,
            Self::Characteristic => 20,
            Self::SecondaryAttribute => 30,
            Self::Skill => 40,
            Self::Possession => 150,
            Self::Backstory => 160,
            Self::Memo => 170,
        }
    }

    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Profile, Language::En(_)) => "Profile",
            (Self::Profile, Language::Ja) => "プロフィール",
            (Self::Characteristic, Language::En(_)) => "Characteristics",
            (Self::Characteristic, Language::Ja) => "能力値",
            (Self::SecondaryAttribute, Language::En(_)) => "Secondary Attributes",
            (Self::SecondaryAttribute, Language::Ja) => "ほかの属性",
            (Self::Skill, Language::En(_)) => "Skills",
            (Self::Skill, Language::Ja) => "技能",
            (Self::Possession, Language::En(_)) => "Gear & Possessions",
            (Self::Possession, Language::Ja) => "装備と所持品",
            (Self::Backstory, Language::En(_)) => "Backstory",
            (Self::Backstory, Language::Ja) => "バックストーリー",
            (Self::Memo, Language::En(_)) => "Memo",
            (Self::Memo, Language::Ja) => "メモ",
        }
    }

    pub fn list() -> &'static [Section] {
        &[
            Self::Profile,
            Self::Characteristic,
            Self::SecondaryAttribute,
            Self::Skill,
            Self::Possession,
            Self::Backstory,
            Self::Memo,
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    Name,
    Birthplace,
    Pronoun,
    Occupation,
    Residence,
    Age,
}

impl Profile {
    pub fn numbers(&self) -> &'static [u32] {
        const BASE: u32 = Section::Profile.number();
        match self {
            Self::Name => &[BASE, BASE + 1],
            Self::Birthplace => &[BASE + 2],
            Self::Pronoun => &[BASE + 3],
            Self::Occupation => &[BASE + 4, BASE + 5, BASE + 6],
            Self::Residence => &[BASE + 7],
            Self::Age => &[BASE + 8],
        }
    }

    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Name, Language::En(_)) => "Name",
            (Self::Name, Language::Ja) => "名前",
            (Self::Birthplace, Language::En(_)) => "Birthplace",
            (Self::Birthplace, Language::Ja) => "出身",
            (Self::Pronoun, Language::En(_)) => "Pronoun",
            (Self::Pronoun, Language::Ja) => "性別",
            (Self::Occupation, Language::En(_)) => "Occupation",
            (Self::Occupation, Language::Ja) => "職業",
            (Self::Residence, Language::En(_)) => "Residence",
            (Self::Residence, Language::Ja) => "住所",
            (Self::Age, Language::En(_)) => "Age",
            (Self::Age, Language::Ja) => "年齢",
        }
    }

    pub fn list() -> &'static [Self] {
        &[Self::Name, Self::Birthplace, Self::Pronoun, Self::Occupation, Self::Residence, Self::Age]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccupationKind {
    // p.38
    Activist = 1,
    Antiquarian,
    Artist,
    Athlete,
    Author,
    Clergy,
    Criminal,
    Dilettante,
    Doctor,
    Drifter,
    Engineer,
    Entertainer,
    Farmer,
    Hacker,
    Journalist,
    Lawyer,
    Librarian,
    MilitaryOfficer,
    Missionary,
    Musician,
    Parapsychologist,
    Pilot,
    Police,
    PoliceDetective,
    PrivateInvestigator,
    Professor,
    Soldier,
    TribeMember,
    Custom,
}

impl OccupationKind {
    pub fn detect(kind_id: u8) -> Self {
        match kind_id {
            1 => OccupationKind::Activist,
            2 => OccupationKind::Antiquarian,
            3 => OccupationKind::Artist,
            4 => OccupationKind::Athlete,
            5 => OccupationKind::Author,
            6 => OccupationKind::Clergy,
            7 => OccupationKind::Criminal,
            8 => OccupationKind::Dilettante,
            9 => OccupationKind::Doctor,
            10 => OccupationKind::Drifter,
            11 => OccupationKind::Engineer,
            12 => OccupationKind::Entertainer,
            13 => OccupationKind::Farmer,
            14 => OccupationKind::Hacker,
            15 => OccupationKind::Journalist,
            16 => OccupationKind::Lawyer,
            17 => OccupationKind::Librarian,
            18 => OccupationKind::MilitaryOfficer,
            19 => OccupationKind::Missionary,
            20 => OccupationKind::Musician,
            21 => OccupationKind::Parapsychologist,
            22 => OccupationKind::Pilot,
            23 => OccupationKind::Police,
            24 => OccupationKind::PoliceDetective,
            25 => OccupationKind::PrivateInvestigator,
            26 => OccupationKind::Professor,
            27 => OccupationKind::Soldier,
            28 => OccupationKind::TribeMember,
            _ => OccupationKind::Custom,
        }
    }

    pub fn id(&self) -> u8 {
        *self as u8
    }
}

pub struct Occupation;

impl Occupation {
    pub fn display(
        kind: &OccupationKind,
        custom_name: Option<&str>,
        title: Option<&str>,
        language: Language,
    ) -> String {
        let name = match (kind, language) {
            (OccupationKind::Activist, Language::En(_)) => "Activist",
            (OccupationKind::Activist, Language::Ja) => "活動家",
            (OccupationKind::Antiquarian, Language::En(_)) => "Antiquarian",
            (OccupationKind::Antiquarian, Language::Ja) => "古物研究家",
            (OccupationKind::Artist, Language::En(_)) => "Artist",
            (OccupationKind::Artist, Language::Ja) => "芸術家",
            (OccupationKind::Athlete, Language::En(_)) => "Athlete",
            (OccupationKind::Athlete, Language::Ja) => "スポーツ選手",
            (OccupationKind::Author, Language::En(_)) => "Author",
            (OccupationKind::Author, Language::Ja) => "作家",
            (OccupationKind::Clergy, Language::En(_)) => "Clergy",
            (OccupationKind::Clergy, Language::Ja) => "聖職者",
            (OccupationKind::Criminal, Language::En(_)) => "Criminal",
            (OccupationKind::Criminal, Language::Ja) => "犯罪者",
            (OccupationKind::PoliceDetective, Language::En(_)) => "Police Detective",
            (OccupationKind::PoliceDetective, Language::Ja) => "刑事",
            (OccupationKind::Dilettante, Language::En(_)) => "Dilettante",
            (OccupationKind::Dilettante, Language::Ja) => "ディレッタント",
            (OccupationKind::Doctor, Language::En(_)) => "Doctor",
            (OccupationKind::Doctor, Language::Ja) => "医師",
            (OccupationKind::Drifter, Language::En(_)) => "Drifter",
            (OccupationKind::Drifter, Language::Ja) => "放浪者",
            (OccupationKind::Engineer, Language::En(_)) => "Engineer",
            (OccupationKind::Engineer, Language::Ja) => "技術者",
            (OccupationKind::Entertainer, Language::En(_)) => "Entertainer",
            (OccupationKind::Entertainer, Language::Ja) => "エンターテイナー",
            (OccupationKind::Farmer, Language::En(_)) => "Farmer",
            (OccupationKind::Farmer, Language::Ja) => "農民",
            (OccupationKind::Hacker, Language::En(_)) => "Hacker",
            (OccupationKind::Hacker, Language::Ja) => "ハッカー",
            (OccupationKind::Journalist, Language::En(_)) => "Journalist",
            (OccupationKind::Journalist, Language::Ja) => "ジャーナリスト",
            (OccupationKind::Lawyer, Language::En(_)) => "Lawyer",
            (OccupationKind::Lawyer, Language::Ja) => "弁護士",
            (OccupationKind::Librarian, Language::En(_)) => "Librarian",
            (OccupationKind::Librarian, Language::Ja) => "司書",
            (OccupationKind::MilitaryOfficer, Language::En(_)) => "Military Officer",
            (OccupationKind::MilitaryOfficer, Language::Ja) => "士官",
            (OccupationKind::Missionary, Language::En(_)) => "Missionary",
            (OccupationKind::Missionary, Language::Ja) => "伝道者",
            (OccupationKind::Musician, Language::En(_)) => "Musician",
            (OccupationKind::Musician, Language::Ja) => "ミュージシャン",
            (OccupationKind::Parapsychologist, Language::En(_)) => "Parapsychologist",
            (OccupationKind::Parapsychologist, Language::Ja) => "超心理学者",
            (OccupationKind::Pilot, Language::En(_)) => "Pilot",
            (OccupationKind::Pilot, Language::Ja) => "パイロット",
            (OccupationKind::Police, Language::En(_)) => "Police",
            (OccupationKind::Police, Language::Ja) => "警察官",
            (OccupationKind::PrivateInvestigator, Language::En(_)) => "Private Investigator",
            (OccupationKind::PrivateInvestigator, Language::Ja) => "私立探偵",
            (OccupationKind::Professor, Language::En(_)) => "Professor",
            (OccupationKind::Professor, Language::Ja) => "教授",
            (OccupationKind::Soldier, Language::En(_)) => "Soldier",
            (OccupationKind::Soldier, Language::Ja) => "兵士",
            (OccupationKind::TribeMember, Language::En(_)) => "Tribe Member",
            (OccupationKind::TribeMember, Language::Ja) => "トライブ・メンバー",
            (OccupationKind::Custom, _) => custom_name.unwrap_or(""),
        };
        match title {
            Some(t) if !t.is_empty() => format!("{name} ({t})"),
            _ => name.to_string(),
        }
    }

    pub fn list() -> &'static [OccupationKind] {
        &[
            OccupationKind::Activist,
            OccupationKind::Antiquarian,
            OccupationKind::Artist,
            OccupationKind::Athlete,
            OccupationKind::Author,
            OccupationKind::Clergy,
            OccupationKind::Criminal,
            OccupationKind::PoliceDetective,
            OccupationKind::Dilettante,
            OccupationKind::Doctor,
            OccupationKind::Drifter,
            OccupationKind::Engineer,
            OccupationKind::Entertainer,
            OccupationKind::Farmer,
            OccupationKind::Hacker,
            OccupationKind::Journalist,
            OccupationKind::Lawyer,
            OccupationKind::Librarian,
            OccupationKind::MilitaryOfficer,
            OccupationKind::Missionary,
            OccupationKind::Musician,
            OccupationKind::Parapsychologist,
            OccupationKind::Pilot,
            OccupationKind::Police,
            OccupationKind::PrivateInvestigator,
            OccupationKind::Professor,
            OccupationKind::Soldier,
            OccupationKind::TribeMember,
            OccupationKind::Custom,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Characteristic {
    // Characteristic {initial: u16, change: i16, modifier: i16} // p.28
    Strength = 1,
    Constitution,
    Size,
    Dexterity,
    Appearance,
    Intelligence,
    Power,
    Education,
}

impl Characteristic {
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Strength, _) => "STR",
            (Self::Constitution, _) => "CON",
            (Self::Size, _) => "SIZ",
            (Self::Dexterity, _) => "DEX",
            (Self::Appearance, _) => "APP",
            (Self::Intelligence, _) => "INT",
            (Self::Power, _) => "POW",
            (Self::Education, _) => "EDU",
        }
    }

    pub const fn number(&self) -> u32 {
        Section::Characteristic.number() + *self as u32
    }

    pub fn from_number(number: u32) -> Option<Self> {
        Self::list().iter().find(|characteristic| characteristic.number() == number).copied()
    }

    pub const fn index(&self) -> usize {
        *self as usize - 1
    }

    pub fn score(&self, investigator: &Investigator) -> Score {
        investigator.characteristics[self.index()]
    }

    pub fn sum(&self, investigator: &Investigator) -> i32 {
        self.score(investigator).sum()
    }

    pub fn target(&self, investigator: &Investigator) -> (i32, i32, i32) {
        let sum = self.sum(investigator);
        (sum, (sum as f64 * 0.5) as i32, (sum as f64 * 0.2) as i32)
    }

    pub fn list() -> &'static [Characteristic] {
        &[
            Self::Strength,
            Self::Constitution,
            Self::Size,
            Self::Dexterity,
            Self::Appearance,
            Self::Intelligence,
            Self::Power,
            Self::Education,
        ]
    }

    pub fn roll_initial(&self) -> u16 {
        match self {
            Self::Size | Self::Intelligence | Self::Education => {
                dice::roll(&[(2, 6, 6)]) as u16 * 5
            }
            _ => dice::roll(&[(3, 6, 0)]) as u16 * 5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Score {
    pub initial:  u16,
    pub change:   i16,
    pub modifier: i16,
}

impl Zero for Score {
    const ZERO: Self = Self { initial: 0, change: 0, modifier: 0 };
}

impl Score {
    pub fn sum(&self) -> i32 {
        (self.initial as i32 + self.change as i32 + self.modifier as i32).max(1)
    }
}

impl Input<'_> for Score {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            initial:  u16::decode(input)?,
            change:   i16::decode(input)?,
            modifier: i16::decode(input)?,
        })
    }
}

impl Output for Score {
    fn encode(&self, output: &mut Vec<u8>) {
        self.initial.encode(output);
        self.change.encode(output);
        self.modifier.encode(output);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SecondaryAttribute {
    HitPoints = 1,         // CON, SIZ -> u8
    MagicPoints,           // POW -> u8
    Luck,                  // u8
    Sanity,                // u8 | POW -> u8
    Build,                 // STR, SIZ -> i8
    DamageBonus,           // Build -> Dice
    MoveRate,              // u8 | STR, DEX, SIZ, Age -> u8
    OccupationSkillPoints, // (Characteristic, Characteristic) | (Characteristic, Characteristic) -> (u16, u16)
    InterestSkillPoints,   // INT -> (u16, u16)
}

impl SecondaryAttribute {
    pub const fn number(&self) -> u32 {
        Section::SecondaryAttribute.number() + *self as u32
    }

    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::HitPoints, _) => "HP",
            (Self::MagicPoints, _) => "MP",
            (Self::Luck, Language::En(_)) => "Luck",
            (Self::Luck, Language::Ja) => "幸運",
            (Self::Sanity, Language::En(_)) => "Sanity",
            (Self::Sanity, Language::Ja) => "正気度",
            (Self::Build, Language::En(_)) => "Build",
            (Self::Build, Language::Ja) => "ビルド",
            (Self::DamageBonus, Language::En(_)) => "Damage Bonus",
            (Self::DamageBonus, Language::Ja) => "ダメージボーナス",
            (Self::MoveRate, Language::En(_)) => "Move Rate",
            (Self::MoveRate, Language::Ja) => "移動率 (MOV)",
            (Self::OccupationSkillPoints, Language::En(_)) => "Occupation Skill Points",
            (Self::OccupationSkillPoints, Language::Ja) => "職業技能ポイント",
            (Self::InterestSkillPoints, Language::En(_)) => "Interest Skill Points",
            (Self::InterestSkillPoints, Language::Ja) => "興味技能ポイント",
        }
    }
}

pub struct HitPoints;

impl HitPoints {
    pub fn derive(investigator: &Investigator) -> u8 {
        let constitution = Characteristic::Constitution.sum(investigator);
        let size = Characteristic::Size.sum(investigator);
        ((constitution + size) / 10) as u8
    }
}

pub struct MagicPoints;

impl MagicPoints {
    pub fn derive(investigator: &Investigator) -> u8 {
        (Characteristic::Power.sum(investigator) / 5) as u8
    }
}

pub struct Luck;

impl Luck {
    pub fn roll() -> u8 {
        (dice::roll(&[(3, 6, 0)]) * 5) as u8
    }
}

pub struct Sanity;

impl Sanity {
    pub fn derive(investigator: &Investigator) -> u8 {
        Characteristic::Power.score(investigator).initial as u8
    }
}

pub struct Build;

impl Build {
    pub fn derive(investigator: &Investigator) -> i8 {
        let strength = Characteristic::Strength.sum(investigator);
        let size = Characteristic::Size.sum(investigator);
        match strength + size {
            2..=64 => -2,
            65..=84 => -1,
            85..=124 => 0,
            125..=164 => 1,
            165..=204 => 2,
            205..=284 => 3,
            285..=364 => 4,
            365..=444 => 5,
            445..=524 => 6,
            n => (7 + (n - 525) / 80) as i8,
        }
    }
}

pub struct DamageBonus;

impl DamageBonus {
    pub fn display(investigator: &Investigator) -> String {
        dice::display(&[Self::derive(investigator)])
    }

    pub fn derive(investigator: &Investigator) -> Dice {
        match investigator.build {
            i8::MIN..=-2 => (0, 0, -2),
            -1 => (0, 0, -1),
            0 => (0, 0, 0),
            1 => (1, 4, 0),
            2 => (1, 6, 0),
            3 => (2, 6, 0),
            4 => (3, 6, 0),
            5 => (4, 6, 0),
            6 => (5, 6, 0),
            n => (n - 2, 6, 0),
        }
    }
}

pub struct MoveRate;

impl MoveRate {
    pub fn derive(investigator: &Investigator) -> u8 {
        let str = Characteristic::Strength.sum(investigator);
        let dex = Characteristic::Dexterity.sum(investigator);
        let siz = Characteristic::Size.sum(investigator);
        let base: i32 = if str > siz && dex > siz {
            9
        } else if str < siz && dex < siz {
            7
        } else {
            8
        };
        let age_penalty: i32 = match investigator.age {
            40..=49 => 1,
            50..=59 => 2,
            60..=69 => 3,
            70..=79 => 4,
            80.. => 5,
            _ => 0,
        };
        (base - age_penalty).max(0) as u8
    }
}

pub struct OccupationSkillPoints;

impl OccupationSkillPoints {
    pub fn display(
        characteristics: Option<(Characteristic, Characteristic)>,
        language: Language,
    ) -> String {
        let Some((c1, c2)) = characteristics else {
            return String::new();
        };
        if c1 == c2 {
            format!("{}×4", c1.display(language))
        } else {
            format!("{}×2+{}×2", c1.display(language), c2.display(language))
        }
    }

    pub fn derive(investigator: &Investigator) -> (u16, u16) {
        let Some((c1, c2)) = investigator.occupation_skill_points else {
            return (0, 0);
        };
        let used = 0u16;
        let (c1, c2) = (c1.score(investigator), c2.score(investigator));
        let total =
            (c1.initial as i32 + c1.modifier as i32 + c2.initial as i32 + c2.modifier as i32) * 2;
        (used, total.max(0) as u16)
    }
}

pub struct InterestSkillPoints;

impl InterestSkillPoints {
    pub fn derive(investigator: &Investigator) -> (u16, u16) {
        let used = 0u16;
        let score = Characteristic::Intelligence.score(investigator);
        let total = ((score.initial as i32 + score.modifier as i32) * 2).max(0) as u16;
        (used, total)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Skill {
    Accounting,
    Anthropology,
    Appraise,
    Archaeology,
    ArtAndCraft,
    Charm,
    Climb,
    ComputerUse,
    CreditRating,
    CthulhuMythos,
    Disguise,
    Dodge,
    DriveAuto,
    ElectricalRepair,
    Electronics,
    FastTalk,
    Fighting,
    Firearms,
    FirstAid,
    History,
    Intimidate,
    Jump,
    LanguageOther,
    LanguageOwn,
    Law,
    LibraryUse,
    Listen,
    Locksmith,
    MechanicalRepair,
    Medicine,
    NaturalWorld,
    Navigate,
    Occult,
    Persuade,
    Pilot,
    Psychoanalysis,
    Psychology,
    Ride,
    Science,
    SleightOfHand,
    SpotHidden,
    Stealth,
    Survival,
    Swim,
    Throw,
    Track,
    Custom,
}

impl Skill {
    pub fn list() -> &'static [Skill] {
        &[
            Self::Accounting,
            Self::Anthropology,
            Self::Appraise,
            Self::Archaeology,
            Self::ArtAndCraft,
            Self::Charm,
            Self::Climb,
            Self::ComputerUse,
            Self::CreditRating,
            Self::CthulhuMythos,
            Self::Disguise,
            Self::Dodge,
            Self::DriveAuto,
            Self::ElectricalRepair,
            Self::Electronics,
            Self::FastTalk,
            Self::Fighting,
            Self::Firearms,
            Self::FirstAid,
            Self::History,
            Self::Intimidate,
            Self::Jump,
            Self::LanguageOther,
            Self::LanguageOwn,
            Self::Law,
            Self::LibraryUse,
            Self::Listen,
            Self::Locksmith,
            Self::MechanicalRepair,
            Self::Medicine,
            Self::NaturalWorld,
            Self::Navigate,
            Self::Occult,
            Self::Persuade,
            Self::Pilot,
            Self::Psychoanalysis,
            Self::Psychology,
            Self::Ride,
            Self::Science,
            Self::SleightOfHand,
            Self::SpotHidden,
            Self::Stealth,
            Self::Survival,
            Self::Swim,
            Self::Throw,
            Self::Track,
            Self::Custom,
        ]
    }

    pub const fn row(&self) -> u32 {
        const BASE: u32 = Section::Skill.number();
        match self {
            Self::Accounting => BASE + 1, // 1 slot (minimum)
            Self::Anthropology => BASE + 2,
            Self::Appraise => BASE + 3,
            Self::Archaeology => BASE + 4,
            Self::ArtAndCraft => BASE + 5, // 12 + 1 slots (defined + custom)
            Self::Charm => BASE + 19,
            Self::Climb => BASE + 20,
            Self::ComputerUse => BASE + 21,
            Self::CreditRating => BASE + 22,
            Self::CthulhuMythos => BASE + 23,
            Self::Disguise => BASE + 24,
            Self::Dodge => BASE + 25,
            Self::DriveAuto => BASE + 26,
            Self::ElectricalRepair => BASE + 27,
            Self::Electronics => BASE + 28,
            Self::FastTalk => BASE + 29,
            Self::Fighting => BASE + 30, // 8 + 1 slots (defined + custom)
            Self::Firearms => BASE + 40, // 7 + 1 slots (defined + custom)
            Self::FirstAid => BASE + 49,
            Self::History => BASE + 50,
            Self::Intimidate => BASE + 51,
            Self::Jump => BASE + 52,
            Self::LanguageOther => BASE + 53, // 1 + 1 slot (+ name)
            Self::LanguageOwn => BASE + 55,   // 1 + 1 slot (+ name)
            Self::Law => BASE + 57,
            Self::LibraryUse => BASE + 58,
            Self::Listen => BASE + 59,
            Self::Locksmith => BASE + 60,
            Self::MechanicalRepair => BASE + 61,
            Self::Medicine => BASE + 62,
            Self::NaturalWorld => BASE + 63,
            Self::Navigate => BASE + 64,
            Self::Occult => BASE + 65,
            Self::Persuade => BASE + 66,
            Self::Pilot => BASE + 67, // 10 + 1 slots (defined + custom)
            Self::Psychoanalysis => BASE + 79,
            Self::Psychology => BASE + 80,
            Self::Ride => BASE + 81,
            Self::Science => BASE + 82, // 13 + 1 slots (defined + custom)
            Self::SleightOfHand => BASE + 97,
            Self::SpotHidden => BASE + 98,
            Self::Stealth => BASE + 99,
            Self::Survival => BASE + 100, // 3 + 1 slots (defined + custom)
            Self::Swim => BASE + 105,
            Self::Throw => BASE + 106,
            Self::Track => BASE + 107,
            Self::Custom => BASE + 108,
        }
    }

    pub const fn base_percent(&self) -> u16 {
        match self {
            Self::Accounting => 5,
            Self::Anthropology => 1,
            Self::Appraise => 5,
            Self::Archaeology => 1,
            Self::ArtAndCraft => 5,
            Self::Charm => 15,
            Self::Climb => 20,
            Self::ComputerUse => 5,
            Self::CreditRating => 0,
            Self::CthulhuMythos => 0,
            Self::Disguise => 5,
            Self::DriveAuto => 20,
            Self::ElectricalRepair => 10,
            Self::Electronics => 1,
            Self::FastTalk => 5,
            Self::FirstAid => 30,
            Self::History => 5,
            Self::Intimidate => 15,
            Self::Jump => 20,
            Self::LanguageOther => 1,
            Self::Law => 5,
            Self::LibraryUse => 20,
            Self::Listen => 20,
            Self::Locksmith => 1,
            Self::MechanicalRepair => 10,
            Self::Medicine => 1,
            Self::NaturalWorld => 10,
            Self::Navigate => 10,
            Self::Occult => 5,
            Self::Persuade => 10,
            Self::Psychoanalysis => 1,
            Self::Psychology => 10,
            Self::Ride => 5,
            Self::Science => 1,
            Self::SleightOfHand => 10,
            Self::SpotHidden => 25,
            Self::Stealth => 20,
            Self::Survival => 5,
            Self::Swim => 20,
            Self::Throw => 20,
            Self::Track => 10,
            Self::Custom => 0,
            _ => 0,
        }
    }

    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Accounting, Language::En(_)) => "Accounting",
            (Self::Accounting, Language::Ja) => "経理",
            (Self::Anthropology, Language::En(_)) => "Anthropology",
            (Self::Anthropology, Language::Ja) => "人類学",
            (Self::Appraise, Language::En(_)) => "Appraise",
            (Self::Appraise, Language::Ja) => "鑑定",
            (Self::Archaeology, Language::En(_)) => "Archaeology",
            (Self::Archaeology, Language::Ja) => "考古学",
            (Self::ArtAndCraft, Language::En(_)) => "Art / Craft",
            (Self::ArtAndCraft, Language::Ja) => "芸術/製作",
            (Self::Charm, Language::En(_)) => "Charm",
            (Self::Charm, Language::Ja) => "魅惑",
            (Self::Climb, Language::En(_)) => "Climb",
            (Self::Climb, Language::Ja) => "登攀",
            (Self::ComputerUse, Language::En(_)) => "Computer Use",
            (Self::ComputerUse, Language::Ja) => "コンピューター",
            (Self::CreditRating, Language::En(_)) => "Credit Rating",
            (Self::CreditRating, Language::Ja) => "信用",
            (Self::CthulhuMythos, Language::En(_)) => "Cthulhu Mythos",
            (Self::CthulhuMythos, Language::Ja) => "クトゥルフ神話",
            (Self::Disguise, Language::En(_)) => "Disguise",
            (Self::Disguise, Language::Ja) => "変装",
            (Self::Dodge, Language::En(_)) => "Dodge",
            (Self::Dodge, Language::Ja) => "回避",
            (Self::DriveAuto, Language::En(_)) => "Drive Auto",
            (Self::DriveAuto, Language::Ja) => "運転（自動車）",
            (Self::ElectricalRepair, Language::En(_)) => "Elec. Repair",
            (Self::ElectricalRepair, Language::Ja) => "電気修理",
            (Self::Electronics, Language::En(_)) => "Electronics",
            (Self::Electronics, Language::Ja) => "電子工学",
            (Self::FastTalk, Language::En(_)) => "Fast Talk",
            (Self::FastTalk, Language::Ja) => "言いくるめ",
            (Self::Fighting, Language::En(_)) => "Fighting",
            (Self::Fighting, Language::Ja) => "近接戦闘",
            (Self::Firearms, Language::En(_)) => "Firearms",
            (Self::Firearms, Language::Ja) => "射撃",
            (Self::FirstAid, Language::En(_)) => "First Aid",
            (Self::FirstAid, Language::Ja) => "応急手当",
            (Self::History, Language::En(_)) => "History",
            (Self::History, Language::Ja) => "歴史",
            (Self::Intimidate, Language::En(_)) => "Intimidate",
            (Self::Intimidate, Language::Ja) => "威圧",
            (Self::Jump, Language::En(_)) => "Jump",
            (Self::Jump, Language::Ja) => "跳躍",
            (Self::LanguageOther, Language::En(_)) => "Language (Other)",
            (Self::LanguageOther, Language::Ja) => "ほかの言語",
            (Self::LanguageOwn, Language::En(_)) => "Language (Own)",
            (Self::LanguageOwn, Language::Ja) => "母国語",
            (Self::Law, Language::En(_)) => "Law",
            (Self::Law, Language::Ja) => "法律",
            (Self::LibraryUse, Language::En(_)) => "Library Use",
            (Self::LibraryUse, Language::Ja) => "図書館",
            (Self::Listen, Language::En(_)) => "Listen",
            (Self::Listen, Language::Ja) => "聞き耳",
            (Self::Locksmith, Language::En(_)) => "Locksmith",
            (Self::Locksmith, Language::Ja) => "鍵開け",
            (Self::MechanicalRepair, Language::En(_)) => "Mech. Repair",
            (Self::MechanicalRepair, Language::Ja) => "機械修理",
            (Self::Medicine, Language::En(_)) => "Medicine",
            (Self::Medicine, Language::Ja) => "医学",
            (Self::NaturalWorld, Language::En(_)) => "Natural World",
            (Self::NaturalWorld, Language::Ja) => "自然",
            (Self::Navigate, Language::En(_)) => "Navigate",
            (Self::Navigate, Language::Ja) => "ナビゲート",
            (Self::Occult, Language::En(_)) => "Occult",
            (Self::Occult, Language::Ja) => "オカルト",
            (Self::Persuade, Language::En(_)) => "Persuade",
            (Self::Persuade, Language::Ja) => "説得",
            (Self::Pilot, Language::En(_)) => "Pilot",
            (Self::Pilot, Language::Ja) => "操縦",
            (Self::Psychoanalysis, Language::En(_)) => "Psychoanalysis",
            (Self::Psychoanalysis, Language::Ja) => "精神分析",
            (Self::Psychology, Language::En(_)) => "Psychology",
            (Self::Psychology, Language::Ja) => "心理学",
            (Self::Ride, Language::En(_)) => "Ride",
            (Self::Ride, Language::Ja) => "乗馬",
            (Self::Science, Language::En(_)) => "Science",
            (Self::Science, Language::Ja) => "科学",
            (Self::SleightOfHand, Language::En(_)) => "Sleight of Hand",
            (Self::SleightOfHand, Language::Ja) => "手さばき",
            (Self::SpotHidden, Language::En(_)) => "Spot Hidden",
            (Self::SpotHidden, Language::Ja) => "目星",
            (Self::Stealth, Language::En(_)) => "Stealth",
            (Self::Stealth, Language::Ja) => "隠密",
            (Self::Survival, Language::En(_)) => "Survival",
            (Self::Survival, Language::Ja) => "サバイバル",
            (Self::Swim, Language::En(_)) => "Swim",
            (Self::Swim, Language::Ja) => "水泳",
            (Self::Throw, Language::En(_)) => "Throw",
            (Self::Throw, Language::Ja) => "投擲",
            (Self::Track, Language::En(_)) => "Track",
            (Self::Track, Language::Ja) => "追跡",
            (_, _) => "",
        }
    }
}

bit::field! {
    OccupationPoints(u9),
    InterestPoints(u9),
    Change(i10),
    Modifier(i10),
}

bit::composite! {
    struct SkillValue { OccupationPoints, InterestPoints, Change, Modifier }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Points {
    pub occupation: u16,
    pub interest:   u16,
    pub change:     i16,
    pub modifier:   i16,
}

impl Zero for Points {
    const ZERO: Self = Self { occupation: 0, interest: 0, change: 0, modifier: 0 };
}

impl Points {
    pub fn sum(&self, base: u16) -> i32 {
        base as i32
            + self.occupation as i32
            + self.interest as i32
            + self.change as i32
            + self.modifier as i32
    }

    pub fn tuple(&self) -> (u16, u16, i16, i16) {
        (self.occupation, self.interest, self.change, self.modifier)
    }
}

impl From<(u16, u16, i16, i16)> for Points {
    fn from((occupation, interest, change, modifier): (u16, u16, i16, i16)) -> Self {
        Self { occupation, interest, change, modifier }
    }
}

impl Input<'_> for Points {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let value = SkillValue::from_le_bytes(take(input, SkillValue::BYTES)?.try_into().ok()?);
        Some(Self {
            occupation: value.get::<OccupationPoints>().0.value(),
            interest:   value.get::<InterestPoints>().0.value(),
            change:     value.get::<Change>().0.value(),
            modifier:   value.get::<Modifier>().0.value(),
        })
    }
}

impl Output for Points {
    fn encode(&self, output: &mut Vec<u8>) {
        let mut value = SkillValue::ZERO;
        value.set(OccupationPoints(u9::masked_new(self.occupation)));
        value.set(InterestPoints(u9::masked_new(self.interest)));
        value.set(Change(i10::masked_new(self.change)));
        value.set(Modifier(i10::masked_new(self.modifier)));
        value.encode(output);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Specialty {
    ArtAndCraft(ArtAndCraft),
    Fighting(Fighting),
    Firearms(Firearms),
    Pilot(Pilot),
    Science(Science),
    Survival(Survival),
    LanguageOwn,
    LanguageOther,
    Custom,
}

impl Specialty {
    pub fn skill(&self) -> Skill {
        match self {
            Self::ArtAndCraft(_) => Skill::ArtAndCraft,
            Self::Fighting(_) => Skill::Fighting,
            Self::Firearms(_) => Skill::Firearms,
            Self::Pilot(_) => Skill::Pilot,
            Self::Science(_) => Skill::Science,
            Self::Survival(_) => Skill::Survival,
            Self::LanguageOwn => Skill::LanguageOwn,
            Self::LanguageOther => Skill::LanguageOther,
            Self::Custom => Skill::Custom,
        }
    }

    pub fn custom(skill: &Skill) -> Option<Self> {
        Some(match skill {
            Skill::ArtAndCraft => Self::ArtAndCraft(ArtAndCraft::Custom),
            Skill::Fighting => Self::Fighting(Fighting::Custom),
            Skill::Firearms => Self::Firearms(Firearms::Custom),
            Skill::Pilot => Self::Pilot(Pilot::Custom),
            Skill::Science => Self::Science(Science::Custom),
            Skill::Survival => Self::Survival(Survival::Custom),
            Skill::LanguageOwn => Self::LanguageOwn,
            Skill::LanguageOther => Self::LanguageOther,
            Skill::Custom => Self::Custom,
            _ => return None,
        })
    }

    pub fn known(&self) -> bool {
        !matches!(
            self,
            Self::ArtAndCraft(ArtAndCraft::Custom)
                | Self::Fighting(Fighting::Custom)
                | Self::Firearms(Firearms::Custom)
                | Self::Pilot(Pilot::Custom)
                | Self::Science(Science::Custom)
                | Self::Survival(Survival::Custom)
                | Self::LanguageOwn
                | Self::LanguageOther
                | Self::Custom
        )
    }

    pub fn display(&self, language: Language) -> &str {
        match self {
            Self::ArtAndCraft(a) if self.known() => a.display(language),
            Self::Fighting(f) if self.known() => f.display(language),
            Self::Firearms(f) if self.known() => f.display(language),
            Self::Pilot(p) if self.known() => p.display(language),
            Self::Science(s) if self.known() => s.display(language).unwrap_or(""),
            Self::Survival(s) if self.known() => s.display(language),
            _ => "",
        }
    }

    pub fn base_percent(&self) -> u16 {
        match self {
            Self::Fighting(f) if self.known() => f.base_percent(),
            Self::Firearms(f) if self.known() => f.base_percent(),
            Self::Survival(s) if self.known() => s.base_percent(),
            other => other.skill().base_percent(),
        }
    }

    fn number(&self) -> u8 {
        match self {
            Self::ArtAndCraft(a) => a.number() as u8,
            Self::Fighting(f) => f.number() as u8,
            Self::Firearms(f) => f.number() as u8,
            Self::Pilot(p) => p.number() as u8,
            Self::Science(s) => s.number() as u8,
            Self::Survival(s) => s.number() as u8,
            _ => 0,
        }
    }

    fn from_numbers(skill: u8, number: u8) -> Option<Self> {
        let number = u32::from(number);
        Some(match Skill::list().get(usize::from(skill))? {
            Skill::ArtAndCraft => Self::ArtAndCraft(
                *once(&ArtAndCraft::Custom)
                    .chain(ArtAndCraft::list())
                    .find(|a| a.number() == number)?,
            ),
            Skill::Fighting => Self::Fighting(
                *once(&Fighting::Custom).chain(Fighting::list()).find(|f| f.number() == number)?,
            ),
            Skill::Firearms => Self::Firearms(
                *once(&Firearms::Custom).chain(Firearms::list()).find(|f| f.number() == number)?,
            ),
            Skill::Pilot => Self::Pilot(
                *once(&Pilot::Custom).chain(Pilot::list()).find(|p| p.number() == number)?,
            ),
            Skill::Science => Self::Science(
                *once(&Science::Custom).chain(Science::list()).find(|s| s.number() == number)?,
            ),
            Skill::Survival => Self::Survival(
                *once(&Survival::Custom).chain(Survival::list()).find(|s| s.number() == number)?,
            ),
            Skill::LanguageOwn => Self::LanguageOwn,
            Skill::LanguageOther => Self::LanguageOther,
            Skill::Custom => Self::Custom,
            _ => return None,
        })
    }
}

impl Input<'_> for Specialty {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let skill = u8::decode(input)?;
        let number = u8::decode(input)?;
        Self::from_numbers(skill, number)
    }
}

impl Output for Specialty {
    fn encode(&self, output: &mut Vec<u8>) {
        (self.skill().index() as u8).encode(output);
        self.number().encode(output);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Specialization {
    pub specialty: Specialty,
    pub name:      String,
    pub base:      u8,
    pub points:    Points,
}

impl Input<'_> for Specialization {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            specialty: Specialty::decode(input)?,
            name:      String::decode(input)?,
            base:      u8::decode(input)?,
            points:    Points::decode(input)?,
        })
    }
}

impl Output for Specialization {
    fn encode(&self, output: &mut Vec<u8>) {
        self.specialty.encode(output);
        self.name.encode(output);
        self.base.encode(output);
        self.points.encode(output);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memo {
    pub title: String,
    pub body:  String,
}

impl Zero for Memo {
    const ZERO: Self = Self { title: String::new(), body: String::new() };
}

impl Input<'_> for Memo {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self { title: String::decode(input)?, body: String::decode(input)? })
    }
}

impl Output for Memo {
    fn encode(&self, output: &mut Vec<u8>) {
        self.title.encode(output);
        self.body.encode(output);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Investigator {
    pub name:                    String,
    pub pronunciation:           String,
    pub occupation:              OccupationKind,
    pub occupation_name:         String,
    pub title:                   String,
    pub birthplace:              String,
    pub pronoun:                 String,
    pub residence:               String,
    pub age:                     u16,
    pub characteristics:         [Score; 8],
    pub hit_points:              u8,
    pub magic_points:            u8,
    pub luck:                    u8,
    pub sanity:                  u8,
    pub build:                   i8,
    pub move_rate:               u8,
    pub occupation_skill_points: Option<(Characteristic, Characteristic)>,
    pub skills:                  [Points; SKILL_COUNT],
    pub specializations:         Vec<Specialization>,
    pub key_connection:          Option<Backstory>,
    pub backstory:               [String; BACKSTORY_COUNT],
    pub memos:                   Vec<Memo>,
}

impl Zero for Investigator {
    const ZERO: Self = Self {
        name:                    String::new(),
        pronunciation:           String::new(),
        occupation:              OccupationKind::Custom,
        occupation_name:         String::new(),
        title:                   String::new(),
        birthplace:              String::new(),
        pronoun:                 String::new(),
        residence:               String::new(),
        age:                     0,
        characteristics:         [Score::ZERO; 8],
        hit_points:              0,
        magic_points:            0,
        luck:                    0,
        sanity:                  0,
        build:                   0,
        move_rate:               0,
        occupation_skill_points: None,
        skills:                  [Points::ZERO; SKILL_COUNT],
        specializations:         Vec::new(),
        key_connection:          None,
        backstory:               [const { String::new() }; BACKSTORY_COUNT],
        memos:                   Vec::new(),
    };
}

impl Investigator {
    pub fn points(&self, skill: &Skill) -> Points {
        self.skills[skill.index()]
    }

    pub fn points_mut(&mut self, skill: &Skill) -> &mut Points {
        &mut self.skills[skill.index()]
    }

    pub fn known_specialization(&self, skill: &Skill) -> Option<&Specialization> {
        self.specializations.iter().find(|s| {
            s.specialty.skill() == *skill && s.specialty.known() && s.points != Points::ZERO
        })
    }

    pub fn custom_specialization(&self, skill: &Skill, index: usize) -> Option<&Specialization> {
        self.specializations
            .iter()
            .filter(|s| s.specialty.skill() == *skill && !s.specialty.known())
            .nth(index)
    }

    pub fn text(&self, entry: &Backstory) -> &str {
        entry.index().map_or("", |index| self.backstory[index].as_str())
    }
}

impl Body for Investigator {
    type Kind = Kind;
    const KIND: Kind = Kind::Investigator;
}

impl Input<'_> for Investigator {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let mut investigator = Self {
            name: String::decode(input)?,
            pronunciation: String::decode(input)?,
            occupation: OccupationKind::detect(u8::decode(input)?),
            occupation_name: String::decode(input)?,
            title: String::decode(input)?,
            birthplace: String::decode(input)?,
            pronoun: String::decode(input)?,
            residence: String::decode(input)?,
            age: u16::decode(input)?,
            ..Self::ZERO
        };
        for score in &mut investigator.characteristics {
            *score = Score::decode(input)?;
        }
        investigator.hit_points = u8::decode(input)?;
        investigator.magic_points = u8::decode(input)?;
        investigator.luck = u8::decode(input)?;
        investigator.sanity = u8::decode(input)?;
        investigator.build = i8::decode(input)?;
        investigator.move_rate = u8::decode(input)?;
        let (c1, c2) = (u8::decode(input)?, u8::decode(input)?);
        investigator.occupation_skill_points = match (c1, c2) {
            (0, 0) => None,
            (c1, c2) => Some((
                Characteristic::from_number(u32::from(c1))?,
                Characteristic::from_number(u32::from(c2))?,
            )),
        };
        for points in &mut investigator.skills {
            *points = Points::decode(input)?;
        }
        investigator.specializations = Vec::decode(input)?;
        investigator.key_connection = match u8::decode(input)? {
            0 => None,
            n => Some(Backstory::list().get(usize::from(n) - 1)?.clone()),
        };
        for text in &mut investigator.backstory {
            *text = String::decode(input)?;
        }
        investigator.memos = Vec::decode(input)?;
        Some(investigator)
    }
}

impl Output for Investigator {
    fn encode(&self, output: &mut Vec<u8>) {
        self.name.encode(output);
        self.pronunciation.encode(output);
        self.occupation.id().encode(output);
        self.occupation_name.encode(output);
        self.title.encode(output);
        self.birthplace.encode(output);
        self.pronoun.encode(output);
        self.residence.encode(output);
        self.age.encode(output);
        for score in &self.characteristics {
            score.encode(output);
        }
        self.hit_points.encode(output);
        self.magic_points.encode(output);
        self.luck.encode(output);
        self.sanity.encode(output);
        self.build.encode(output);
        self.move_rate.encode(output);
        let (c1, c2) = self
            .occupation_skill_points
            .map_or((0, 0), |(c1, c2)| (c1.number() as u8, c2.number() as u8));
        c1.encode(output);
        c2.encode(output);
        for points in &self.skills {
            points.encode(output);
        }
        self.specializations.encode(output);
        (self.key_connection.as_ref().and_then(Backstory::index).map_or(0, |i| i + 1) as u8)
            .encode(output);
        for text in &self.backstory {
            text.encode(output);
        }
        self.memos.encode(output);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtAndCraft {
    Acting = 1,
    Barber,
    Calligraphy,
    Carpentry,
    Cook,
    Dancing,
    FineArt,
    Forgery,
    Photography,
    Pottery,
    Sculpting,
    Writing,
    Custom,
}

impl ArtAndCraft {
    pub const fn number(&self) -> u32 {
        *self as u32
    }

    pub fn display(&self, language: Language) -> &str {
        match (self, language) {
            (Self::Acting, Language::En(_)) => "Acting",
            (Self::Acting, Language::Ja) => "演劇",
            (Self::Barber, Language::En(_)) => "Barber",
            (Self::Barber, Language::Ja) => "理容",
            (Self::Calligraphy, Language::En(_)) => "Calligraphy",
            (Self::Calligraphy, Language::Ja) => "書道",
            (Self::Carpentry, Language::En(_)) => "Carpentry",
            (Self::Carpentry, Language::Ja) => "大工仕事",
            (Self::Cook, Language::En(_)) => "Cook",
            (Self::Cook, Language::Ja) => "料理",
            (Self::Dancing, Language::En(_)) => "Dancing",
            (Self::Dancing, Language::Ja) => "ダンス",
            (Self::FineArt, Language::En(_)) => "Fine Art",
            (Self::FineArt, Language::Ja) => "絵画",
            (Self::Forgery, Language::En(_)) => "Forgery",
            (Self::Forgery, Language::Ja) => "文書偽造",
            (Self::Photography, Language::En(_)) => "Photography",
            (Self::Photography, Language::Ja) => "写真術",
            (Self::Pottery, Language::En(_)) => "Pottery",
            (Self::Pottery, Language::Ja) => "陶芸",
            (Self::Sculpting, Language::En(_)) => "Sculpting",
            (Self::Sculpting, Language::Ja) => "彫刻",
            (Self::Writing, Language::En(_)) => "Writing",
            (Self::Writing, Language::Ja) => "執筆",
            (Self::Custom, _) => {
                unreachable!("ArtAndCraft::Customはリスト格納スロットでありname() 不可")
            }
        }
    }

    pub fn list() -> &'static [Self] {
        &[
            Self::Acting,
            Self::Barber,
            Self::Calligraphy,
            Self::Carpentry,
            Self::Cook,
            Self::Dancing,
            Self::FineArt,
            Self::Forgery,
            Self::Photography,
            Self::Pottery,
            Self::Sculpting,
            Self::Writing,
            Self::Custom,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fighting {
    Axe = 1,
    Brawl,
    Chainsaw,
    Flail,
    Garrote,
    Spear,
    Sword,
    Whip,
    Custom,
}

impl Fighting {
    pub fn list() -> &'static [Self] {
        &[
            Self::Axe,
            Self::Brawl,
            Self::Chainsaw,
            Self::Flail,
            Self::Garrote,
            Self::Spear,
            Self::Sword,
            Self::Whip,
        ]
    }

    pub const fn number(&self) -> u32 {
        *self as u32
    }

    pub const fn base_percent(&self) -> u16 {
        match self {
            Self::Axe => 15,
            Self::Brawl => 25,
            Self::Chainsaw => 10,
            Self::Flail => 10,
            Self::Garrote => 15,
            Self::Spear => 20,
            Self::Sword => 20,
            Self::Whip => 5,
            Self::Custom => unreachable!(),
        }
    }

    pub fn display(&self, language: Language) -> &str {
        match (self, language) {
            (Self::Axe, Language::En(_)) => "Axe",
            (Self::Axe, Language::Ja) => "斧",
            (Self::Brawl, Language::En(_)) => "Brawl",
            (Self::Brawl, Language::Ja) => "格闘",
            (Self::Chainsaw, Language::En(_)) => "Chainsaw",
            (Self::Chainsaw, Language::Ja) => "チェーンソー",
            (Self::Flail, Language::En(_)) => "Flail",
            (Self::Flail, Language::Ja) => "フレイル",
            (Self::Garrote, Language::En(_)) => "Garrote",
            (Self::Garrote, Language::Ja) => "絞殺ひも",
            (Self::Spear, Language::En(_)) => "Spear",
            (Self::Spear, Language::Ja) => "槍",
            (Self::Sword, Language::En(_)) => "Sword",
            (Self::Sword, Language::Ja) => "刀剣",
            (Self::Whip, Language::En(_)) => "Whip",
            (Self::Whip, Language::Ja) => "鞭",
            (Self::Custom, _) => {
                unreachable!("Fighting::Customはリスト格納スロットでありname() 不可")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Firearms {
    Bow = 1,
    FlameThrower,
    Handgun,
    HeavyWeapons,
    MachineGun,
    RifleShotgun,
    SubmachineGun,
    Custom,
}

impl Firearms {
    pub fn list() -> &'static [Self] {
        &[
            Self::Bow,
            Self::FlameThrower,
            Self::Handgun,
            Self::HeavyWeapons,
            Self::MachineGun,
            Self::RifleShotgun,
            Self::SubmachineGun,
        ]
    }

    pub const fn number(&self) -> u32 {
        *self as u32
    }

    pub const fn base_percent(&self) -> u16 {
        match self {
            Self::Bow => 15,
            Self::FlameThrower => 10,
            Self::Handgun => 20,
            Self::HeavyWeapons => 10,
            Self::MachineGun => 10,
            Self::RifleShotgun => 25,
            Self::SubmachineGun => 15,
            Self::Custom => unreachable!(),
        }
    }

    pub fn display(&self, language: Language) -> &str {
        match (self, language) {
            (Self::Bow, Language::En(_)) => "Bow",
            (Self::Bow, Language::Ja) => "弓",
            (Self::FlameThrower, Language::En(_)) => "Flamethrower",
            (Self::FlameThrower, Language::Ja) => "火炎放射器",
            (Self::Handgun, Language::En(_)) => "Handgun",
            (Self::Handgun, Language::Ja) => "拳銃",
            (Self::HeavyWeapons, Language::En(_)) => "Heavy Weapons",
            (Self::HeavyWeapons, Language::Ja) => "重火器",
            (Self::MachineGun, Language::En(_)) => "Machine Gun",
            (Self::MachineGun, Language::Ja) => "マシンガン",
            (Self::RifleShotgun, Language::En(_)) => "Rifle/Shotgun",
            (Self::RifleShotgun, Language::Ja) => "ライフル/ショットガン",
            (Self::SubmachineGun, Language::En(_)) => "Submachine Gun",
            (Self::SubmachineGun, Language::Ja) => "サブマシンガン",
            (Self::Custom, _) => {
                unreachable!("Firearms::Customはリスト格納スロットでありname() 不可")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pilot {
    Boat = 1,
    SteamShip,
    Sailboat,
    CivilProp,
    Balloon,
    Dirigible,
    CivilJet,
    Airliner,
    JetFighter,
    Helicopter,
    Custom,
}

impl Pilot {
    pub fn list() -> &'static [Self] {
        &[
            Self::Boat,
            Self::SteamShip,
            Self::Sailboat,
            Self::CivilProp,
            Self::Balloon,
            Self::Dirigible,
            Self::CivilJet,
            Self::Airliner,
            Self::JetFighter,
            Self::Helicopter,
        ]
    }

    pub const fn number(&self) -> u32 {
        *self as u32
    }

    pub const fn base_percent(&self) -> u16 {
        1
    }

    pub const fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Boat, Language::Ja) => "ボート",
            (Self::Boat, Language::En(_)) => "Boat",
            (Self::SteamShip, Language::Ja) => "汽船",
            (Self::SteamShip, Language::En(_)) => "Steam Ship",
            (Self::Sailboat, Language::Ja) => "帆船",
            (Self::Sailboat, Language::En(_)) => "Sailboat",
            (Self::CivilProp, Language::Ja) => "民間プロペラ機",
            (Self::CivilProp, Language::En(_)) => "Civil Prop",
            (Self::Balloon, Language::Ja) => "気球",
            (Self::Balloon, Language::En(_)) => "Balloon",
            (Self::Dirigible, Language::Ja) => "飛行船",
            (Self::Dirigible, Language::En(_)) => "Dirigible",
            (Self::CivilJet, Language::Ja) => "民間ジェット機",
            (Self::CivilJet, Language::En(_)) => "Civil Jet",
            (Self::Airliner, Language::Ja) => "旅客機",
            (Self::Airliner, Language::En(_)) => "Airliner",
            (Self::JetFighter, Language::Ja) => "ジェット戦闘機",
            (Self::JetFighter, Language::En(_)) => "Jet Fighter",
            (Self::Helicopter, Language::Ja) => "ヘリコプター",
            (Self::Helicopter, Language::En(_)) => "Helicopter",
            (Self::Custom, _) => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Survival {
    Arctic = 1,
    Desert,
    Sea,
    Custom,
}

impl Survival {
    pub fn list() -> &'static [Self] {
        &[Self::Arctic, Self::Desert, Self::Sea]
    }

    pub const fn number(&self) -> u32 {
        *self as u32
    }

    pub const fn base_percent(&self) -> u16 {
        10
    }

    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Arctic, Language::Ja) => "極地",
            (Self::Arctic, Language::En(_)) => "Arctic",
            (Self::Desert, Language::Ja) => "砂漠",
            (Self::Desert, Language::En(_)) => "Desert",
            (Self::Sea, Language::Ja) => "海",
            (Self::Sea, Language::En(_)) => "Sea",
            (Self::Custom, _) => {
                unreachable!("Survival::Customはリスト格納スロットでありname() 不可")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Science {
    None,
    Astronomy,
    Biology,
    Botany,
    Chemistry,
    Cryptography,
    Engineering,
    Forensics,
    Geology,
    Mathematics,
    Meteorology,
    Pharmacy,
    Physics,
    Zoology,
    Custom,
}

impl Science {
    pub fn list() -> &'static [Self] {
        &[
            Self::Astronomy,
            Self::Biology,
            Self::Botany,
            Self::Chemistry,
            Self::Cryptography,
            Self::Engineering,
            Self::Forensics,
            Self::Geology,
            Self::Mathematics,
            Self::Meteorology,
            Self::Pharmacy,
            Self::Physics,
            Self::Zoology,
        ]
    }

    pub const fn number(&self) -> u32 {
        *self as u32
    }

    pub const fn base_percent(&self) -> u16 {
        match self {
            Self::Mathematics => 10,
            _ => 1,
        }
    }

    pub fn display(&self, language: Language) -> Option<&'static str> {
        match (self, language) {
            (Self::None, _) => None,
            (Self::Astronomy, Language::Ja) => Some("天文学"),
            (Self::Astronomy, Language::En(_)) => Some("Astronomy"),
            (Self::Biology, Language::Ja) => Some("生物学"),
            (Self::Biology, Language::En(_)) => Some("Biology"),
            (Self::Botany, Language::Ja) => Some("植物学"),
            (Self::Botany, Language::En(_)) => Some("Botany"),
            (Self::Chemistry, Language::Ja) => Some("化学"),
            (Self::Chemistry, Language::En(_)) => Some("Chemistry"),
            (Self::Cryptography, Language::Ja) => Some("暗号学"),
            (Self::Cryptography, Language::En(_)) => Some("Cryptography"),
            (Self::Engineering, Language::Ja) => Some("工学"),
            (Self::Engineering, Language::En(_)) => Some("Engineering"),
            (Self::Forensics, Language::Ja) => Some("法医学"),
            (Self::Forensics, Language::En(_)) => Some("Forensics"),
            (Self::Geology, Language::Ja) => Some("地質学"),
            (Self::Geology, Language::En(_)) => Some("Geology"),
            (Self::Mathematics, Language::Ja) => Some("数学"),
            (Self::Mathematics, Language::En(_)) => Some("Mathematics"),
            (Self::Meteorology, Language::Ja) => Some("気象学"),
            (Self::Meteorology, Language::En(_)) => Some("Meteorology"),
            (Self::Pharmacy, Language::Ja) => Some("薬学"),
            (Self::Pharmacy, Language::En(_)) => Some("Pharmacy"),
            (Self::Physics, Language::Ja) => Some("物理学"),
            (Self::Physics, Language::En(_)) => Some("Physics"),
            (Self::Zoology, Language::Ja) => Some("動物学"),
            (Self::Zoology, Language::En(_)) => Some("Zoology"),
            (Self::Custom, _) => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Possession {
    Weapon = 1,
    Armor,
    Equipment,
    Wealth,
}

impl Possession {
    pub fn number(&self) -> u32 {
        Section::Possession.number() + *self as u32
    }

    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Weapon, Language::En(_)) => "Weapon",
            (Self::Weapon, Language::Ja) => "武器",
            (Self::Armor, Language::En(_)) => "Armor",
            (Self::Armor, Language::Ja) => "装甲",
            (Self::Equipment, Language::En(_)) => "Equipment",
            (Self::Equipment, Language::Ja) => "所持品",
            (Self::Wealth, Language::En(_)) => "Wealth",
            (Self::Wealth, Language::Ja) => "収入と財産",
        }
    }
}

pub enum Weapon {
    BowAndArrows,
    BrassKnuckles, // Brass Knuckles      1D3+1+DB
    Bullwhip,      // Bullwhip            1D3+half DB
    BurningTorch,  // Burning Torch       1D6+burn
    Blackjack,     // Blackjack           1D8+DB
    ClubLarge,     // Club, Large         1D8+DB
    ClubSmall,     // Club, Small         1D6+DB
    Crossbow,
    Garrote,
    HatchetSickle,
    KnifeLarge,
    KnifeMedium,
    KnifeSmall,
    Nunchaku,   // Nunchaku            1D8+DB
    RockThrown, // Rock, Thrown        1D4+half DB
    Shuriken,
    Spear,
    SpearThrown,
    Auto22Short,            // .22 Short Automatic 1D6
    Derringer25,            // .25 Derringer       1D6
    Revolver32,             // .32 Revolver        1D8
    Automatic32,            // .32 Automatic       1D8
    LugerP08,               // Model P08 Luger     1D10
    Revolver45,             // .45 Revolver        1D10+2
    Automatic45,            // .45 Automatic       1D10+2
    BoltAction22,           // .22 Bolt-Action     1D6+1
    LeverAction30,          // .30 Lever-Action    2D6
    MartiniHenry45,         // .45 Martini-Henry   1D8+1D6+3
    MoranAirRifle,          // Col. Moran's Air    2D6+1
    LeeEnfield303,          // .303 Lee-Enfield    2D6+4
    BoltAction3006,         // .30-06 Bolt-Action  2D6+4
    ElephantGun,            // Elephant Gun        3D6+4
    Shotgun20Gauge,         // 20-gauge (2B)        2D6/1D6/1D3
    Shotgun16Gauge,         // 16-gauge (2B)        2D6+2/1D6+1/1D4
    Shotgun12Gauge,         // 12-gauge (2B)        4D6/2D6/1D6
    Shotgun12GaugeSemiAuto, // 12-gauge semi-auto   4D6/2D6/1D6
    Shotgun12GaugeSawedOff, // 12-gauge sawed off   4D6/1D6
    BergmannMP18,           // Bergmann MP18        1D10
    Thompson,               // Thompson             1D10+2
    BrowningAutoRifle,      // Browning Auto Rifle  2D6+4
    BrowningM1917,          // .30 Browning M1917   2D6+4
    BrenGun,                // Bren Gun             2D6+4
    LewisGun,               // Mark I Lewis Gun     2D6+4
    Vickers303,             // Vickers .303         2D6+4
    Custom(String),
}

impl Weapon {
    pub fn display(&self, language: Language) -> &str {
        match (self, language) {
            (Self::BowAndArrows, Language::En(_)) => "Bow and Arrows",
            (Self::BowAndArrows, Language::Ja) => "弓と矢",
            (Self::BrassKnuckles, Language::En(_)) => "Brass Knuckles",
            (Self::BrassKnuckles, Language::Ja) => "ブラスナックル",
            (Self::Bullwhip, Language::En(_)) => "Bullwhip",
            (Self::Bullwhip, Language::Ja) => "むち",
            (Self::BurningTorch, Language::En(_)) => "Burning Torch",
            (Self::BurningTorch, Language::Ja) => "燃えているたいまつ",
            (Self::Blackjack, Language::En(_)) => "Blackjack",
            (Self::Blackjack, Language::Ja) => "ブラックジャック",
            (Self::ClubLarge, Language::En(_)) => "Club, Large",
            (Self::ClubLarge, Language::Ja) => "大きい棍棒",
            (Self::ClubSmall, Language::En(_)) => "Club, Small",
            (Self::ClubSmall, Language::Ja) => "小さい棍棒",
            (Self::Crossbow, Language::En(_)) => "Crossbow",
            (Self::Crossbow, Language::Ja) => "クロスボウ",
            (Self::Garrote, Language::En(_)) => "Garrote",
            (Self::Garrote, Language::Ja) => "絞殺ひも",
            (Self::HatchetSickle, Language::En(_)) => "Hatchet/Sickle",
            (Self::HatchetSickle, Language::Ja) => "手斧/小鎌",
            (Self::KnifeLarge, Language::En(_)) => "Knife, Large",
            (Self::KnifeLarge, Language::Ja) => "大型ナイフ",
            (Self::KnifeMedium, Language::En(_)) => "Knife, Medium",
            (Self::KnifeMedium, Language::Ja) => "中型ナイフ",
            (Self::KnifeSmall, Language::En(_)) => "Knife, Small",
            (Self::KnifeSmall, Language::Ja) => "小型ナイフ",
            (Self::Nunchaku, Language::En(_)) => "Nunchaku",
            (Self::Nunchaku, Language::Ja) => "ヌンチャク",
            (Self::RockThrown, Language::En(_)) => "Rock, Thrown",
            (Self::RockThrown, Language::Ja) => "投石",
            (Self::Shuriken, Language::En(_)) => "Shuriken",
            (Self::Shuriken, Language::Ja) => "手裏剣",
            (Self::Spear, Language::En(_)) => "Spear",
            (Self::Spear, Language::Ja) => "騎兵槍",
            (Self::SpearThrown, Language::En(_)) => "Spear, Thrown",
            (Self::SpearThrown, Language::Ja) => "投げ槍",
            (Self::Auto22Short, Language::En(_)) => ".22 Short Automatic",
            (Self::Auto22Short, Language::Ja) => ".22ショートオートマチック",
            (Self::Derringer25, Language::En(_)) => ".25 Derringer",
            (Self::Derringer25, Language::Ja) => ".25デリンジャー",
            (Self::Revolver32, Language::En(_)) => ".32 Revolver",
            (Self::Revolver32, Language::Ja) => ".32リボルバー",
            (Self::Automatic32, Language::En(_)) => ".32 Automatic",
            (Self::Automatic32, Language::Ja) => ".32オートマチック",
            (Self::LugerP08, Language::En(_)) => "Model P08 Luger",
            (Self::LugerP08, Language::Ja) => "P08ルガー",
            (Self::Revolver45, Language::En(_)) => ".45 Revolver",
            (Self::Revolver45, Language::Ja) => ".45リボルバー",
            (Self::Automatic45, Language::En(_)) => ".45 Automatic",
            (Self::Automatic45, Language::Ja) => ".45オートマチック",
            (Self::BoltAction22, Language::En(_)) => ".22 Bolt-Action Rifle",
            (Self::BoltAction22, Language::Ja) => ".22ボルトアクションライフル",
            (Self::LeverAction30, Language::En(_)) => ".30 Lever-Action Carbine",
            (Self::LeverAction30, Language::Ja) => ".30レバーアクションカービン",
            (Self::MartiniHenry45, Language::En(_)) => ".45 Martini-Henry Rifle",
            (Self::MartiniHenry45, Language::Ja) => ".45マルティニ・ヘンリー",
            (Self::MoranAirRifle, Language::En(_)) => "Col. Moran's Air Rifle",
            (Self::MoranAirRifle, Language::Ja) => "モラン大佐の空気銃",
            (Self::LeeEnfield303, Language::En(_)) => ".303 Lee-Enfield",
            (Self::LeeEnfield303, Language::Ja) => ".303リー・エンフィールド",
            (Self::BoltAction3006, Language::En(_)) => ".30-06 Bolt-Action Rifle",
            (Self::BoltAction3006, Language::Ja) => ".30-06ボルトアクションライフル",
            (Self::ElephantGun, Language::En(_)) => "Elephant Gun",
            (Self::ElephantGun, Language::Ja) => "エレファントガン",
            (Self::Shotgun20Gauge, Language::En(_)) => "20-gauge Shotgun",
            (Self::Shotgun20Gauge, Language::Ja) => "20ゲージショットガン",
            (Self::Shotgun16Gauge, Language::En(_)) => "16-gauge Shotgun",
            (Self::Shotgun16Gauge, Language::Ja) => "16ゲージショットガン",
            (Self::Shotgun12Gauge, Language::En(_)) => "12-gauge Shotgun",
            (Self::Shotgun12Gauge, Language::Ja) => "12ゲージショットガン",
            (Self::Shotgun12GaugeSemiAuto, Language::En(_)) => "12-gauge Shotgun (semi-auto)",
            (Self::Shotgun12GaugeSemiAuto, Language::Ja) => "12ゲージショットガン(半自動)",
            (Self::Shotgun12GaugeSawedOff, Language::En(_)) => "12-gauge Shotgun (sawed off)",
            (Self::Shotgun12GaugeSawedOff, Language::Ja) => "12ゲージショットガン(短銃身)",
            (Self::BergmannMP18, Language::En(_)) => "Bergmann MP18",
            (Self::BergmannMP18, Language::Ja) => "ベルグマンMP18",
            (Self::Thompson, Language::En(_)) => "Thompson",
            (Self::Thompson, Language::Ja) => "トンプソン",
            (Self::BrowningAutoRifle, Language::En(_)) => "Browning Automatic Rifle M1918",
            (Self::BrowningAutoRifle, Language::Ja) => "ブローニング自動小銃M1918",
            (Self::BrowningM1917, Language::En(_)) => ".30 Browning M1917A1",
            (Self::BrowningM1917, Language::Ja) => ".30ブローニングM1917A1",
            (Self::BrenGun, Language::En(_)) => "Bren Gun",
            (Self::BrenGun, Language::Ja) => "ブレンガン",
            (Self::LewisGun, Language::En(_)) => "Mark I Lewis Gun",
            (Self::LewisGun, Language::Ja) => "ルイス軽機関銃Mk.I",
            (Self::Vickers303, Language::En(_)) => "Vickers .303 Machine Gun",
            (Self::Vickers303, Language::Ja) => "ヴィッカース.303機関銃",
            (Self::Custom(_), _) => "Custom",
        }
    }

    pub fn skill(&self) -> Skill {
        todo!()
    }

    pub fn range(&self, _lang: Language) -> (u8, &str) {
        // integer, unit
        todo!()
    }

    pub fn damage(&self) -> Option<(&'static [Dice], u8)> {
        match self {
            Self::BowAndArrows => Some((&[(1, 6, 0)], 2)),
            Self::BrassKnuckles => Some((&[(1, 3, 1)], 1)),
            Self::Bullwhip => Some((&[(1, 3, 0)], 2)),
            Self::BurningTorch => Some((&[(1, 6, 0)], 0)),
            Self::Blackjack => Some((&[(1, 8, 0)], 1)),
            Self::ClubLarge => Some((&[(1, 8, 0)], 1)),
            Self::ClubSmall => Some((&[(1, 6, 0)], 1)),
            Self::Crossbow => Some((&[(1, 8, 2)], 0)),
            Self::Garrote => Some((&[(1, 6, 0)], 1)),
            Self::HatchetSickle => Some((&[(1, 6, 1)], 1)),
            Self::KnifeLarge => Some((&[(1, 8, 0)], 1)),
            Self::KnifeMedium => Some((&[(1, 4, 2)], 1)),
            Self::KnifeSmall => Some((&[(1, 4, 0)], 1)),
            Self::Nunchaku => Some((&[(1, 8, 0)], 1)),
            Self::RockThrown => Some((&[(1, 4, 0)], 2)),
            Self::Shuriken => Some((&[(1, 3, 0)], 2)),
            Self::Spear => Some((&[(1, 8, 1)], 0)),
            Self::SpearThrown => Some((&[(1, 8, 0)], 2)),
            Self::Auto22Short => Some((&[(1, 6, 0)], 0)),
            Self::Derringer25 => Some((&[(1, 6, 0)], 0)),
            Self::Revolver32 => Some((&[(1, 8, 0)], 0)),
            Self::Automatic32 => Some((&[(1, 8, 0)], 0)),
            Self::LugerP08 => Some((&[(1, 10, 0)], 0)),
            Self::Revolver45 => Some((&[(1, 10, 2)], 0)),
            Self::Automatic45 => Some((&[(1, 10, 2)], 0)),
            Self::BoltAction22 => Some((&[(1, 6, 1)], 0)),
            Self::LeverAction30 => Some((&[(2, 6, 0)], 0)),
            Self::MartiniHenry45 => Some((&[(1, 8, 0), (1, 6, 3)], 0)),
            Self::MoranAirRifle => Some((&[(2, 6, 1)], 0)),
            Self::LeeEnfield303 => Some((&[(2, 6, 4)], 0)),
            Self::BoltAction3006 => Some((&[(2, 6, 4)], 0)),
            Self::ElephantGun => Some((&[(3, 6, 4)], 0)),
            Self::Shotgun20Gauge => Some((&[(2, 6, 0)], 0)),
            Self::Shotgun16Gauge => Some((&[(2, 6, 2)], 0)),
            Self::Shotgun12Gauge => Some((&[(4, 6, 0)], 0)),
            Self::Shotgun12GaugeSemiAuto => Some((&[(4, 6, 0)], 0)),
            Self::Shotgun12GaugeSawedOff => Some((&[(4, 6, 0)], 0)),
            Self::BergmannMP18 => Some((&[(1, 10, 0)], 0)),
            Self::Thompson => Some((&[(1, 10, 2)], 0)),
            Self::BrowningAutoRifle => Some((&[(2, 6, 4)], 0)),
            Self::BrowningM1917 => Some((&[(2, 6, 4)], 0)),
            Self::BrenGun => Some((&[(2, 6, 4)], 0)),
            Self::LewisGun => Some((&[(2, 6, 4)], 0)),
            Self::Vickers303 => Some((&[(2, 6, 4)], 0)),
            Self::Custom(_) => None,
        }
    }

    pub fn is_impalable(&self) -> bool {
        matches!(
            self,
            Self::BowAndArrows
                | Self::Crossbow
                | Self::Garrote
                | Self::HatchetSickle
                | Self::KnifeLarge
                | Self::KnifeMedium
                | Self::KnifeSmall
                | Self::Shuriken
                | Self::Spear
                | Self::SpearThrown
                | Self::Auto22Short
                | Self::Derringer25
                | Self::Revolver32
                | Self::Automatic32
                | Self::LugerP08
                | Self::Revolver45
                | Self::Automatic45
                | Self::BoltAction22
                | Self::LeverAction30
                | Self::MartiniHenry45
                | Self::MoranAirRifle
                | Self::LeeEnfield303
                | Self::BoltAction3006
                | Self::ElephantGun
                | Self::BergmannMP18
                | Self::Thompson
                | Self::BrowningAutoRifle
                | Self::BrowningM1917
                | Self::BrenGun
                | Self::LewisGun
                | Self::Vickers303
        )
    }

    pub fn attacks_per_round(&self) -> u8 {
        match self {
            Self::BowAndArrows => 1,
            Self::BrassKnuckles => 1,
            Self::Bullwhip => 1,
            Self::BurningTorch => 1,
            Self::Blackjack => 1,
            Self::ClubLarge => 1,
            Self::ClubSmall => 1,
            Self::Crossbow => 1,
            Self::Garrote => 1,
            Self::HatchetSickle => 1,
            Self::KnifeLarge => 1,
            Self::KnifeMedium => 1,
            Self::KnifeSmall => 1,
            Self::Nunchaku => 1,
            Self::RockThrown => 1,
            Self::Shuriken => 2,
            Self::Spear => 1,
            Self::SpearThrown => 1,
            Self::Auto22Short => 1,
            Self::Derringer25 => 1,
            Self::Revolver32 => 1,
            Self::Automatic32 => 1,
            Self::LugerP08 => 1,
            Self::Revolver45 => 1,
            Self::Automatic45 => 1,
            Self::BoltAction22 => 1,
            Self::LeverAction30 => 1,
            Self::MartiniHenry45 => 1,
            Self::MoranAirRifle => 1,
            Self::LeeEnfield303 => 1,
            Self::BoltAction3006 => 1,
            Self::ElephantGun => 1,
            Self::Shotgun20Gauge => 1,
            Self::Shotgun16Gauge => 1,
            Self::Shotgun12Gauge => 1,
            Self::Shotgun12GaugeSemiAuto => 1,
            Self::Shotgun12GaugeSawedOff => 1,
            Self::BergmannMP18 => 1,
            Self::Thompson => 1,
            Self::BrowningAutoRifle => 1,
            Self::BrowningM1917 => 1,
            Self::BrenGun => 1,
            Self::LewisGun => 1,
            Self::Vickers303 => 1,
            Self::Custom(_) => 1,
        }
    }

    pub fn ammunition(&self) -> Option<u8> {
        match self {
            Self::BowAndArrows => Some(1),
            Self::Crossbow => Some(1),
            Self::Shuriken => Some(1), // one use
            Self::Auto22Short => Some(6),
            Self::Derringer25 => Some(1),
            Self::Revolver32 => Some(6),
            Self::Automatic32 => Some(8),
            Self::LugerP08 => Some(8),
            Self::Revolver45 => Some(6),
            Self::Automatic45 => Some(7),
            Self::BoltAction22 => Some(6),
            Self::LeverAction30 => Some(6),
            Self::MartiniHenry45 => Some(1),
            Self::MoranAirRifle => Some(1),
            Self::LeeEnfield303 => Some(10),
            Self::BoltAction3006 => Some(5),
            Self::ElephantGun => Some(2),
            Self::Shotgun20Gauge => Some(2),
            Self::Shotgun16Gauge => Some(2),
            Self::Shotgun12Gauge => Some(2),
            Self::Shotgun12GaugeSemiAuto => Some(5),
            Self::Shotgun12GaugeSawedOff => Some(2),
            Self::BergmannMP18 => Some(32), // 20/30/32
            Self::Thompson => Some(30),     // 20/30/50
            Self::BrowningAutoRifle => Some(20),
            Self::BrowningM1917 => Some(250),
            Self::BrenGun => Some(30),  // 30/100
            Self::LewisGun => Some(47), // 47/97
            Self::Vickers303 => Some(250),
            _ => None,
        }
    }

    pub fn malfunction(&self) -> Option<u8> {
        match self {
            Self::BowAndArrows => Some(97),
            Self::Crossbow => Some(96),
            Self::Shuriken => Some(100),
            Self::Auto22Short => Some(100),
            Self::Derringer25 => Some(100),
            Self::Revolver32 => Some(100),
            Self::Automatic32 => Some(99),
            Self::LugerP08 => Some(99),
            Self::Revolver45 => Some(100),
            Self::Automatic45 => Some(100),
            Self::BoltAction22 => Some(99),
            Self::LeverAction30 => Some(98),
            Self::MartiniHenry45 => Some(100),
            Self::MoranAirRifle => Some(88),
            Self::LeeEnfield303 => Some(100),
            Self::BoltAction3006 => Some(100),
            Self::ElephantGun => Some(100),
            Self::Shotgun20Gauge => Some(100),
            Self::Shotgun16Gauge => Some(100),
            Self::Shotgun12Gauge => Some(100),
            Self::Shotgun12GaugeSemiAuto => Some(100),
            Self::Shotgun12GaugeSawedOff => Some(100),
            Self::BergmannMP18 => Some(96),
            Self::Thompson => Some(96),
            Self::BrowningAutoRifle => Some(100),
            Self::BrowningM1917 => Some(96),
            Self::BrenGun => Some(96),
            Self::LewisGun => Some(96),
            Self::Vickers303 => None, // N/A
            _ => None,
        }
    }
}

// p.108
pub enum Armor {
    ThickLeatherJacket, // Thick Leather Jacket    1pt
    WwiHelmet,          // WWI Helmet              2pt
    Hardwood1In,        // 1" Hardwood             3pt
    PresentUsHelmet,    // Present U.S. Helmet     5pt
    HeavyKevlarVest,    // Heavy Kevlar Vest       8pt
    MilitaryBodyArmor,  // Military Body Armor    12pt
    BulletproofGlass,   // 1.5" Bulletproof Glass 15pt
    SteelPlate1In,      // 1" Steel Plate         19pt
    LargeSandbag,       // Large Sandbag          20pt
    Custom(String),
}

impl Armor {
    pub fn display(&self, language: Language) -> &str {
        match (self, language) {
            (Self::ThickLeatherJacket, Language::En(_)) => "Thick Leather Jacket",
            (Self::ThickLeatherJacket, Language::Ja) => "厚い皮のジャケット",
            (Self::WwiHelmet, Language::En(_)) => "WWI Helmet",
            (Self::WwiHelmet, Language::Ja) => "第一次大戦型のヘルメット",
            (Self::Hardwood1In, Language::En(_)) => "1\" Hardwood",
            (Self::Hardwood1In, Language::Ja) => "3cmの堅い木",
            (Self::PresentUsHelmet, Language::En(_)) => "Present U.S. Helmet",
            (Self::PresentUsHelmet, Language::Ja) => "現代アメリカ軍のヘルメット",
            (Self::HeavyKevlarVest, Language::En(_)) => "Heavy Kevlar Vest",
            (Self::HeavyKevlarVest, Language::Ja) => "厚いケブラー製のベスト",
            (Self::MilitaryBodyArmor, Language::En(_)) => "Military Body Armor",
            (Self::MilitaryBodyArmor, Language::Ja) => "軍用ボディ・アーマー",
            (Self::BulletproofGlass, Language::En(_)) => "1.5\" Bulletproof Glass",
            (Self::BulletproofGlass, Language::Ja) => "4cmの防弾ガラス",
            (Self::SteelPlate1In, Language::En(_)) => "1\" Steel Plate",
            (Self::SteelPlate1In, Language::Ja) => "5cmの鋼鉄板",
            (Self::LargeSandbag, Language::En(_)) => "Large Sandbag",
            (Self::LargeSandbag, Language::Ja) => "大きなサンドバッグ",
            (Self::Custom(s), _) => s.as_str(),
        }
    }

    pub fn points(&self) -> Option<u8> {
        match self {
            Self::ThickLeatherJacket => Some(1),
            Self::WwiHelmet => Some(2),
            Self::Hardwood1In => Some(3),
            Self::PresentUsHelmet => Some(5),
            Self::HeavyKevlarVest => Some(8),
            Self::MilitaryBodyArmor => Some(12),
            Self::BulletproofGlass => Some(15),
            Self::SteelPlate1In => Some(19),
            Self::LargeSandbag => Some(20),
            Self::Custom(_) => None,
        }
    }
}

pub enum StandardOfLiving {
    Pauper,
    Poor,
    Average,
    Wealthy,
    Rich,
}

pub struct Wealth {
    pub spending_level: StandardOfLiving,
    pub cash:           String, // e.g. "$20"
    pub assets:         String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WealthField {
    SpendingLevel,
    Cash,
    Assets,
}

impl WealthField {
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::SpendingLevel, Language::En(_)) => "Spending Level",
            (Self::SpendingLevel, Language::Ja) => "支出レベル",
            (Self::Cash, Language::En(_)) => "Cash",
            (Self::Cash, Language::Ja) => "現金",
            (Self::Assets, Language::En(_)) => "Assets",
            (Self::Assets, Language::Ja) => "資産",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Backstory {
    KeyConnection(Box<Backstory>),
    PersonalDescription,
    IdeologyAndBeliefs,
    SignificantPeople,
    MeaningfulLocation,
    TreasuredPossession,
    Trait,
    InjuresAndScars,
    PhobiasAndManias,
    ArcaneTomesAndSpells,
    EncountersWithStrangeEntities,
}

impl Backstory {
    pub fn number(&self) -> u32 {
        Section::Backstory.number()
            + match self {
                Self::KeyConnection(_) => 0,
                Self::PersonalDescription => 1,
                Self::IdeologyAndBeliefs => 2,
                Self::SignificantPeople => 3,
                Self::MeaningfulLocation => 4,
                Self::TreasuredPossession => 5,
                Self::Trait => 6,
                Self::InjuresAndScars => 7,
                // todo
                Self::PhobiasAndManias => 8,
                Self::ArcaneTomesAndSpells => 9,
                Self::EncountersWithStrangeEntities => 10,
            }
    }

    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::KeyConnection(_), Language::En(_)) => "Key Connection",
            (Self::KeyConnection(_), Language::Ja) => "キーコネクション",
            (Self::PersonalDescription, Language::En(_)) => "Personal Description",
            (Self::PersonalDescription, Language::Ja) => "容姿の描写",
            (Self::IdeologyAndBeliefs, Language::En(_)) => "Ideology & Beliefs",
            (Self::IdeologyAndBeliefs, Language::Ja) => "イデオロギー・信念",
            (Self::SignificantPeople, Language::En(_)) => "Significant People",
            (Self::SignificantPeople, Language::Ja) => "重要な人物",
            (Self::MeaningfulLocation, Language::En(_)) => "Meaningful Location",
            (Self::MeaningfulLocation, Language::Ja) => "意味のある場所",
            (Self::TreasuredPossession, Language::En(_)) => "Treasured Possession",
            (Self::TreasuredPossession, Language::Ja) => "秘蔵の品",
            (Self::Trait, Language::En(_)) => "Trait",
            (Self::Trait, Language::Ja) => "特徴",
            (Self::InjuresAndScars, Language::En(_)) => "Injuries & Scars",
            (Self::InjuresAndScars, Language::Ja) => "負傷、傷跡",
            (Self::PhobiasAndManias, Language::En(_)) => "Phobias & Manias",
            (Self::PhobiasAndManias, Language::Ja) => "恐怖症とマニア",
            (Self::ArcaneTomesAndSpells, Language::En(_)) => "Arcane Tomes & Spells",
            (Self::ArcaneTomesAndSpells, Language::Ja) => "魔道書、呪文、アーティファクト",
            (Self::EncountersWithStrangeEntities, Language::En(_)) => {
                "Encounters with Strange Entities"
            }
            (Self::EncountersWithStrangeEntities, Language::Ja) => "遭遇した超自然の存在",
        }
    }
}

const MANAGEMENT_SECTION: u32 = 1;
const SHEET_SECTION: u32 = 1;
pub const WEAPON_ROWS: u32 = 4;
pub const WEAPON_COLUMNS: u32 = 8;
pub const ARMOR_ROWS: u32 = 2;
pub const ARMOR_COLUMNS: u32 = 4;
pub const EQUIPMENT_ROWS: u32 = 12;
pub const EQUIPMENT_DETAILS: u32 = 3;
const EQUIPMENT_LIST: u32 = 1;
const WEALTH_LIST: u32 = 2;

pub fn sheet(part: u32) -> Vec<(Tag, Option<u32>)> {
    Vec::from([(Tag::Main, None), (Tag::Section, Some(SHEET_SECTION)), (Tag::Section, Some(part))])
}

pub fn join(mut path: Vec<(Tag, Option<u32>)>, tail: &[(Tag, Option<u32>)]) -> Id {
    path.extend_from_slice(tail);
    Id::new(&path)
}

fn entry(part: u32, tag: Tag, id: u32, tail: &[(Tag, Option<u32>)]) -> Id {
    let mut path = sheet(part);
    path.extend([(Tag::Dl, None), (tag, Some(id))]);
    join(path, tail)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Management {
    Identity,
    CreatedAt,
    ModifiedAt,
}

impl Management {
    pub const fn number(&self) -> u32 {
        match self {
            Self::Identity => 1,
            Self::CreatedAt => 2,
            Self::ModifiedAt => 3,
        }
    }

    pub fn from_number(number: u32) -> Option<Self> {
        Self::list().iter().find(|m| m.number() == number).copied()
    }

    pub fn list() -> &'static [Self] {
        &[Self::Identity, Self::CreatedAt, Self::ModifiedAt]
    }

    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Identity, _) => "ID",
            (Self::CreatedAt, Language::En(_)) => "Created",
            (Self::CreatedAt, Language::Ja) => "作成日",
            (Self::ModifiedAt, Language::En(_)) => "Modified",
            (Self::ModifiedAt, Language::Ja) => "更新日",
        }
    }

    pub fn section() -> Id {
        Id::new(&sheet(MANAGEMENT_SECTION))
    }

    pub fn dom(&self) -> [Id; 2] {
        [
            entry(MANAGEMENT_SECTION, Tag::Dt, self.number(), &[]),
            entry(MANAGEMENT_SECTION, Tag::Dd, self.number(), &[]),
        ]
    }
}

impl Section {
    pub fn from_number(number: u32) -> Option<Self> {
        Self::list().iter().find(|part| part.number() == number).copied()
    }

    pub fn dom(&self) -> [Id; 2] {
        [join(sheet(self.number()), &[(Tag::H3, None)]), Id::new(&sheet(self.number()))]
    }
}

impl Profile {
    pub fn number(&self) -> u32 {
        self.numbers()[0]
    }

    pub fn from_number(number: u32) -> Option<Self> {
        Self::list().iter().find(|field| field.numbers().contains(&number)).copied()
    }

    pub fn dom(&self) -> Vec<Id> {
        const PART: u32 = Section::Profile.number();
        let input = |id| entry(PART, Tag::Dd, id, &[(Tag::Input, None)]);
        let ids = self.numbers();
        let mut dom = Vec::from([entry(PART, Tag::Dt, ids[0], &[])]);
        match self {
            Self::Name => dom.extend([input(ids[0]), input(ids[1])]),
            Self::Occupation => dom.extend([input(ids[0]), input(ids[2])]),
            Self::Age => dom.extend([
                input(ids[0]),
                entry(PART, Tag::Dd, ids[0], &[(Tag::Button, Some(1))]),
                entry(PART, Tag::Dd, ids[0], &[(Tag::Button, Some(2))]),
            ]),
            _ => dom.push(input(ids[0])),
        }
        dom
    }
}

impl Characteristic {
    pub fn dom(&self) -> [Id; 5] {
        let mut path = sheet(Section::Characteristic.number());
        path.extend([(Tag::Table, None), (Tag::Tbody, None), (Tag::Tr, Some(self.number()))]);
        let cell = |column, input: bool| {
            let mut tail = Vec::from([(Tag::Td, Some(column))]);
            if input {
                tail.push((Tag::Input, None));
            }
            join(path.clone(), &tail)
        };
        [
            join(path.clone(), &[(Tag::Th, None)]),
            cell(1, true),
            cell(2, true),
            cell(3, true),
            cell(4, false),
        ]
    }
}

impl SecondaryAttribute {
    pub fn from_number(number: u32) -> Option<Self> {
        Self::list().iter().find(|attribute| attribute.number() == number).copied()
    }

    pub fn list() -> &'static [Self] {
        &[
            Self::HitPoints,
            Self::MagicPoints,
            Self::Luck,
            Self::Sanity,
            Self::Build,
            Self::DamageBonus,
            Self::MoveRate,
            Self::OccupationSkillPoints,
            Self::InterestSkillPoints,
        ]
    }

    pub fn editable(&self) -> bool {
        matches!(self, Self::Luck | Self::Sanity)
    }

    pub fn dom(&self) -> [Id; 2] {
        const PART: u32 = Section::SecondaryAttribute.number();
        let tail: &[(Tag, Option<u32>)] = if self.editable() { &[(Tag::Input, None)] } else { &[] };
        [entry(PART, Tag::Dt, self.number(), &[]), entry(PART, Tag::Dd, self.number(), tail)]
    }
}

pub struct SkillCells {
    pub name:           Id,
    pub specialization: Id,
    pub base:           Id,
    pub occupation:     Id,
    pub interest:       Id,
    pub change:         Id,
    pub modifier:       Id,
    pub sum:            Id,
}

impl Skill {
    pub fn free_row(&self) -> Option<u32> {
        match self {
            Self::ArtAndCraft => Some(self.row() + ArtAndCraft::Custom.number()),
            Self::Fighting => Some(self.row() + Fighting::Custom.number()),
            Self::Firearms => Some(self.row() + Firearms::Custom.number()),
            Self::LanguageOther => Some(self.row() + 1),
            Self::Pilot => Some(self.row() + Pilot::Custom.number()),
            Self::Science => Some(self.row() + Science::Custom.number()),
            Self::Survival => Some(self.row() + Survival::Custom.number()),
            _ => None,
        }
    }

    pub fn rows(&self) -> Vec<u32> {
        let mut rows = Vec::from([self.row()]);
        rows.extend(self.free_row());
        rows
    }

    pub fn from_row(row: u32) -> Option<(Self, bool)> {
        Self::list().iter().find_map(|skill| {
            if skill.row() == row {
                Some((skill.clone(), false))
            } else if skill.free_row() == Some(row) {
                Some((skill.clone(), true))
            } else {
                None
            }
        })
    }

    fn row_path(row: u32) -> Vec<(Tag, Option<u32>)> {
        let mut path = sheet(Section::Skill.number());
        path.extend([(Tag::Table, None), (Tag::Tbody, None), (Tag::Tr, Some(row))]);
        path
    }

    pub fn cells(row: u32) -> SkillCells {
        let path = Self::row_path(row);
        let listbox =
            matches!(Self::from_row(row), Some((skill, false)) if skill.free_row().is_some());
        let cell = |column, input: bool| {
            let mut tail = Vec::from([(Tag::Td, Some(column))]);
            if input {
                tail.push((Tag::Input, None));
            }
            join(path.clone(), &tail)
        };
        let specialization = if listbox {
            join(path.clone(), &[(Tag::Td, Some(1)), (Tag::Label, None), (Tag::Input, None)])
        } else {
            cell(1, true)
        };
        SkillCells {
            name: join(path.clone(), &[(Tag::Th, None)]),
            specialization,
            base: cell(2, false),
            occupation: cell(3, true),
            interest: cell(4, true),
            change: cell(5, true),
            modifier: cell(6, true),
            sum: cell(7, false),
        }
    }

    pub fn option(row: u32, n: u32) -> Id {
        join(Self::row_path(row), &[(Tag::Td, Some(1)), (Tag::Div, None), (Tag::Button, Some(n))])
    }

    pub fn options(&self, language: Language) -> Vec<(Id, String)> {
        let row = self.row();
        let named = |n: u32, name: &str| (Self::option(row, n), String::from(name));
        match self {
            Self::ArtAndCraft => ArtAndCraft::list()
                .iter()
                .filter(|a| **a != ArtAndCraft::Custom)
                .map(|a| named(a.number(), a.display(language)))
                .collect(),
            Self::Fighting => Fighting::list()
                .iter()
                .filter(|f| **f != Fighting::Custom)
                .map(|f| named(f.number(), f.display(language)))
                .collect(),
            Self::Firearms => Firearms::list()
                .iter()
                .filter(|f| **f != Firearms::Custom)
                .map(|f| named(f.number(), f.display(language)))
                .collect(),
            Self::Pilot => Pilot::list()
                .iter()
                .filter(|p| **p != Pilot::Custom)
                .map(|p| named(p.number(), p.display(language)))
                .collect(),
            Self::Science => Science::list()
                .iter()
                .filter(|s| **s != Science::Custom)
                .map(|s| named(s.number(), s.display(language).unwrap_or("")))
                .collect(),
            Self::Survival => Survival::list()
                .iter()
                .filter(|s| **s != Survival::Custom)
                .map(|s| named(s.number(), s.display(language)))
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl Possession {
    pub fn from_number(number: u32) -> Option<Self> {
        Self::list().iter().find(|possession| possession.number() == number).copied()
    }

    pub fn list() -> &'static [Self] {
        &[Self::Weapon, Self::Armor, Self::Equipment, Self::Wealth]
    }

    pub fn dom(&self) -> [Id; 2] {
        const PART: u32 = Section::Possession.number();
        let tail: &[(Tag, Option<u32>)] = match self {
            Self::Weapon => &[(Tag::Table, Some(1))],
            Self::Armor => &[(Tag::Table, Some(2))],
            Self::Equipment => &[(Tag::Dl, Some(EQUIPMENT_LIST))],
            Self::Wealth => &[(Tag::Dl, Some(WEALTH_LIST))],
        };
        [join(sheet(PART), &[(Tag::P, Some(self.number()))]), join(sheet(PART), tail)]
    }

    pub fn cells(&self) -> Vec<Id> {
        let (rows, columns) = match self {
            Self::Weapon => (WEAPON_ROWS, WEAPON_COLUMNS),
            Self::Armor => (ARMOR_ROWS, ARMOR_COLUMNS),
            Self::Equipment => {
                let [_, list] = self.dom();
                return (1..=EQUIPMENT_ROWS)
                    .flat_map(|row| {
                        let list = list.clone();
                        let mut path: Vec<_> = list.0.iter().map(|s| (s.tag, s.n)).collect();
                        path.push((Tag::Dt, Some(row)));
                        let name = join(path, &[(Tag::Input, None)]);
                        core::iter::once(name).chain((1..=EQUIPMENT_DETAILS).map(move |detail| {
                            let mut path: Vec<_> = list.0.iter().map(|s| (s.tag, s.n)).collect();
                            path.push((Tag::Dd, Some((row - 1) * EQUIPMENT_DETAILS + detail)));
                            join(path, &[(Tag::Input, None)])
                        }))
                    })
                    .collect();
            }
            Self::Wealth => {
                return WealthField::list().iter().flat_map(WealthField::dom).collect();
            }
        };
        let [_, table] = self.dom();
        (1..=rows)
            .flat_map(|row| {
                let table = table.clone();
                (1..=columns).map(move |column| {
                    let mut path: Vec<_> = table.0.iter().map(|s| (s.tag, s.n)).collect();
                    path.extend([
                        (Tag::Tbody, None),
                        (Tag::Tr, Some(row)),
                        (Tag::Td, Some(column)),
                        (Tag::Input, None),
                    ]);
                    Id::new(&path)
                })
            })
            .collect()
    }

    pub fn heads(&self) -> Vec<Id> {
        let columns = match self {
            Self::Weapon => WEAPON_COLUMNS,
            Self::Armor => ARMOR_COLUMNS,
            _ => return Vec::new(),
        };
        let [_, table] = self.dom();
        (1..=columns)
            .map(|column| {
                let mut path: Vec<_> = table.0.iter().map(|s| (s.tag, s.n)).collect();
                path.extend([(Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(column))]);
                Id::new(&path)
            })
            .collect()
    }
}

impl WealthField {
    pub const fn number(&self) -> u32 {
        match self {
            Self::SpendingLevel => 1,
            Self::Cash => 2,
            Self::Assets => 3,
        }
    }

    pub fn from_number(number: u32) -> Option<Self> {
        Self::list().iter().find(|field| field.number() == number).copied()
    }

    pub fn list() -> &'static [Self] {
        &[Self::SpendingLevel, Self::Cash, Self::Assets]
    }

    pub fn dom(&self) -> [Id; 2] {
        let mut path = sheet(Section::Possession.number());
        path.push((Tag::Dl, Some(WEALTH_LIST)));
        [
            join(path.clone(), &[(Tag::Dt, Some(self.number()))]),
            join(path, &[(Tag::Dd, Some(self.number())), (Tag::Input, None)]),
        ]
    }
}

impl Backstory {
    pub fn from_number(number: u32) -> Option<Self> {
        Self::list().into_iter().find(|entry| entry.number() == number)
    }

    pub fn list() -> [Self; 10] {
        [
            Self::PersonalDescription,
            Self::IdeologyAndBeliefs,
            Self::SignificantPeople,
            Self::MeaningfulLocation,
            Self::TreasuredPossession,
            Self::Trait,
            Self::InjuresAndScars,
            Self::PhobiasAndManias,
            Self::ArcaneTomesAndSpells,
            Self::EncountersWithStrangeEntities,
        ]
    }

    pub fn dom(&self) -> Vec<Id> {
        const PART: u32 = Section::Backstory.number();
        match self {
            Self::KeyConnection(target) => {
                Vec::from([entry(PART, Tag::Dt, target.number(), &[(Tag::Input, None)])])
            }
            entry_ => Vec::from([
                entry(PART, Tag::Dt, entry_.number(), &[(Tag::Label, None)]),
                entry(PART, Tag::Dd, entry_.number(), &[(Tag::Textarea, None)]),
            ]),
        }
    }
}

impl Memo {
    pub fn dom(slot: usize) -> [Id; 2] {
        const PART: u32 = Section::Memo.number();
        let number = PART + slot as u32;
        [
            entry(PART, Tag::Dt, number, &[(Tag::Input, None)]),
            entry(PART, Tag::Dd, number, &[(Tag::Textarea, None)]),
        ]
    }
}

impl Backstory {
    pub fn index(&self) -> Option<usize> {
        Self::list().iter().position(|entry| entry == self)
    }
}

impl Skill {
    pub fn index(&self) -> usize {
        Self::list().iter().position(|skill| skill == self).unwrap_or(0)
    }
}

pub fn every_dom() -> Vec<Id> {
    let mut ids = Vec::from([Management::section()]);
    ids.extend(Management::list().iter().flat_map(Management::dom));
    ids.extend(Section::list().iter().flat_map(Section::dom));
    ids.extend(Profile::list().iter().flat_map(Profile::dom));
    ids.extend(Characteristic::list().iter().flat_map(Characteristic::dom));
    ids.extend(SecondaryAttribute::list().iter().flat_map(SecondaryAttribute::dom));
    for skill in Skill::list() {
        for row in skill.rows() {
            let cells = Skill::cells(row);
            ids.extend([
                cells.name,
                cells.specialization,
                cells.base,
                cells.occupation,
                cells.interest,
                cells.change,
                cells.modifier,
                cells.sum,
            ]);
        }
        ids.extend(skill.options(Language::Ja).into_iter().map(|(id, _)| id));
    }
    for possession in Possession::list() {
        ids.extend(possession.dom());
        ids.extend(possession.heads());
        ids.extend(possession.cells());
    }
    for entry in Backstory::list() {
        ids.extend(Backstory::KeyConnection(Box::new(entry.clone())).dom());
        ids.extend(entry.dom());
    }
    ids.extend((0..MEMO_SLOTS).flat_map(Memo::dom));
    ids
}

#[cfg(test)]
mod dom_tests {
    use alloc::{format, string::String, vec::Vec};
    use std::fs;

    use super::*;

    fn html_id(id: &Id) -> String {
        let mut parts = Vec::new();
        for segment in &id.0 {
            let name = format!("{:?}", segment.tag).to_lowercase();
            parts.push(match segment.n {
                Some(n) => format!("{name}-{n}"),
                None => name,
            });
        }
        parts.join("_")
    }

    #[test]
    fn every_item_has_a_home_in_the_html() {
        let html = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/distribution/coc7th/index.html"
        ))
        .unwrap();
        for id in every_dom() {
            let id = html_id(&id);
            assert!(html.contains(&format!("id=\"{id}\"")), "{id}");
        }
    }

    #[test]
    fn every_item_has_a_distinct_home() {
        let ids = every_dom();
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b, "{a:?}");
            }
        }
    }

    #[test]
    fn numbers_round_trip_through_from_number() {
        for field in Profile::list() {
            assert_eq!(Profile::from_number(field.number()), Some(*field));
        }
        for attribute in SecondaryAttribute::list() {
            assert_eq!(SecondaryAttribute::from_number(attribute.number()), Some(*attribute));
        }
        for skill in Skill::list() {
            let rows = skill.rows();
            assert_eq!(Skill::from_row(rows[0]), Some((skill.clone(), false)));
            if let Some(free) = rows.get(1) {
                assert_eq!(Skill::from_row(*free), Some((skill.clone(), true)));
            }
        }
        for entry in Backstory::list() {
            assert_eq!(Backstory::from_number(entry.number()), Some(entry));
        }
        assert_eq!(Management::from_number(2), Some(Management::CreatedAt));
        assert_eq!(Section::from_number(40), Some(Section::Skill));
        assert_eq!(WealthField::from_number(2), Some(WealthField::Cash));
    }

    #[test]
    fn skill_rows_do_not_collide() {
        let rows: Vec<u32> = Skill::list().iter().flat_map(Skill::rows).collect();
        for (i, a) in rows.iter().enumerate() {
            assert!(!rows[i + 1..].contains(a), "{a}");
        }
        assert_eq!(rows.len(), 54);
    }
}

#[cfg(test)]
mod record_tests {
    use alloc::{string::ToString, vec};

    use super::*;
    use crate::{Rng, parse};

    fn sample() -> Investigator {
        let mut investigator = Investigator {
            name: "鏖".to_string(),
            pronunciation: "じん".to_string(),
            occupation: OccupationKind::Soldier,
            title: "Sergeant".to_string(),
            age: 39,
            build: -1,
            occupation_skill_points: Some((Characteristic::Education, Characteristic::Strength)),
            key_connection: Some(Backstory::Trait),
            memos: vec![Memo { title: "Scenarios".to_string(), body: "A\nB".to_string() }],
            ..Investigator::ZERO
        };
        investigator.characteristics[Characteristic::Strength.index()] =
            Score { initial: 75, change: -5, modifier: 10 };
        *investigator.points_mut(&Skill::Dodge) = Points::from((48, 0, 0, 0));
        investigator.specializations.push(Specialization {
            specialty: Specialty::Fighting(Fighting::Sword),
            name:      String::new(),
            base:      20,
            points:    Points::from((60, 0, -512, 511)),
        });
        investigator.specializations.push(Specialization {
            specialty: Specialty::ArtAndCraft(ArtAndCraft::Custom),
            name:      "Origami".to_string(),
            base:      5,
            points:    Points::from((0, 10, 0, 0)),
        });
        investigator.backstory[Backstory::Trait.index().unwrap()] = "低音声".to_string();
        investigator
    }

    #[test]
    fn an_investigator_survives_the_byte_round_trip() {
        let investigator = sample();
        assert_eq!(parse::<Investigator>(&investigator.to_bytes()), Some(investigator));
        let blank = Investigator::ZERO;
        assert_eq!(parse::<Investigator>(&blank.to_bytes()), Some(blank));
    }

    #[test]
    fn a_truncated_investigator_does_not_decode() {
        let bytes = sample().to_bytes();
        for length in [0, 1, bytes.len() / 2, bytes.len() - 1] {
            assert!(parse::<Investigator>(&bytes[..length]).is_none(), "length {length}");
        }
    }

    #[test]
    fn every_specialty_survives_the_byte_round_trip() {
        let mut specialties =
            vec![Specialty::LanguageOwn, Specialty::LanguageOther, Specialty::Custom];
        specialties.extend(ArtAndCraft::list().iter().map(|a| Specialty::ArtAndCraft(*a)));
        specialties.extend(Fighting::list().iter().map(|f| Specialty::Fighting(*f)));
        specialties.extend(Firearms::list().iter().map(|f| Specialty::Firearms(*f)));
        specialties.extend(Pilot::list().iter().map(|p| Specialty::Pilot(*p)));
        specialties.extend(Science::list().iter().map(|s| Specialty::Science(*s)));
        specialties.extend(Survival::list().iter().map(|s| Specialty::Survival(*s)));
        for skill in Skill::list() {
            specialties.extend(Specialty::custom(skill));
        }
        for specialty in specialties {
            assert_eq!(parse::<Specialty>(&specialty.to_bytes()), Some(specialty), "{specialty:?}");
        }
    }

    #[test]
    fn points_keep_their_bit_layout_at_the_extremes() {
        for seed in 0..500 {
            let mut rng = Rng::new(seed);
            let extreme = |rng: &mut Rng, low: i32, high: i32| match rng.below(4) {
                0 => low,
                1 => high,
                _ => low + rng.below((high - low + 1) as usize) as i32,
            };
            let points = Points::from((
                extreme(&mut rng, 0, 511) as u16,
                extreme(&mut rng, 0, 511) as u16,
                extreme(&mut rng, -512, 511) as i16,
                extreme(&mut rng, -512, 511) as i16,
            ));
            let bytes = points.to_bytes();
            assert_eq!(bytes.len(), SkillValue::BYTES);
            assert_eq!(parse::<Points>(&bytes), Some(points), "seed {seed}");
        }
    }

    #[test]
    fn the_skill_list_matches_the_stored_slot_count() {
        assert_eq!(Skill::list().len(), SKILL_COUNT);
        assert_eq!(Backstory::list().len(), BACKSTORY_COUNT);
        assert_eq!(Characteristic::list().len(), 8);
        for (index, skill) in Skill::list().iter().enumerate() {
            assert_eq!(skill.index(), index);
        }
    }
}
