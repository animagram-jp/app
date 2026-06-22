use core::{primitive::{u8, i8, i32}, array::from_fn};
use alloc::string::String;
use crate::{Lang, En};
use crate::list::ListError;
use crate::timestamp::Field;
use crate::data_struct::DataStruct;

// ============================================================
// Dice, dice::{display, roll}
// ============================================================

pub type Dice = (i8, u8, i8); // (count, sides, modifier)
type DamageBonusTuple = (i8, u8, i8); // (count, sides, modifier)

pub mod dice {
    use super::Dice;
    use rand::RngExt as _;

    pub fn display(dice: &[Dice]) -> String {
        let s = dice.iter().map(|&(count, sides, modifier)| {
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
        }).collect::<String>();
        s.trim_start_matches('+').to_string()
    }

    pub fn roll(dice: &[Dice]) -> i32 {
        dice.iter().map(|&(count, sides, modifier)| {
            let rolled = if count != 0 && sides > 0 {
                let mut rng = rand::rng();
                let sum: i32 = (0..count.unsigned_abs())
                    .map(|_| rng.random_range(1..=sides as i32))
                    .sum();
                if count < 0 { -sum } else { sum }
            } else {
                0
            };
            rolled + modifier as i32
        }).sum()
    }
}

// ============================================================
// Character::{Profile, Characteristic, Secondary Attribute, Skill, Posession, Backstory, Memo}
// ============================================================

pub enum Character {
    Profile,
    Characteristic,
    SecondaryAttribute,
    Skill,
    Possession,
    Backstory,
    Memo,
}

impl Character {

    pub fn display(&self, lang: Lang) -> 'static &str {
        match (self, lang) {
            (Self::Profile, Lang::En(_))        => "Profile",
            (Self::Profile, Lang::Ja)           => "プロフィール",
            (Self::Characteristic, Lang::En(_)) => "Characteristics",
            (Self::Characteristic, Lang::Ja)    => "能力値",
            (Self::SecondaryAttribute, Lang::En(_)) => "Secondary Attributes",
            (Self::SecondaryAttribute, Lang::Ja)    => "ほかの属性",
            (Self::Skill, Lang::En(_))          => "Skills",
            (Self::Skill, Lang::Ja)             => "技能",
            (Self::Possession, Lang::En(_))     => "Gear & Possessions",
            (Self::Possession, Lang::Ja)        => "装備と所持品",
            (Self::Backstory, Lang::En(_))      => "Backstory",
            (Self::Backstory, Lang::Ja)         => "バックストーリー",
            (Self::Memo, Lang::En(_))           => "Memo",
            (Self::Memo, Lang::Ja)              => "メモ",
        }
    }
    pub const fn id(&self) -> u32 {
        match self {
            Self::Profile            =>  10, //  10- 17 (8件)
            Self::Characteristic     =>  20, //  20- 28 (9件)
            Self::SecondaryAttribute =>  30, //  30- 37 (8件)
            Self::Skill              =>  40, //  40- 86 (47件)
            Self::Possession         =>  90, //  90-... (拡張余地)
            Self::Backstory          => 100, // 100-109 (10件)
            Self::Memo               => 110, // 110      (1件)
        }
    }
}

// ============================================================
// Profile::{Name, Birthppalce, Pronoun, Occupation, Residence, Age}
// ============================================================

#[derive(Clone, Copy)]
pub enum Profile {
    Name,
    Birthpalce,
    Pronoun,
    Occupation,
    Residence,
    Age,
}

impl Profile {

    pub fn ids(&self) -> &'static [u32] {
        const BASE = Character::Profile::id();
        match self {
            Self::Name       => &[BASE + 0, BASE + 1],
            Self::Birthpalce => &[BASE + 2],
            Self::Pronoun    => &[BASE + 3],
            Self::Occupation => &[BASE + 4, BASE + 5,BASE + 6],
            Self::Residence  => &[BASE + 7],
            Self::Age        => &[BASE + 8],
        }
    }

    pub fn display(&self, lang: Lang) -> 'static &str {
        match (self, lang) {
            (Self::Name, Lang::En(_)) => "Name",
            (Self::Name, Lang::Ja)    => "名前",
            (Self::Birthpalce, Lang::En(_)) => "Birthplace",
            (Self::Birthpalce, Lang::Ja)    => "出身",
            (Self::Pronoun, Lang::En(_)) => "Pronoun",
            (Self::Pronoun, Lang::Ja)    => "性別",
            (Self::Occupation, Lang::En(_)) => "Occupation",
            (Self::Occupation, Lang::Ja)    => "職業",
            (Self::Residence, Lang::En(_)) => "Residence",
            (Self::Residence, Lang::Ja)    => "住所",
            (Self::Age, Lang::En(_)) => "Age",
            (Self::Age, Lang::Ja)    => "年齢",
        }
    }

    pub fn list() -> &'static [Profile] {
        &[
            Self::Name,
            Self::Birthpalce,
            Self::Pronoun,
            Self::Occupation,
            Self::Residence,
            Self::Age,
        ]
    }
}

pub struct Name;

impl Name {

    pub fn read(character: &DataStruct) -> (String, Option<String>) { // name, complement
        let ids = Profile::Name.ids();
        let name = character.get(ids[0]).ok()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let complement = character.get(ids[1]).ok()
            .map(|b| String::from_utf8_lossy(b).into_owned());
        (name, complement)
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: (&str, Option<&str>)) -> &'a mut DataStruct {
        let ids = Profile::Name.ids();
        let _ = character.set(ids[0], value.0.as_bytes(), None);
        match value.1 {
            Some(complement) => { let _ = character.set(ids[1], complement.as_bytes(), None); }
            None => { let _ = character.delete(ids[1]); }
        }
        character
    }

    pub fn display(name: &String, complement: &Option<String>) -> String {
        match complement {
            Some(c) if !c.is_empty() => format!("{name} ({c})"),
            _ => name,
        }
    }
}

pub enum OccupationKind { // p.38
    Activist,
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
            1  => OccupationKind::Activist,
            2  => OccupationKind::Antiquarian,
            3  => OccupationKind::Artist,
            4  => OccupationKind::Athlete,
            5  => OccupationKind::Author,
            6  => OccupationKind::Clergy,
            7  => OccupationKind::Criminal,
            8  => OccupationKind::Dilettante,
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
            _  => OccupationKind::Custom,
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Self::Activist            =>  1,
            Self::Antiquarian         =>  2,
            Self::Artist              =>  3,
            Self::Athlete             =>  4,
            Self::Author              =>  5,
            Self::Clergy              =>  6,
            Self::Criminal            =>  7,
            Self::Dilettante          =>  8,
            Self::Doctor              =>  9,
            Self::Drifter             => 10,
            Self::Engineer            => 11,
            Self::Entertainer         => 12,
            Self::Farmer              => 13,
            Self::Hacker              => 14,
            Self::Journalist          => 15,
            Self::Lawyer              => 16,
            Self::Librarian           => 17,
            Self::MilitaryOfficer     => 18,
            Self::Missionary          => 19,
            Self::Musician            => 20,
            Self::Parapsychologist    => 21,
            Self::Pilot               => 22,
            Self::Police              => 23,
            Self::PoliceDetective     => 24,
            Self::PrivateInvestigator => 25,
            Self::Professor           => 26,
            Self::Soldier             => 27,
            Self::TribeMember         => 28,
            Self::Custom              => 29,
        }
    }
}

pub struct Occupation;

impl Occupation {
    // ids[0]: kind_id (u8), ids[1]: custom_name (str), ids[2]: title (str)
    pub fn read(character: &DataStruct) -> (OccupationKind, Option<String>, Option<String>) {
        let ids = Profile::Occupation.ids();
        let kind_id = character.get(ids[0]).ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0);
        let custom_name = character.get(ids[1]).ok()
            .map(|b| String::from_utf8_lossy(b).into_owned());
        let title = character.get(ids[2]).ok()
            .map(|b| String::from_utf8_lossy(b).into_owned());
        let kind = OccupationKind::detect(kind_id);
        (kind, custom_name, title)
    }

    pub fn write<'a>(character: &'a mut DataStruct, kind_id: u8, custom_name: Option<&str>, title: Option<&str>) -> &'a mut DataStruct {
        let ids = Profile::Occupation.ids();
        let _ = character.set(ids[0], &[kind_id], None);
        match custom_name {
            Some(v) => { let _ = character.set(ids[1], v.as_bytes(), None); }
            None    => { let _ = character.delete(ids[1]); }
        }
        match title {
            Some(t) => { let _ = character.set(ids[2], t.as_bytes(), None); }
            None    => { let _ = character.delete(ids[2]); }
        }
        character
    }

    pub fn display(kind: &OccupationKind, custom_name: Option<&str>, title: Option<&str>, lang: Lang) -> String {
        let name = match (kind, lang) {
            (OccupationKind::Activist,            Lang::En(_)) => "Activist",
            (OccupationKind::Activist,            Lang::Ja)    => "活動家",
            (OccupationKind::Antiquarian,         Lang::En(_)) => "Antiquarian",
            (OccupationKind::Antiquarian,         Lang::Ja)    => "古物研究家",
            (OccupationKind::Artist,              Lang::En(_)) => "Artist",
            (OccupationKind::Artist,              Lang::Ja)    => "芸術家",
            (OccupationKind::Athlete,             Lang::En(_)) => "Athlete",
            (OccupationKind::Athlete,             Lang::Ja)    => "スポーツ選手",
            (OccupationKind::Author,              Lang::En(_)) => "Author",
            (OccupationKind::Author,              Lang::Ja)    => "作家",
            (OccupationKind::Clergy,              Lang::En(_)) => "Clergy",
            (OccupationKind::Clergy,              Lang::Ja)    => "聖職者",
            (OccupationKind::Criminal,            Lang::En(_)) => "Criminal",
            (OccupationKind::Criminal,            Lang::Ja)    => "犯罪者",
            (OccupationKind::PoliceDetective,     Lang::En(_)) => "Police Detective",
            (OccupationKind::PoliceDetective,     Lang::Ja)    => "刑事",
            (OccupationKind::Dilettante,          Lang::En(_)) => "Dilettante",
            (OccupationKind::Dilettante,          Lang::Ja)    => "ディレッタント",
            (OccupationKind::Doctor,              Lang::En(_)) => "Doctor",
            (OccupationKind::Doctor,              Lang::Ja)    => "医師",
            (OccupationKind::Drifter,             Lang::En(_)) => "Drifter",
            (OccupationKind::Drifter,             Lang::Ja)    => "放浪者",
            (OccupationKind::Engineer,            Lang::En(_)) => "Engineer",
            (OccupationKind::Engineer,            Lang::Ja)    => "技術者",
            (OccupationKind::Entertainer,         Lang::En(_)) => "Entertainer",
            (OccupationKind::Entertainer,         Lang::Ja)    => "エンターテイナー",
            (OccupationKind::Farmer,              Lang::En(_)) => "Farmer",
            (OccupationKind::Farmer,              Lang::Ja)    => "農民",
            (OccupationKind::Hacker,              Lang::En(_)) => "Hacker",
            (OccupationKind::Hacker,              Lang::Ja)    => "ハッカー",
            (OccupationKind::Journalist,          Lang::En(_)) => "Journalist",
            (OccupationKind::Journalist,          Lang::Ja)    => "ジャーナリスト",
            (OccupationKind::Lawyer,              Lang::En(_)) => "Lawyer",
            (OccupationKind::Lawyer,              Lang::Ja)    => "弁護士",
            (OccupationKind::Librarian,           Lang::En(_)) => "Librarian",
            (OccupationKind::Librarian,           Lang::Ja)    => "司書",
            (OccupationKind::MilitaryOfficer,     Lang::En(_)) => "Military Officer",
            (OccupationKind::MilitaryOfficer,     Lang::Ja)    => "士官",
            (OccupationKind::Missionary,          Lang::En(_)) => "Missionary",
            (OccupationKind::Missionary,          Lang::Ja)    => "伝道者",
            (OccupationKind::Musician,            Lang::En(_)) => "Musician",
            (OccupationKind::Musician,            Lang::Ja)    => "ミュージシャン",
            (OccupationKind::Parapsychologist,    Lang::En(_)) => "Parapsychologist",
            (OccupationKind::Parapsychologist,    Lang::Ja)    => "超心理学者",
            (OccupationKind::Pilot,               Lang::En(_)) => "Pilot",
            (OccupationKind::Pilot,               Lang::Ja)    => "パイロット",
            (OccupationKind::Police,              Lang::En(_)) => "Police",
            (OccupationKind::Police,              Lang::Ja)    => "警察官",
            (OccupationKind::PrivateInvestigator, Lang::En(_)) => "Private Investigator",
            (OccupationKind::PrivateInvestigator, Lang::Ja)    => "私立探偵",
            (OccupationKind::Professor,           Lang::En(_)) => "Professor",
            (OccupationKind::Professor,           Lang::Ja)    => "教授",
            (OccupationKind::Soldier,             Lang::En(_)) => "Soldier",
            (OccupationKind::Soldier,             Lang::Ja)    => "兵士",
            (OccupationKind::TribeMember,         Lang::En(_)) => "Tribe Member",
            (OccupationKind::TribeMember,         Lang::Ja)    => "トライブ・メンバー",
            (OccupationKind::Custom,              _)           => custom_name.unwrap_or(""),
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


pub struct Birthplace; // Birthplace: str

impl Birthplace {
    pub fn read(character: &DataStruct) -> String {
        character.get(Profile::Birthpalce.ids()[0]).ok()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default()
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: &str) -> &'a mut DataStruct {
        let _ = character.set(Profile::Birthpalce.ids()[0], value.as_bytes(), None);
        character
    }
}

pub struct Pronoun; // Pronoun: str

impl Pronoun {
    pub fn read(character: &DataStruct) -> String {
        character.get(Profile::Pronoun.ids()[0]).ok()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default()
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: &str) -> &'a mut DataStruct {
        let _ = character.set(Profile::Pronoun.ids()[0], value.as_bytes(), None);
        character
    }
}

pub struct Residence; // Residence: str

impl Residence {

    pub fn read(character: &DataStruct) -> String {
        character.get(Profile::Residence.ids()[0]).ok()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default()
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: &str) -> &'a mut DataStruct {
        let _ = character.set(Profile::Residence.ids()[0], value.as_bytes(), None);
        character
    }
}

pub struct Age; // Age: u16

impl Age {

    pub fn read(character: &DataStruct) -> u16 {
        character.get(Profile::Age.ids()[0]).ok()
            .and_then(|b| b.get(0..2)?.try_into().ok())
            .map(u16::from_le_bytes)
            .unwrap_or(0)
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: u16) -> &'a mut DataStruct {
        let _ = character.set(Profile::Age.ids()[0], &value.to_le_bytes(), None);
        character
    }
}

// ============================================================
// Characteristics (Strength, Constitution, Size, Dexterity, Appearance, Intelligence, Power, Education)
// ============================================================

#[derive(Clone, Copy)]
pub enum Characteristic { // Characteristic {initial: u16, change: i16, modifier: i16} // p.28
    Strength,
    Constitution,
    Size,
    Dexterity,
    Appearance,
    Intelligence,
    Power,
    Education,
}

impl Characteristic {

    pub fn label(&self, lang: Lang) -> 'static &str {
        match (self, lang) {
            (Self::Strength,     _) => "STR",
            (Self::Constitution, _) => "CON",
            (Self::Size,         _) => "SIZ",
            (Self::Dexterity,    _) => "DEX",
            (Self::Appearance,   _) => "APP",
            (Self::Intelligence, _) => "INT",
            (Self::Power,        _) => "POW",
            (Self::Education,    _) => "EDU",
        }
    }

    pub const fn id(&self) -> u32 {
        Character::Characteristic.id() + match self {
            Self::Strength     => 0,
            Self::Constitution => 1,
            Self::Size         => 2,
            Self::Dexterity    => 3,
            Self::Appearance   => 4,
            Self::Intelligence => 5,
            Self::Power        => 6,
            Self::Education    => 7,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        let offset = id as u32 - Character::Characteristic.id();
        match offset {
            0 => Some(Self::Strength),
            1 => Some(Self::Constitution),
            2 => Some(Self::Size),
            3 => Some(Self::Dexterity),
            4 => Some(Self::Appearance),
            5 => Some(Self::Intelligence),
            6 => Some(Self::Power),
            7 => Some(Self::Education),
            _ => None,
        }
    }

    pub fn read(&self, character: &DataStruct) -> (u16, i16, i16) {
        character.get(self.id()).ok()
            .map(|b| {
                let initial  = b.get(0..2).and_then(|x| x.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0);
                let change   = b.get(2..4).and_then(|x| x.try_into().ok()).map(i16::from_le_bytes).unwrap_or(0);
                let modifier = b.get(4..6).and_then(|x| x.try_into().ok()).map(i16::from_le_bytes).unwrap_or(0);
                (initial, change, modifier)
            })
            .unwrap_or((0, 0, 0))
    }

    pub fn write<'a>(&self, character: &'a mut DataStruct, value: (u16, i16, i16)) -> &'a mut DataStruct {
        let mut b = Vec::with_capacity(6);
        b.extend_from_slice(&value.0.to_le_bytes());
        b.extend_from_slice(&value.1.to_le_bytes());
        b.extend_from_slice(&value.2.to_le_bytes());
        let _ = character.set(self.id(), &b, None);
        character
    }

    pub fn sum(&self, character: &DataStruct) -> i32 {
        let (initial, change, modifier) = self.read(character);
        (initial as i32 + change as i32 + modifier as i32).max(1)
    }

    pub fn target(&self, character: &DataStruct) -> (i32, i32, i32) {
        let sum = self.sum(character);
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
        // SIZ / INT / EDU は (2d6+6)×5、それ以外は 3d6×5
        match self {
            Self::Size | Self::Intelligence | Self::Education =>
                dice::roll(&[(2, 6, 6)]) as u16 * 5,
            _ => dice::roll(&[(3, 6, 0)]) as u16 * 5,
        }
    }
}

// ============================================================
// --- Secondary Attributes
// ============================================================

pub enum SecondaryAttribute {
    HitPoints,             // CON, SIZ -> u8
    MagicPoints,           // POW -> u8
    Luck,                  // u8
    Sanity,                // u8 | POW -> u8
    Build,                 // STR, SIZ -> i8
    DamageBonus,           // Build -> DamageBonusTuple
    MoveRate,              // u8 | STR, DEX, SIZ, Age -> u8
    OccupationSkillPoints, // (Characteristic, Characteristic) | (Characteristic, Characteristic) -> (u16, u16)
    InterestSkillPoints,   // INT -> (u16, u16)
}

impl SecondaryAttribute {
    pub const fn id(&self) -> u32 {
        Character::SecondaryAttribute.id() + match self {
            Self::HitPoints             => 0,
            Self::MagicPoints           => 1,
            Self::Luck                  => 2,
            Self::Sanity                => 3,
            Self::Build                 => 4,
            Self::DamageBonus           => 5,
            Self::MoveRate              => 6,
            Self::OccupationSkillPoints => 7,
            Self::InterestSkillPoints   => 8,
        }
    }

    pub fn label(&self, lang: Lang) -> 'static &str {
        match (self, lang) {
            (Self::HitPoints,                    _) => "HP",
            (Self::MagicPoints,                  _) => "MP",
            (Self::Luck,                  Lang::En(_)) => "Luck",
            (Self::Luck,                  Lang::Ja) => "幸運",
            (Self::Sanity,                Lang::En(_)) => "Sanity",
            (Self::Sanity,                Lang::Ja) => "正気度",
            (Self::Build,                 Lang::En(_)) => "Build",
            (Self::Build,                 Lang::Ja) => "ビルド",
            (Self::DamageBonus,           Lang::En(_)) => "Damage Bonus",
            (Self::DamageBonus,           Lang::Ja) => "ダメージボーナス",
            (Self::MoveRate,              Lang::En(_)) => "Move Rate",
            (Self::MoveRate,              Lang::Ja) => "移動率 (MOV)",
            (Self::OccupationSkillPoints, Lang::En(_)) => "Occupation Skill Points",
            (Self::OccupationSkillPoints, Lang::Ja) => "職業技能ポイント",
            (Self::InterestSkillPoints,   Lang::En(_)) => "Interest Skill Points",
            (Self::InterestSkillPoints,   Lang::Ja) => "興味技能ポイント",
        }
    }
}

pub struct HitPoints; // HitPoints: u8 | CON, SIZ -> u8

impl HitPoints {
    pub fn id() -> u32 { SecondaryAttribute::HitPoints.id() }

    pub fn read(character: &DataStruct) -> u8 {
        character.get(Self::id()).ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0)
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: u8) -> &'a mut DataStruct {
        let _ = character.set(Self::id(), &[value], None);
        character
    }

    pub fn derive(character: &DataStruct) -> u8 {
        let constitution = Characteristic::Constitution.sum(character);
        let size         = Characteristic::Size.sum(character);
        ((constitution + size) / 10) as u8
    }
}

pub struct MagicPoints; // MagicPoints: u8 | POW -> u8

impl MagicPoints {
    pub fn id() -> u32 { SecondaryAttribute::MagicPoints.id() }

    pub fn read(character: &DataStruct) -> u8 {
        character.get(Self::id()).ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0)
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: u8) -> &'a mut DataStruct {
        let _ = character.set(Self::id(), &[value], None);
        character
    }

    pub fn derive(character: &DataStruct) -> u8 {
        (Characteristic::Power.sum(character) / 5) as u8
    }
}

pub struct Luck; // Luck: u8 | -> u8

impl Luck {

    pub fn read(character: &DataStruct) -> u8 {
        character.get(SecondaryAttribute::Luck.id()).ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0)
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: u8) -> &'a mut DataStruct {
        let _ = character.set(SecondaryAttribute::Luck.id(), &[value], None);
        character
    }

    pub fn roll() -> u8 {
        use super::dice;
        (dice::roll(&[(3, 6, 0)]) * 5) as u8
    }
}

pub struct Sanity; // Sanity: u8 | POW -> u8

impl Sanity {
    pub fn read(character: &DataStruct) -> u8 {
        character.get(SecondaryAttribute::Sanity.id()).ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0)
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: u8) -> &'a mut DataStruct {
        let _ = character.set(SecondaryAttribute::Sanity.id(), &[value], None);
        character
    }

    pub fn derive(character: &DataStruct) -> u8 {
        Characteristic::Power.read(character).0 as u8 // todo 99以上は99へ変換
    }
}

pub struct Build; // Build: i8 | STR, SIZ -> i8

impl Build {

    pub fn read(character: &DataStruct) -> i8 {
        character.get(SecondaryAttribute::Build.id()).ok()
            .and_then(|b| b.first().copied())
            .map(|b| b as i8)
            .unwrap_or(0)
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: i8) -> &'a mut DataStruct {
        let _ = character.set(Self::id(), &[value as u8], None);
        character
    }

    pub fn derive(character: &DataStruct) -> i8 {
        let strength = Characteristic::Strength.sum(character);
        let size     = Characteristic::Size.sum(character);
        match strength + size {
             2..= 64 => -2,
            65..= 84 => -1,
            85..=124 =>  0,
           125..=164 =>  1,
           165..=204 =>  2,
           205..=284 =>  3,
           285..=364 =>  4,
           365..=444 =>  5,
           445..=524 =>  6,
            n        => (7 + (n - 525) / 80) as i8,
        }
    }
}

pub struct DamageBonus; // DamageBonus: DamageBonusTuple | Build -> DamageBonusTuple

impl DamageBonus {

    pub fn display(character: &DataStruct) -> String {
        dice::display(&[Self::read(character)])
    }

    pub fn derive(character: &DataStruct) -> DamageBonusTuple {
        match Build::read(character) {
            i8::MIN..=-2 => (0, 0, -2),
                      -1 => (0, 0, -1),
                       0 => (0, 0,  0),
                       1 => (1, 4,  0),
                       2 => (1, 6,  0),
                       3 => (2, 6,  0),
                       4 => (3, 6,  0),
                       5 => (4, 6,  0),
                       6 => (5, 6,  0),
                       n => (n - 2, 6, 0),
        }
    }
}

pub struct MoveRate; // MoveRate: u8 | STR, DEX, SIZ, Age -> u8

impl MoveRate {

    pub fn read(character: &DataStruct) -> u8 {
        character.get(SecondaryAttribute::MoveRate.id()).ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0)
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: u8) -> &'a mut DataStruct {
        let _ = character.set(SecondaryAttribute::MoveRate.id(), &[value], None);
        character
    }

    pub fn derive(character: &DataStruct) -> u8 {
        let str = Characteristic::Strength.sum(character);
        let dex = Characteristic::Dexterity.sum(character);
        let siz = Characteristic::Size.sum(character);
        let base: i32 = if str > siz && dex > siz { 9 }
                   else if str < siz && dex < siz { 7 }
                   else                           { 8 };
        let age_penalty: i32 = match Age::read(character) {
            40..=49 => 1,
            50..=59 => 2,
            60..=69 => 3,
            70..=79 => 4,
            80..    => 5,
            _       => 0,
        };
        (base - age_penalty).max(0) as u8
    }
}

pub struct OccupationSkillPoints; // OccupationSkillPoints: (Characteristic, Characteristic) | (Characteristic, Characteristic), Profile -> (u16, u16)

impl OccupationSkillPoints {

    pub fn read(character: &DataStruct) -> Option<(Characteristic, Characteristic)> {
        let b = character.get(SecondaryAttribute::OccupationSkillPoints.id()).ok()?;
        let c1 = Characteristic::from_id(*b.first()?)?;
        let c2 = Characteristic::from_id(*b.get(1)?)?;
        Some((c1, c2))
    }

    pub fn write<'a>(character: &'a mut DataStruct, value: (Characteristic, Characteristic)) -> &'a mut DataStruct {
        let _ = character.set(SecondaryAttribute::OccupationSkillPoints.id(), &[value.0.id() as u8, value.1.id() as u8], None);
        character
    }

    pub fn label(characteristic_tuple: Option<(Characteristic, Characteristic)>, lang: Lang) -> String {
        let Some((c1, c2)) = characteristic_tuple else { return String::new(); };
        if c1.id() == c2.id() {
            format!("{}×4", c1.label(lang))
        } else {
            format!("{}×2+{}×2", c1.label(lang), c2.label(lang))
        }
    }

    pub fn derive(character: &DataStruct) -> (u16, u16) {
        let Some((c1, c2)) = Self::read(character) else { return (0, 0); };
        let used = 0u16; // todo: 割り振り済みポイント: Skill::sum().0で計算
        let (c1_initial, _, c1_modifier) = c1.read(character);
        let (c2_initial, _, c2_modifier) = c2.read(character);
        let total = (c1_initial as i32 + c1_modifier as i32 + c2_initial as i32 + c2_modifier as i32) * 2;
        (used, total.max(0) as u16)
    }
}

pub struct InterestSkillPoints; // InterestSkillPoints: u16 | INT -> (u16, u16)

impl InterestSkillPoints {
    pub fn derive(character: &DataStruct) -> (u16, u16) {
        let used = 0u16; // todo: 割り振り済みポイント: Skill::sum().0で計算
        let (initial, _, modifier) = Characteristic::Intelligence.read(character);
        let total = ((initial as i32 + modifier as i32) * 2).max(0) as u16;
        (used, total)
    }
}

// ============================================================
// Skill, ArtAndCraft, Fighting, Firearms, LanguageOther, Survival p.54
// ============================================================

#[derive(Clone)]
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
    Custom, // 変動id帯予約用item
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

    pub const fn id(&self) -> u32 {
        // ルールブック記載specialization id帯、custom id帯と衝突しないように基準idを割り振る
        const BASE: u32 = Character::Skill.id();
        match self {
            Self::Accounting       => BASE +  0, //  1 slot
            Self::Anthropology     => BASE +  1, //  1 slot
            Self::Appraise         => BASE +  2, //  1 slot
            Self::Archaeology      => BASE +  3, //  1 slot
            Self::ArtAndCraft      => BASE +  4, // 13 slots (0..=12, Custom(0)=12)
            Self::Charm            => BASE + 17,
            Self::Climb            => BASE + 18,
            Self::ComputerUse      => BASE + 19,
            Self::CreditRating     => BASE + 20,
            Self::CthulhuMythos    => BASE + 21,
            Self::Disguise         => BASE + 22,
            Self::Dodge            => BASE + 23,
            Self::DriveAuto        => BASE + 24,
            Self::ElectricalRepair => BASE + 25,
            Self::Electronics      => BASE + 26,
            Self::FastTalk         => BASE + 27,
            Self::Fighting         => BASE + 28, //  9 slots (0..=8,  Custom(0)=8)
            Self::Firearms         => BASE + 37, //  7 slots (0..=6,  Custom(0)=6)
            Self::FirstAid         => BASE + 44,
            Self::History          => BASE + 45,
            Self::Intimidate       => BASE + 46,
            Self::Jump             => BASE + 47,
            Self::LanguageOther    => BASE + 48, //  1 slot  (Custom(0)=0)
            Self::LanguageOwn      => BASE + 49,
            Self::Law              => BASE + 50,
            Self::LibraryUse       => BASE + 51,
            Self::Listen           => BASE + 52,
            Self::Locksmith        => BASE + 53,
            Self::MechanicalRepair => BASE + 54,
            Self::Medicine         => BASE + 55,
            Self::NaturalWorld     => BASE + 56,
            Self::Navigate         => BASE + 57,
            Self::Occult           => BASE + 58,
            Self::Persuade         => BASE + 59,
            Self::Pilot            => BASE + 60, // 11 slots (0..=10, Custom(0)=10)
            Self::Psychoanalysis   => BASE + 71,
            Self::Psychology       => BASE + 72,
            Self::Ride             => BASE + 73,
            Self::Science          => BASE + 74, // 14 slots (0..=13, Custom(0)=13)
            Self::SleightOfHand    => BASE + 88,
            Self::SpotHidden       => BASE + 89,
            Self::Stealth          => BASE + 90,
            Self::Survival         => BASE + 91, //  4 slots (0..=3,  Custom(0)=3)
            Self::Swim             => BASE + 95,
            Self::Throw            => BASE + 96,
            Self::Track            => BASE + 97,
            Self::Custom           => BASE + 98,
        }
    }

    // 固定値の基本成功率のみ
    pub fn base(&self) -> u16 {
        match self {
            Self::Accounting       =>  5,
            Self::Anthropology     =>  1,
            Self::Appraise         =>  5,
            Self::Archaeology      =>  1,
            Self::ArtAndCraft      =>  5,
            Self::Charm            => 15,
            Self::Climb            => 20,
            Self::ComputerUse      =>  5,
            Self::CreditRating     =>  0,
            Self::CthulhuMythos    =>  0,
            Self::Disguise         =>  5,
            Self::DriveAuto        => 20,
            Self::ElectricalRepair => 10,
            Self::Electronics      =>  1,
            Self::FastTalk         =>  5,
            Self::FirstAid         => 30,
            Self::History          =>  5,
            Self::Intimidate       => 15,
            Self::Jump             => 20,
            Self::LanguageOther    =>  1,
            Self::Law              =>  5,
            Self::LibraryUse       => 20,
            Self::Listen           => 20,
            Self::Locksmith        =>  1,
            Self::MechanicalRepair => 10,
            Self::Medicine         =>  1,
            Self::NaturalWorld     => 10,
            Self::Navigate         => 10,
            Self::Occult           =>  5,
            Self::Persuade         => 10,
            Self::Psychoanalysis   =>  1,
            Self::Psychology       => 10,
            Self::Ride             =>  5,
            Self::Science          =>  1,
            Self::SleightOfHand    => 10,
            Self::SpotHidden       => 25,
            Self::Stealth          => 20,
            Self::Survival         =>  5,
            Self::Swim             => 20,
            Self::Throw            => 20,
            Self::Track            => 10,
            Self::Custom           =>  0,
            _                      =>  0,
        }
    }

    // 固定値の技能名のみ
    pub fn name(&self, lang: &Lang) -> 'static &str {
        match (self, lang) {
            (Self::Accounting,       Lang::En(_)) => "Accounting",
            (Self::Accounting,       Lang::Ja)    => "経理",
            (Self::Anthropology,     Lang::En(_)) => "Anthropology",
            (Self::Anthropology,     Lang::Ja)    => "人類学",
            (Self::Appraise,         Lang::En(_)) => "Appraise",
            (Self::Appraise,         Lang::Ja)    => "鑑定",
            (Self::Archaeology,      Lang::En(_)) => "Archaeology",
            (Self::Archaeology,      Lang::Ja)    => "考古学",
            (Self::ArtAndCraft,      Lang::En(_)) => "Art / Craft",
            (Self::ArtAndCraft,      Lang::Ja)    => "芸術/製作",
            (Self::Charm,            Lang::En(_)) => "Charm",
            (Self::Charm,            Lang::Ja)    => "魅惑",
            (Self::Climb,            Lang::En(_)) => "Climb",
            (Self::Climb,            Lang::Ja)    => "登攀",
            (Self::ComputerUse,      Lang::En(_)) => "Computer Use",
            (Self::ComputerUse,      Lang::Ja)    => "コンピューター",
            (Self::CreditRating,     Lang::En(_)) => "Credit Rating",
            (Self::CreditRating,     Lang::Ja)    => "信用",
            (Self::CthulhuMythos,    Lang::En(_)) => "Cthulhu Mythos",
            (Self::CthulhuMythos,    Lang::Ja)    => "クトゥルフ神話",
            (Self::Disguise,         Lang::En(_)) => "Disguise",
            (Self::Disguise,         Lang::Ja)    => "変装",
            (Self::Dodge,            Lang::En(_)) => "Dodge",
            (Self::Dodge,            Lang::Ja)    => "回避",
            (Self::DriveAuto,        Lang::En(_)) => "Drive Auto",
            (Self::DriveAuto,        Lang::Ja)    => "運転（自動車）",
            (Self::ElectricalRepair, Lang::En(_)) => "Elec. Repair",
            (Self::ElectricalRepair, Lang::Ja)    => "電気修理",
            (Self::Electronics,      Lang::En(_)) => "Electronics",
            (Self::Electronics,      Lang::Ja)    => "電子工学",
            (Self::FastTalk,         Lang::En(_)) => "Fast Talk",
            (Self::FastTalk,         Lang::Ja)    => "言いくるめ",
            (Self::Fighting,         Lang::En(_)) => "Fighting",
            (Self::Fighting,         Lang::Ja)    => "近接戦闘",
            (Self::Firearms,         Lang::En(_)) => "Firearms",
            (Self::Firearms,         Lang::Ja)    => "射撃",
            (Self::FirstAid,         Lang::En(_)) => "First Aid",
            (Self::FirstAid,         Lang::Ja)    => "応急手当",
            (Self::History,          Lang::En(_)) => "History",
            (Self::History,          Lang::Ja)    => "歴史",
            (Self::Intimidate,       Lang::En(_)) => "Intimidate",
            (Self::Intimidate,       Lang::Ja)    => "威圧",
            (Self::Jump,             Lang::En(_)) => "Jump",
            (Self::Jump,             Lang::Ja)    => "跳躍",
            (Self::LanguageOther,    Lang::En(_)) => "Language (Other)",
            (Self::LanguageOther,    Lang::Ja)    => "ほかの言語",
            (Self::LanguageOwn,      Lang::En(_)) => "Language (Own)",
            (Self::LanguageOwn,      Lang::Ja)    => "母国語",
            (Self::Law,              Lang::En(_)) => "Law",
            (Self::Law,              Lang::Ja)    => "法律",
            (Self::LibraryUse,       Lang::En(_)) => "Library Use",
            (Self::LibraryUse,       Lang::Ja)    => "図書館",
            (Self::Listen,           Lang::En(_)) => "Listen",
            (Self::Listen,           Lang::Ja)    => "聞き耳",
            (Self::Locksmith,        Lang::En(_)) => "Locksmith",
            (Self::Locksmith,        Lang::Ja)    => "鍵開け",
            (Self::MechanicalRepair, Lang::En(_)) => "Mech. Repair",
            (Self::MechanicalRepair, Lang::Ja)    => "機械修理",
            (Self::Medicine,         Lang::En(_)) => "Medicine",
            (Self::Medicine,         Lang::Ja)    => "医学",
            (Self::NaturalWorld,     Lang::En(_)) => "Natural World",
            (Self::NaturalWorld,     Lang::Ja)    => "自然",
            (Self::Navigate,         Lang::En(_)) => "Navigate",
            (Self::Navigate,         Lang::Ja)    => "ナビゲート",
            (Self::Occult,           Lang::En(_)) => "Occult",
            (Self::Occult,           Lang::Ja)    => "オカルト",
            (Self::Persuade,         Lang::En(_)) => "Persuade",
            (Self::Persuade,         Lang::Ja)    => "説得",
            (Self::Pilot,            Lang::En(_)) => "Pilot",
            (Self::Pilot,            Lang::Ja)    => "操縦",
            (Self::Psychoanalysis,   Lang::En(_)) => "Psychoanalysis",
            (Self::Psychoanalysis,   Lang::Ja)    => "精神分析",
            (Self::Psychology,       Lang::En(_)) => "Psychology",
            (Self::Psychology,       Lang::Ja)    => "心理学",
            (Self::Ride,             Lang::En(_)) => "Ride",
            (Self::Ride,             Lang::Ja)    => "乗馬",
            (Self::Science,          Lang::En(_)) => "Science",
            (Self::Science,          Lang::Ja)    => "科学",
            (Self::SleightOfHand,    Lang::En(_)) => "Sleight of Hand",
            (Self::SleightOfHand,    Lang::Ja)    => "手さばき",
            (Self::SpotHidden,       Lang::En(_)) => "Spot Hidden",
            (Self::SpotHidden,       Lang::Ja)    => "目星",
            (Self::Stealth,          Lang::En(_)) => "Stealth",
            (Self::Stealth,          Lang::Ja)    => "隠密",
            (Self::Survival,         Lang::En(_)) => "Survival",
            (Self::Survival,         Lang::Ja)    => "サバイバル",
            (Self::Swim,             Lang::En(_)) => "Swim",
            (Self::Swim,             Lang::Ja)    => "水泳",
            (Self::Throw,            Lang::En(_)) => "Throw",
            (Self::Throw,            Lang::Ja)    => "投擲",
            (Self::Track,            Lang::En(_)) => "Track",
            (Self::Track,            Lang::Ja)    => "追跡",
            (_, _)                               => "",
        }
    }
}

pub trait SkillTrait<const S: Skill> {
    const SKILL: Skill = S,
    const ID:   SKILL.id();   // characterインスタンス内id
    const NAME: SKILL.name(); // 技能名
    const BASE: SKILL.base(); // 基本成功率

    const OCCUPATION_POINTS: Field = Field {position: 32, mask: (1 <<  9) - 1}; // 0~400, u9, bit 32~40
    const INTEREST_POINTS:   Field = Field {position: 23, mask: (1 <<  9) - 1}; // 0~400, u9, bit 23~31
    const CHANGE:            Field = Field {position: 13, mask: (1 << 10) - 1}; // -400~400, i10, bit 13~22
    const MODIFIER:          Field = Field {position:  3, mask: (1 << 10) - 1}; // -400~400, i10, bit 3~12
    // -> occupation_points, interest_points, change, modifier
    fn read(&self,character: &DataStruct) -> (u9, u9, i10, i10) {
        bytes = character.get(Self::ID);
    }

    fn write(&self, character: 'a &mut DataStruct, occupation_points: u9, interest_points: u9, change: i10, modifier: i10) -> 'a &mut DataStruct {
        _ = character.set(Self::ID, value: [u8:5], None);
        character
    }

    // -> name, specialization
    fn as_editable_string(&self) -> (String, String) { (Self::NAME, String::new()) }

    // -> base, occupation_points, interest_points, change, modifier, sum
    fn as_editable_numeric(&self, occupation_points: u9, interest_points: u9, change: i10, modifier: i10) -> (u7, u9, u9, i10, i10, i10) {
        Self::BASE,
        occupation_points,
        interest_points,
        change,
        modifier,
        sum = Self::BASE + occupation_points + interest_points + change + modifier,
    }

    fn as_immutable_string(&self) -> String {
        match specialization {
            Some(s) => format!("{name} ({s})"),
            None    => format!("{name}"),
        }
    }
}

pub struct Accounting;    impl SkillTrait<{ Skill::Accounting    }> for Accounting    {}
pub struct Anthropology;  impl SkillTrait<{ Skill::Anthropology  }> for Anthropology  {}
pub struct Appraise;      impl SkillTrait<{ Skill::Appraise      }> for Appraise      {}
pub struct Archaeology;   impl SkillTrait<{ Skill::Archaeology   }> for Archaeology   {}
pub struct Charm;         impl SkillTrait<{ Skill::Charm         }> for Charm         {}
pub struct Climb;         impl SkillTrait<{ Skill::Climb         }> for Climb         {}
pub struct ComputerUse;   impl SkillTrait<{ Skill::ComputerUse   }> for ComputerUse   {}
pub struct Disguise;      impl SkillTrait<{ Skill::Disguise      }> for Disguise      {}
pub struct DriveAuto;     impl SkillTrait<{ Skill::DriveAuto     }> for DriveAuto     {}
pub struct ElectricalRepair; impl SkillTrait<{ Skill::ElectricalRepair }> for ElectricalRepair {}
pub struct Electronics;   impl SkillTrait<{ Skill::Electronics   }> for Electronics   {}
pub struct FastTalk;      impl SkillTrait<{ Skill::FastTalk      }> for FastTalk      {}
pub struct FirstAid;      impl SkillTrait<{ Skill::FirstAid      }> for FirstAid      {}
pub struct History;       impl SkillTrait<{ Skill::History       }> for History       {}
pub struct Intimidate;    impl SkillTrait<{ Skill::Intimidate    }> for Intimidate    {}
pub struct Jump;          impl SkillTrait<{ Skill::Jump          }> for Jump          {}
pub struct Law;           impl SkillTrait<{ Skill::Law           }> for Law           {}
pub struct LibraryUse;    impl SkillTrait<{ Skill::LibraryUse    }> for LibraryUse    {}
pub struct Listen;        impl SkillTrait<{ Skill::Listen        }> for Listen        {}
pub struct Locksmith;     impl SkillTrait<{ Skill::Locksmith     }> for Locksmith     {}
pub struct MechanicalRepair; impl SkillTrait<{ Skill::MechanicalRepair }> for MechanicalRepair {}
pub struct Medicine;      impl SkillTrait<{ Skill::Medicine      }> for Medicine      {}
pub struct NaturalWorld;  impl SkillTrait<{ Skill::NaturalWorld  }> for NaturalWorld  {}
pub struct Navigate;      impl SkillTrait<{ Skill::Navigate      }> for Navigate      {}
pub struct Occult;        impl SkillTrait<{ Skill::Occult        }> for Occult        {}
pub struct Persuade;      impl SkillTrait<{ Skill::Persuade      }> for Persuade      {}
pub struct Psychoanalysis; impl SkillTrait<{ Skill::Psychoanalysis }> for Psychoanalysis {}
pub struct Psychology;    impl SkillTrait<{ Skill::Psychology    }> for Psychology    {}
pub struct Ride;          impl SkillTrait<{ Skill::Ride          }> for Ride          {}
pub struct SleightOfHand; impl SkillTrait<{ Skill::SleightOfHand }> for SleightOfHand {}
pub struct SpotHidden;    impl SkillTrait<{ Skill::SpotHidden    }> for SpotHidden    {}
pub struct Stealth;       impl SkillTrait<{ Skill::Stealth       }> for Stealth       {}
pub struct Swim;          impl SkillTrait<{ Skill::Swim          }> for Swim          {}
pub struct Throw;         impl SkillTrait<{ Skill::Throw         }> for Throw         {}
pub struct Track;         impl SkillTrait<{ Skill::Track         }> for Track         {}

pub srtuct Dodge;
impl SkillTrait<S: Skill::Dodge> for Dodge {
    fn display_numeric(character: &DataStruct) -> (u7, u9, u9, i10, i10, i10) {
        (
        base = Characteristic::Dexterity::sum(character) / 2,
        occupation_points,
        interest_points,
        change,
        modifier,
        sum = base + occupation_points + interest_points + change + modifier,
        )
    }
}

pub struct LanguageOwn;
impl SkillTrait<S: Skill: LanguageOwn> for LanguageOwn {
    fn display_numeric(character: &DataStruct) -> (u7, u9, u9, i10, i10, i10) {
        (
        base = Characteristic::Education::sum(character),
        occupation_points,
        interest_points,
        change,
        modifier,
        sum = base + occupation_points + interest_points + change + modifier,
        )
    }
}

/// 芸術/製作 (専門分野) Art/Craft (Specialization) // p.62 モリダンス等は長いので除外
enum ArtAndCraft {
    Acting,       // 演劇
    Barber,       // 理容
    Calligraphy,  // 書道
    Carpentry,    // 大工仕事
    Cook,         // 料理
    Dancing,      // ダンス
    FineArt,      // 絵画
    Forgery,      // 文書偽造
    Photography,  // 写真術
    Pottery,      // 陶芸
    Sculpting,    // 彫刻
    Writing,      // 執筆
    Custom,
}

impl ArtAndCraft {

    pub fn id(&self, character: &DataStruct) -> u32 {
        const base = Skill::ArtAndCraft::id();
        match self {
            Self::Acting      => base +  1,
            Self::Barber      => base +  2,
            Self::Calligraphy => base +  3,
            Self::Carpentry   => base +  4,
            Self::Cook        => base +  5,
            Self::Dancing     => base +  6,
            Self::FineArt     => base +  7,
            Self::Forgery     => base +  8,
            Self::Photography => base +  9,
            Self::Pottery     => base + 10,
            Self::Sculpting   => base + 11,
            Self::Writing     => base + 12,
            Self::Custom      => base + 13, // Custom(u8)のidリスト格納スロット
        }
    }

    pub fn read(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Acting,      Lang::En(_)) => "Acting",
            (Self::Acting,      Lang::Ja)    => "演劇",
            (Self::Barber,      Lang::En(_)) => "Barber",
            (Self::Barber,      Lang::Ja)    => "理容",
            (Self::Calligraphy, Lang::En(_)) => "Calligraphy",
            (Self::Calligraphy, Lang::Ja)    => "書道",
            (Self::Carpentry,   Lang::En(_)) => "Carpentry",
            (Self::Carpentry,   Lang::Ja)    => "大工仕事",
            (Self::Cook,        Lang::En(_)) => "Cook",
            (Self::Cook,        Lang::Ja)    => "料理",
            (Self::Dancing,     Lang::En(_)) => "Dancing",
            (Self::Dancing,     Lang::Ja)    => "ダンス",
            (Self::FineArt,     Lang::En(_)) => "Fine Art",
            (Self::FineArt,     Lang::Ja)    => "絵画",
            (Self::Forgery,     Lang::En(_)) => "Forgery",
            (Self::Forgery,     Lang::Ja)    => "文書偽造",
            (Self::Photography, Lang::En(_)) => "Photography",
            (Self::Photography, Lang::Ja)    => "写真術",
            (Self::Pottery,     Lang::En(_)) => "Pottery",
            (Self::Pottery,     Lang::Ja)    => "陶芸",
            (Self::Sculpting,   Lang::En(_)) => "Sculpting",
            (Self::Sculpting,   Lang::Ja)    => "彫刻",
            (Self::Writing,     Lang::En(_)) => "Writing",
            (Self::Writing,     Lang::Ja)    => "執筆",
        }
    }

    /// カスタム専門分野を新規登録し、そのschema_idを返す。
    /// Custom(0) スロットにidリスト（u32 LE配列）を保持し、末尾に追記する。
    pub fn write_custom<'a>(&self, character: &'a mut DataStruct, value: &[u8]) -> Option<u32> {
        let mut ids: Vec<u32> = character.get(Self::CUSTOM_LIST_ID).ok()
            .map(|b| b.chunks_exact(4)
                .filter_map(|c| c.try_into().ok().map(u32::from_le_bytes))
                .collect())
            .unwrap_or_default();

        let builtin_max: u32 = 12;
        let list_id: u32     = Self::CUSTOM_LIST_ID;
        let existing_max = ids.iter().copied().max().unwrap_or(list_id);
        let new_id = existing_max.max(builtin_max).max(list_id) + 1;

        ids.push(new_id);
        let list_bytes: Vec<u8> = ids.iter().flat_map(|id| id.to_le_bytes()).collect();
        character.set(Self::CUSTOM_LIST_ID, &list_bytes, None).ok()?;
        character.set(new_id, value, None).ok()?;
        Some(new_id)
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

pub trait ArtAndCraftTrait<const A: ArtAndCraft> {
    const VARIANT: ArtAndCraft = A;
    const SKILL:   Skill       = Skill::ArtAndCraft;
    const NAME:    'static &str = SKILL.name();
    const BASE:    u7           = SKILL.base();

    const OCCUPATION_POINTS: Field = Field {position: 32, mask: (1 <<  9) - 1}
    const INTEREST_POINTS:   Field = Field {position: 23, mask: (1 <<  9) - 1}
    const CHANGE:            Field = Field {position: 13, mask: (1 << 10) - 1}
    const MODIFIER:          Field = Field {position:  3, mask: (1 << 10) - 1}

    // Custom以外はconst解決、Custom(i)はcharacterを参照
    fn id(&self, character: &DataStruct) -> u32 { A.id(character) }

    // -> occupation_points, interest_points, change, modifier
    fn read(&self, character: &DataStruct) -> (u9, u9, i10, i10) {
        bytes = character.get(self.id(character));
    }

    fn write<'a>(&self, character: &'a mut DataStruct, occupation_points: u9, interest_points: u9, change: i10, modifier: i10) -> &'a mut DataStruct {
        _ = character.set(self.id(character), value: [u8; 5], None);
        character
    }

    // -> name, specialization
    fn display_string(&self, lang: Lang) -> (String, String) { (Self::NAME, A.read(lang).to_string()) }

    // -> base, occupation_points, interest_points, change, modifier, sum
    fn display_numeric(&self, occupation_points: u9, interest_points: u9, change: i10, modifier: i10) -> (u7, u9, u9, i10, i10, i10) {
        Self::BASE,
        occupation_points,
        interest_points,
        change,
        modifier,
        sum = Self::BASE + occupation_points + interest_points + change + modifier,
    }
}

impl ArtAndCraftTrait<{ ArtAndCraft::Custom }> for Custom {
    fn id(&self, character: &DataStruct) -> u32 {
        let offset = Skill::ArtAndCraft.id();
        let list_id = offset + 13;
        if self.0 == 0 {
            return list_id;
        }
        let bytes = character.get(list_id).ok()?;
        let idx = (self.0 as usize).checked_sub(1)?;
        bytes.get(idx * 4..idx * 4 + 4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
    }

    fn display_string(&self, character: &DataStruct, lang: Lang) -> (String, String) {
        let name = Self::NAME.to_string();
        let spec = character.get(self.id(character)).unwrap_or_default();
        (name, spec)
    }
}

pub struct Custom(pub u8);

/// 近接戦闘 (専門分野) Fighting (Specialization) // p.61
#[derive(Clone)]
pub enum Fighting {
    Axe,        // 斧         15%
    Brawl,      // 格闘       25%
    Chainsaw,   // チェーンソー 10%
    Flail,      // フレイル    10%
    Garrote,    // 絞殺ひも    15%
    Spear,      // 槍         20%
    Sword,      // 刀剣       20%
    Whip,       // 鞭         05%
    Custom(u8),
}

impl Fighting {

    pub fn list() -> &'static [Self] {
        &[
            Self::Axe, Self::Brawl, Self::Chainsaw, Self::Flail, Self::Garrote, Self::Spear, Self::Sword, Self::Whip, Self::Custom(_)
        ]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::Axe            => 1,
            Self::Brawl          => 2,
            Self::Chainsaw       => 3,
            Self::Flail          => 4,
            Self::Garrote        => 5,
            Self::Spear          => 6,
            Self::Sword          => 7,
            Self::Whip           => 8,
            Self::Custom(0)      => 9,
            Self::Custom(i)      => ,
        }
    }

    // (spec_name: &str, base_value: u16, occupation_point: u8, interest_point: u8, modifier: i8)
    pub fn read(&self, character: &DataStruct) -> (u16) { 
        match self {
            Self::Axe       => 15,
            Self::Brawl     => 25,
            Self::Chainsaw  => 10,
            Self::Flail     => 10,
            Self::Garrote   => 15,
            Self::Spear     => 20,
            Self::Sword     => 20,
            Self::Whip      =>  5,
            Self::Custom(_) => ,
        }
    }

    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Axe,      Lang::En(_)) => "Axe",
            (Self::Axe,      Lang::Ja) => "斧",
            (Self::Brawl,    Lang::En(_)) => "Brawl",
            (Self::Brawl,    Lang::Ja) => "格闘",
            (Self::Chainsaw, Lang::En(_)) => "Chainsaw",
            (Self::Chainsaw, Lang::Ja) => "チェーンソー",
            (Self::Flail,    Lang::En(_)) => "Flail",
            (Self::Flail,    Lang::Ja) => "フレイル",
            (Self::Garrote,  Lang::En(_)) => "Garrote",
            (Self::Garrote,  Lang::Ja) => "絞殺ひも",
            (Self::Spear,    Lang::En(_)) => "Spear",
            (Self::Spear,    Lang::Ja) => "槍",
            (Self::Sword,    Lang::En(_)) => "Sword",
            (Self::Sword,    Lang::Ja) => "刀剣",
            (Self::Whip,     Lang::En(_)) => "Whip",
            (Self::Whip,     Lang::Ja) => "鞭",
            (Self::Custom(0), _) => CharacterError::Skill::Fighting("Custom(0) is not to label()")
            (Self::Custom(i), _) => ,
        }
    }
}

/// 射撃 (専門分野) Firearms (Specialization) // p.64
#[derive(Clone)]
pub enum Firearms {
    Bow,           // 弓, 15%
    FlameThrower,  // 火炎放射器, 10%
    Handgun,       // 拳銃, 20%
    HeavyWeapons,  // 重火器, 10%
    MachineGun,    // マシンガン, 10%
    RifleShotgun,  // ライフル/ショットガン, 25%
    SubmachineGun, // サブマシンガン, 15%
    Custom(u8),
}

impl Firearms {

    pub fn list() -> &'static [Self] {
        &[Self::Bow, Self::FlameThrower, Self::Handgun, Self::HeavyWeapons, Self::MachineGun, Self::RifleShotgun, Self::SubmachineGun]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::Bow            => 0,
            Self::Handgun        => 1,
            Self::HeavyWeapons   => 2,
            Self::MachineGun     => 3,
            Self::RifleShotgun   => 4,
            Self::SubmachineGun  => 5,
            Self::Custom(0) => 6,
            Self::Custom(i) => ,
        }
    }

    pub fn base_value(&self) -> u16 {
        match self {
            Self::None                              =>  0,
            Self::Bow                               => 15,
            Self::Handgun                           => 20,
            Self::HeavyWeapons                      => 10,
            Self::MachineGun                        => 10,
            Self::RifleShotgun                      => 25,
            Self::SubmachineGun                     => 15,
            Self::Custom(0) => CharacterError::Skill::Firearms("Custom(0) is not to read()")
            Self::Custom(i)      => ,
        }
    }

    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Bow,           Lang::En(_)) => "Bow",
            (Self::Bow,           Lang::Ja) => "弓",
            (Self::Handgun,       Lang::En(_)) => "Handgun",
            (Self::Handgun,       Lang::Ja) => "拳銃",
            (Self::HeavyWeapons,  Lang::En(_)) => "Heavy Weapons",
            (Self::HeavyWeapons,  Lang::Ja) => "重火器",
            (Self::MachineGun,    Lang::En(_)) => "Machine Gun",
            (Self::MachineGun,    Lang::Ja) => "マシンガン",
            (Self::RifleShotgun,  Lang::En(_)) => "Rifle/Shotgun",
            (Self::RifleShotgun,  Lang::Ja) => "ライフル/ショットガン",
            (Self::SubmachineGun, Lang::En(_)) => "Submachine Gun",
            (Self::SubmachineGun, Lang::Ja) => "サブマシンガン",
            (Self::Custom(0), _) => CharacterError::Skill::Firearms::Custom("Custom(0) is not to read()")
            (Self::Custom(i), _) => ,
        }
    }
}

/// ほかの言語 (専門分野) (Language (Other) (Specialization) // p.73
#[derive(Clone)]
pub enum Language {
    Custom(u8),
}

impl Language {
    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::Custom(0) => 0,
            Self::Custom(i) => ,
        }
    }

    pub fn label(&self, _lang: Lang) -> &str {
        match self {
            Self::Custom(0) => ,
            Self::Custom(i) => ,
        }
    }
}

/// 操縦 (専門分野) Pilot (Specialization) // p.67
#[derive(Clone)]
pub enum Pilot {
    // --- 両時代共通 ---
    Boat,       // ボート
    SteamShip,  // 汽船
    Sailboat,   // 帆船
    CivilProp,  // 民間プロペラ機
    // --- 1920s のみ ---
    Balloon,    // 気球
    Dirigible,  // 飛行船
    // --- Modern (1990s) のみ ---
    CivilJet,   // 民間ジェット機
    Airliner,   // 定期旅客機
    JetFighter, // ジェット戦闘機
    Helicopter, // ヘリコプター
    Custom(u8),
}

impl Pilot {
    pub fn list() -> &'static [Self] {
        &[Self::Boat, Self::SteamShip, Self::Sailboat, Self::CivilProp,
          Self::Balloon, Self::Dirigible, Self::CivilJet, Self::Airliner,
          Self::JetFighter, Self::Helicopter]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::Boat       =>  0,
            Self::SteamShip  =>  1,
            Self::Sailboat   =>  2,
            Self::CivilProp  =>  3,
            Self::Balloon    =>  4,
            Self::Dirigible  =>  5,
            Self::CivilJet   =>  6,
            Self::Airliner   =>  7,
            Self::JetFighter =>  8,
            Self::Helicopter =>  9,
            Self::Custom(0) => 10,
            Self::Custom(i) => ,
        }
    }

    pub fn base_value(&self) -> u16 { 1 }

    pub fn label(&self, lang: Lang) -> Option<&str> {
        match (self, lang) {
            // --- 両時代共通 ---
            (Self::Boat,       Lang::Ja) => "ボート",
            (Self::Boat,       Lang::En(_)) => "Boat",
            (Self::SteamShip,  Lang::Ja) => "汽船",
            (Self::SteamShip,  Lang::En(_)) => "Steam Ship",
            (Self::Sailboat,   Lang::Ja) => "帆船",
            (Self::Sailboat,   Lang::En(_)) => "Sailboat",
            (Self::CivilProp,  Lang::Ja) => "民間プロペラ機",
            (Self::CivilProp,  Lang::En(_)) => "Civil Prop",
            // --- 1920s のみ ---
            (Self::Balloon,    Lang::Ja) => "気球",
            (Self::Balloon,    Lang::En(_)) => "Balloon",
            (Self::Dirigible,  Lang::Ja) => "飛行船",
            (Self::Dirigible,  Lang::En(_)) => "Dirigible",
            // --- Modern (1990s) のみ ---
            (Self::CivilJet,   Lang::Ja) => "民間ジェット機",
            (Self::CivilJet,   Lang::En(_)) => "Civil Jet",
            (Self::Airliner,   Lang::Ja) => "旅客機",
            (Self::Airliner,   Lang::En(_)) => "Airliner",
            (Self::JetFighter, Lang::Ja) => "ジェット戦闘機",
            (Self::JetFighter, Lang::En(_)) => "Jet Fighter",
            (Self::Helicopter, Lang::Ja) => "ヘリコプター",
            (Self::Helicopter, Lang::En(_)) => "Helicopter",
            (Self::Custom(0), _) => ,
            (Self::Custom(i), _) => ,
        }
    }
}

/// 科学 (専門分野) Science (Specialization) // p.59
#[derive(Clone)]
pub enum Science {
    None,
    Astronomy,    // 天文学
    Biology,      // 生物学
    Botany,       // 植物学
    Chemistry,    // 化学
    Cryptography, // 暗号学
    Engineering,  // 工学
    Forensics,    // 法医学
    Geology,      // 地質学
    Mathematics,  // 数学
    Meteorology,  // 気象学
    Pharmacy,     // 薬学
    Physics,      // 物理学
    Zoology,      // 動物学
    Custom(u8),
}


impl Science {
    pub fn list() -> &'static [Self] {
        &[Self::Astronomy, Self::Biology, Self::Botany, Self::Chemistry,
          Self::Cryptography, Self::Engineering, Self::Forensics, Self::Geology,
          Self::Mathematics, Self::Meteorology, Self::Pharmacy, Self::Physics,
          Self::Zoology]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::None         => unreachable!(),
            Self::Astronomy    =>  0,
            Self::Biology      =>  1,
            Self::Botany       =>  2,
            Self::Chemistry    =>  3,
            Self::Cryptography =>  4,
            Self::Engineering  =>  5,
            Self::Forensics    =>  6,
            Self::Geology      =>  7,
            Self::Mathematics  =>  8,
            Self::Meteorology  =>  9,
            Self::Pharmacy     => 10,
            Self::Physics      => 11,
            Self::Zoology      => 12,
            Self::Custom(0)   => 13,
            Self::Custom(i)   => 14,
            Self::Custom3(_)   => 15,
            Self::Custom4(_)   => 16,
        }
    }

    pub fn base_value(&self) -> u16 { 1 }

    pub fn label(&self, lang: Lang) -> Option<&str> {
        match (self, lang) {
            (Self::None,         _)        => None,
            (Self::Astronomy,    Lang::Ja) => "天文学"),
            (Self::Astronomy,    Lang::En(_)) => "Astronomy"),
            (Self::Biology,      Lang::Ja) => "生物学"),
            (Self::Biology,      Lang::En(_)) => "Biology"),
            (Self::Botany,       Lang::Ja) => "植物学"),
            (Self::Botany,       Lang::En(_)) => "Botany"),
            (Self::Chemistry,    Lang::Ja) => "化学"),
            (Self::Chemistry,    Lang::En(_)) => "Chemistry"),
            (Self::Cryptography, Lang::Ja) => "暗号学"),
            (Self::Cryptography, Lang::En(_)) => "Cryptography"),
            (Self::Engineering,  Lang::Ja) => "工学"),
            (Self::Engineering,  Lang::En(_)) => "Engineering"),
            (Self::Forensics,    Lang::Ja) => "法医学"),
            (Self::Forensics,    Lang::En(_)) => "Forensics"),
            (Self::Geology,      Lang::Ja) => "地質学"),
            (Self::Geology,      Lang::En(_)) => "Geology"),
            (Self::Mathematics,  Lang::Ja) => "数学"),
            (Self::Mathematics,  Lang::En(_)) => "Mathematics"),
            (Self::Meteorology,  Lang::Ja) => "気象学"),
            (Self::Meteorology,  Lang::En(_)) => "Meteorology"),
            (Self::Pharmacy,     Lang::Ja) => "薬学"),
            (Self::Pharmacy,     Lang::En(_)) => "Pharmacy"),
            (Self::Physics,      Lang::Ja) => "物理学"),
            (Self::Physics,      Lang::En(_)) => "Physics"),
            (Self::Zoology,      Lang::Ja) => "動物学"),
            (Self::Zoology,      Lang::En(_)) => "Zoology"),
            (Self::Custom(0) | Self::Custom(i), _) => s.as_str()),
        }
    }
}

// --- サバイバル 専門分野 (Survival Specialization) --- p.63
#[derive(Clone)]
pub enum Survival {
    Arctic,
    Desert,
    Sea,
    Custom(u8),
}

impl Survival {
    pub fn list() -> &'static [Self] {
        &[Self::Arctic, Self::Desert, Self::Sea]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::None       => unreachable!(),
            Self::Arctic     => 0,
            Self::Desert     => 1,
            Self::Sea        => 2,
            Self::Custom1(_) => 3,
            Self::Custom2(_) => 4,
            Self::Custom3(_) => 5,
            Self::Custom4(_) => 6,
        }
    }

    pub fn base_value(&self) -> u16 { 10 }

    pub fn label(&self, lang: Lang) -> Option<&str> {
        match (self, lang) {
            (Self::None,     _)        => None,
            (Self::Arctic,   Lang::Ja) => "極地"),
            (Self::Arctic,   Lang::En(_)) => "Arctic"),
            (Self::Desert,   Lang::Ja) => "砂漠"),
            (Self::Desert,   Lang::En(_)) => "Desert"),
            (Self::Sea,      Lang::Ja) => "海"),
            (Self::Sea,      Lang::En(_)) => "Sea"),
            (Self::Custom1(s) | Self::Custom2(s)
            | Self::Custom3(s) | Self::Custom4(s), _) => s.as_str()),
        }
    }
}

// ============================================================
// --- 装備 (Equipment) ---
// ============================================================

pub enum Weapon {
    // --- 近接・投擲武器 ---
    BowAndArrows,           // Bow and Arrows      1D6+half DB      (貫通)
    BrassKnuckles,          // Brass Knuckles      1D3+1+DB
    Bullwhip,               // Bullwhip            1D3+half DB
    BurningTorch,           // Burning Torch       1D6+burn
    Blackjack,              // Blackjack           1D8+DB
    ClubLarge,              // Club, Large         1D8+DB
    ClubSmall,              // Club, Small         1D6+DB
    Crossbow,               // Crossbow            1D8+2            (貫通)
    Garrote,                // Garrote             1D6+DB           (貫通)
    HatchetSickle,          // Hatchet/Sickle      1D6+1+DB         (貫通)
    KnifeLarge,             // Knife, Large        1D8+DB           (貫通)
    KnifeMedium,            // Knife, Medium       1D4+2+DB         (貫通)
    KnifeSmall,             // Knife, Small        1D4+DB           (貫通)
    Nunchaku,               // Nunchaku            1D8+DB
    RockThrown,             // Rock, Thrown        1D4+half DB
    Shuriken,               // Shuriken            1D3+half DB      (貫通)
    Spear,                  // Spear               1D8+1            (貫通)
    SpearThrown,            // Spear, Thrown       1D8+half DB      (貫通)
    // todo!(チェーンソー、マセスプレー、スタンガン、刀剣類、戦闘用ブーメラン、木斧)
    // --- 拳銃 (Handguns) ---
    Auto22Short,            // .22 Short Automatic 1D6
    Derringer25,            // .25 Derringer       1D6
    Revolver32,             // .32 Revolver        1D8
    Automatic32,            // .32 Automatic       1D8
    LugerP08,               // Model P08 Luger     1D10
    Revolver45,             // .45 Revolver        1D10+2
    Automatic45,            // .45 Automatic       1D10+2
    // --- ライフル (Rifles) ---
    BoltAction22,           // .22 Bolt-Action     1D6+1
    LeverAction30,          // .30 Lever-Action    2D6
    MartiniHenry45,         // .45 Martini-Henry   1D8+1D6+3
    MoranAirRifle,          // Col. Moran's Air    2D6+1
    LeeEnfield303,          // .303 Lee-Enfield    2D6+4
    BoltAction3006,         // .30-06 Bolt-Action  2D6+4
    ElephantGun,            // Elephant Gun        3D6+4
    // --- ショットガン (Shotguns) ---
    Shotgun20Gauge,         // 20-gauge (2B)        2D6/1D6/1D3
    Shotgun16Gauge,         // 16-gauge (2B)        2D6+2/1D6+1/1D4
    Shotgun12Gauge,         // 12-gauge (2B)        4D6/2D6/1D6
    Shotgun12GaugeSemiAuto, // 12-gauge semi-auto   4D6/2D6/1D6
    Shotgun12GaugeSawedOff, // 12-gauge sawed off   4D6/1D6
    // --- 短機関銃 (SMG) ---
    BergmannMP18,           // Bergmann MP18        1D10
    Thompson,               // Thompson             1D10+2
    // --- 機関銃 (MG) ---
    BrowningAutoRifle,      // Browning Auto Rifle  2D6+4
    BrowningM1917,          // .30 Browning M1917   2D6+4
    BrenGun,                // Bren Gun             2D6+4
    LewisGun,               // Mark I Lewis Gun     2D6+4
    Vickers303,             // Vickers .303         2D6+4
    Custom(String),
}

impl Weapon {
    // pub fn label(&self, lang: Lang) -> 'static &str {
    //     match (self, lang) {
    //         (Self::Name,            Lang::En(_)) => "Weapon",
    //         (Self::Name,            Lang::Ja) => "武器",
    //         (Self::Regular,         Lang::En(_)) => "Regular",
    //         (Self::Regular,         Lang::Ja) => "レギュラー",
    //         (Self::Hard,            Lang::En(_)) => "Hard",
    //         (Self::Hard,            Lang::Ja) => "ハード",
    //         (Self::Extreme,         Lang::En(_)) => "Extreme",
    //         (Self::Extreme,         Lang::Ja) => "イクストリーム",
    //         (Self::Damage,          Lang::En(_)) => "Damage",
    //         (Self::Damage,          Lang::Ja) => "ダメージ",
    //         (Self::Range,           Lang::En(_)) => "Range",
    //         (Self::Range,           Lang::Ja) => "射程",
    //         (Self::AttacksPerRound, Lang::En(_)) => "Attacks",
    //         (Self::AttacksPerRound, Lang::Ja) => "攻撃回数",
    //         (Self::Ammunition,      Lang::En(_)) => "Ammo",
    //         (Self::Ammunition,      Lang::Ja) => "装弾数",
    //         (Self::Malfunction,     Lang::En(_)) => "Malfunction",
    //         (Self::Malfunction,     Lang::Ja) => "故障",
    //     }
    // }

    pub fn display(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::BowAndArrows,           Lang::En(_)) => "Bow and Arrows",
            (Self::BowAndArrows,           Lang::Ja) => "弓と矢",
            (Self::BrassKnuckles,          Lang::En(_)) => "Brass Knuckles",
            (Self::BrassKnuckles,          Lang::Ja) => "ブラスナックル",
            (Self::Bullwhip,               Lang::En(_)) => "Bullwhip",
            (Self::Bullwhip,               Lang::Ja) => "むち",
            (Self::BurningTorch,           Lang::En(_)) => "Burning Torch",
            (Self::BurningTorch,           Lang::Ja) => "燃えているたいまつ",
            (Self::Blackjack,              Lang::En(_)) => "Blackjack",
            (Self::Blackjack,              Lang::Ja) => "ブラックジャック",
            (Self::ClubLarge,              Lang::En(_)) => "Club, Large",
            (Self::ClubLarge,              Lang::Ja) => "大きい棍棒",
            (Self::ClubSmall,              Lang::En(_)) => "Club, Small",
            (Self::ClubSmall,              Lang::Ja) => "小さい棍棒",
            (Self::Crossbow,               Lang::En(_)) => "Crossbow",
            (Self::Crossbow,               Lang::Ja) => "クロスボウ",
            (Self::Garrote,                Lang::En(_)) => "Garrote",
            (Self::Garrote,                Lang::Ja) => "絞殺ひも",
            (Self::HatchetSickle,          Lang::En(_)) => "Hatchet/Sickle",
            (Self::HatchetSickle,          Lang::Ja) => "手斧/小鎌",
            (Self::KnifeLarge,             Lang::En(_)) => "Knife, Large",
            (Self::KnifeLarge,             Lang::Ja) => "大型ナイフ",
            (Self::KnifeMedium,            Lang::En(_)) => "Knife, Medium",
            (Self::KnifeMedium,            Lang::Ja) => "中型ナイフ",
            (Self::KnifeSmall,             Lang::En(_)) => "Knife, Small",
            (Self::KnifeSmall,             Lang::Ja) => "小型ナイフ",
            (Self::Nunchaku,               Lang::En(_)) => "Nunchaku",
            (Self::Nunchaku,               Lang::Ja) => "ヌンチャク",
            (Self::RockThrown,             Lang::En(_)) => "Rock, Thrown",
            (Self::RockThrown,             Lang::Ja) => "投石",
            (Self::Shuriken,               Lang::En(_)) => "Shuriken",
            (Self::Shuriken,               Lang::Ja) => "手裏剣",
            (Self::Spear,                  Lang::En(_)) => "Spear",
            (Self::Spear,                  Lang::Ja) => "騎兵槍",
            (Self::SpearThrown,            Lang::En(_)) => "Spear, Thrown",
            (Self::SpearThrown,            Lang::Ja) => "投げ槍",
            (Self::Auto22Short,            Lang::En(_)) => ".22 Short Automatic",
            (Self::Auto22Short,            Lang::Ja) => ".22ショートオートマチック",
            (Self::Derringer25,            Lang::En(_)) => ".25 Derringer",
            (Self::Derringer25,            Lang::Ja) => ".25デリンジャー",
            (Self::Revolver32,             Lang::En(_)) => ".32 Revolver",
            (Self::Revolver32,             Lang::Ja) => ".32リボルバー",
            (Self::Automatic32,            Lang::En(_)) => ".32 Automatic",
            (Self::Automatic32,            Lang::Ja) => ".32オートマチック",
            (Self::LugerP08,               Lang::En(_)) => "Model P08 Luger",
            (Self::LugerP08,               Lang::Ja) => "P08ルガー",
            (Self::Revolver45,             Lang::En(_)) => ".45 Revolver",
            (Self::Revolver45,             Lang::Ja) => ".45リボルバー",
            (Self::Automatic45,            Lang::En(_)) => ".45 Automatic",
            (Self::Automatic45,            Lang::Ja) => ".45オートマチック",
            (Self::BoltAction22,           Lang::En(_)) => ".22 Bolt-Action Rifle",
            (Self::BoltAction22,           Lang::Ja) => ".22ボルトアクションライフル",
            (Self::LeverAction30,          Lang::En(_)) => ".30 Lever-Action Carbine",
            (Self::LeverAction30,          Lang::Ja) => ".30レバーアクションカービン",
            (Self::MartiniHenry45,         Lang::En(_)) => ".45 Martini-Henry Rifle",
            (Self::MartiniHenry45,         Lang::Ja) => ".45マルティニ・ヘンリー",
            (Self::MoranAirRifle,          Lang::En(_)) => "Col. Moran's Air Rifle",
            (Self::MoranAirRifle,          Lang::Ja) => "モラン大佐の空気銃",
            (Self::LeeEnfield303,          Lang::En(_)) => ".303 Lee-Enfield",
            (Self::LeeEnfield303,          Lang::Ja) => ".303リー・エンフィールド",
            (Self::BoltAction3006,         Lang::En(_)) => ".30-06 Bolt-Action Rifle",
            (Self::BoltAction3006,         Lang::Ja) => ".30-06ボルトアクションライフル",
            (Self::ElephantGun,            Lang::En(_)) => "Elephant Gun",
            (Self::ElephantGun,            Lang::Ja) => "エレファントガン",
            (Self::Shotgun20Gauge,         Lang::En(_)) => "20-gauge Shotgun",
            (Self::Shotgun20Gauge,         Lang::Ja) => "20ゲージショットガン",
            (Self::Shotgun16Gauge,         Lang::En(_)) => "16-gauge Shotgun",
            (Self::Shotgun16Gauge,         Lang::Ja) => "16ゲージショットガン",
            (Self::Shotgun12Gauge,         Lang::En(_)) => "12-gauge Shotgun",
            (Self::Shotgun12Gauge,         Lang::Ja) => "12ゲージショットガン",
            (Self::Shotgun12GaugeSemiAuto, Lang::En(_)) => "12-gauge Shotgun (semi-auto)",
            (Self::Shotgun12GaugeSemiAuto, Lang::Ja) => "12ゲージショットガン(半自動)",
            (Self::Shotgun12GaugeSawedOff, Lang::En(_)) => "12-gauge Shotgun (sawed off)",
            (Self::Shotgun12GaugeSawedOff, Lang::Ja) => "12ゲージショットガン(短銃身)",
            (Self::BergmannMP18,           Lang::En(_)) => "Bergmann MP18",
            (Self::BergmannMP18,           Lang::Ja) => "ベルグマンMP18",
            (Self::Thompson,               Lang::En(_)) => "Thompson",
            (Self::Thompson,               Lang::Ja) => "トンプソン",
            (Self::BrowningAutoRifle,      Lang::En(_)) => "Browning Automatic Rifle M1918",
            (Self::BrowningAutoRifle,      Lang::Ja) => "ブローニング自動小銃M1918",
            (Self::BrowningM1917,          Lang::En(_)) => ".30 Browning M1917A1",
            (Self::BrowningM1917,          Lang::Ja) => ".30ブローニングM1917A1",
            (Self::BrenGun,                Lang::En(_)) => "Bren Gun",
            (Self::BrenGun,                Lang::Ja) => "ブレンガン",
            (Self::LewisGun,               Lang::En(_)) => "Mark I Lewis Gun",
            (Self::LewisGun,               Lang::Ja) => "ルイス軽機関銃Mk.I",
            (Self::Vickers303,             Lang::En(_)) => "Vickers .303 Machine Gun",
            (Self::Vickers303,             Lang::Ja) => "ヴィッカース.303機関銃",
            (Self::Custom(_),              _        ) => "Custom",
        }
    }

    pub fn skill(&self) -> Skill {
        match self {
            Self::BowAndArrows                          => Skill::Firearms(FirearmsSpec::Bow),
            Self::BrassKnuckles                         => Skill::Fighting(FightingSpec::Brawl),
            Self::Bullwhip                              => Skill::Fighting(FightingSpec::Whip),
            Self::BurningTorch                          => Skill::Fighting(FightingSpec::Brawl),
            Self::Blackjack                             => Skill::Fighting(FightingSpec::Brawl),
            Self::ClubLarge                             => Skill::Fighting(FightingSpec::Brawl),
            Self::ClubSmall                             => Skill::Fighting(FightingSpec::Brawl),
            Self::Crossbow                              => Skill::Firearms(FirearmsSpec::Bow),
            Self::Garrote                               => Skill::Fighting(FightingSpec::Garrote),
            Self::HatchetSickle                         => Skill::Fighting(FightingSpec::Axe),
            Self::KnifeLarge                            => Skill::Fighting(FightingSpec::Brawl),
            Self::KnifeMedium                           => Skill::Fighting(FightingSpec::Brawl),
            Self::KnifeSmall                            => Skill::Fighting(FightingSpec::Brawl),
            Self::Nunchaku                              => Skill::Fighting(FightingSpec::Flail),
            Self::RockThrown                            => Skill::Throw,
            Self::Shuriken                              => Skill::Throw,
            Self::Spear                                 => Skill::Fighting(FightingSpec::Spear),
            Self::SpearThrown                           => Skill::Throw,
            Self::Auto22Short                           => Skill::Firearms(FirearmsSpec::Handgun),
            Self::Derringer25                           => Skill::Firearms(FirearmsSpec::Handgun),
            Self::Revolver32                            => Skill::Firearms(FirearmsSpec::Handgun),
            Self::Automatic32                           => Skill::Firearms(FirearmsSpec::Handgun),
            Self::LugerP08                              => Skill::Firearms(FirearmsSpec::Handgun),
            Self::Revolver45                            => Skill::Firearms(FirearmsSpec::Handgun),
            Self::Automatic45                           => Skill::Firearms(FirearmsSpec::Handgun),
            Self::BoltAction22                          => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::LeverAction30                         => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::MartiniHenry45                        => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::MoranAirRifle                         => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::LeeEnfield303                         => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::BoltAction3006                        => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::ElephantGun                           => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::Shotgun20Gauge                        => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::Shotgun16Gauge                        => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::Shotgun12Gauge                        => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::Shotgun12GaugeSemiAuto                => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::Shotgun12GaugeSawedOff                => Skill::Firearms(FirearmsSpec::RifleShotgun),
            Self::BergmannMP18                          => Skill::Firearms(FirearmsSpec::SubmachineGun),
            Self::Thompson                              => Skill::Firearms(FirearmsSpec::SubmachineGun),
            Self::BrowningAutoRifle                     => Skill::Firearms(FirearmsSpec::MachineGun),
            Self::BrowningM1917                         => Skill::Firearms(FirearmsSpec::MachineGun),
            Self::BrenGun                               => Skill::Firearms(FirearmsSpec::MachineGun),
            Self::LewisGun                              => Skill::Firearms(FirearmsSpec::MachineGun),
            Self::Vickers303                            => Skill::Firearms(FirearmsSpec::MachineGun),
            Self::Custom(_)                             => Skill::Fighting(FightingSpec::Brawl),
        }
    }

    pub fn range(&self, _lang: Lang) -> (u8, &str) { // integer, unit
        todo!()
    }

    /// 基本ダメージ式。`(dice_terms, db_multiplier)` を返す。
    /// `db_multiplier`: 0=なし, 1=DB全量, 2=DB半分(端数切り捨て)。
    /// ダメージボーナスの実値は呼び出し側が `SecondaryAttribute::DamageBonus` から取得して加算する。
    /// `Custom` は固定式が不明なため `None` を返す。
    pub fn damage(&self) -> Option<(&'static [Dice], u8)> {
        match self {
            Self::BowAndArrows          => Some((&[(1,6,0)],              2)),
            Self::BrassKnuckles         => Some((&[(1,3,1)],              1)),
            Self::Bullwhip              => Some((&[(1,3,0)],              2)),
            Self::BurningTorch          => Some((&[(1,6,0)],              0)), // +burn は別途処理
            Self::Blackjack             => Some((&[(1,8,0)],              1)),
            Self::ClubLarge             => Some((&[(1,8,0)],              1)),
            Self::ClubSmall             => Some((&[(1,6,0)],              1)),
            Self::Crossbow              => Some((&[(1,8,2)],              0)),
            Self::Garrote               => Some((&[(1,6,0)],              1)),
            Self::HatchetSickle         => Some((&[(1,6,1)],              1)),
            Self::KnifeLarge            => Some((&[(1,8,0)],              1)),
            Self::KnifeMedium           => Some((&[(1,4,2)],              1)),
            Self::KnifeSmall            => Some((&[(1,4,0)],              1)),
            Self::Nunchaku              => Some((&[(1,8,0)],              1)),
            Self::RockThrown            => Some((&[(1,4,0)],              2)),
            Self::Shuriken              => Some((&[(1,3,0)],              2)),
            Self::Spear                 => Some((&[(1,8,1)],              0)),
            Self::SpearThrown           => Some((&[(1,8,0)],              2)),
            Self::Auto22Short           => Some((&[(1,6,0)],              0)),
            Self::Derringer25           => Some((&[(1,6,0)],              0)),
            Self::Revolver32            => Some((&[(1,8,0)],              0)),
            Self::Automatic32           => Some((&[(1,8,0)],              0)),
            Self::LugerP08              => Some((&[(1,10,0)],             0)),
            Self::Revolver45            => Some((&[(1,10,2)],             0)),
            Self::Automatic45           => Some((&[(1,10,2)],             0)),
            Self::BoltAction22          => Some((&[(1,6,1)],              0)),
            Self::LeverAction30         => Some((&[(2,6,0)],              0)),
            Self::MartiniHenry45        => Some((&[(1,8,0),(1,6,3)],      0)),
            Self::MoranAirRifle         => Some((&[(2,6,1)],              0)),
            Self::LeeEnfield303         => Some((&[(2,6,4)],              0)),
            Self::BoltAction3006        => Some((&[(2,6,4)],              0)),
            Self::ElephantGun           => Some((&[(3,6,4)],              0)),
            Self::Shotgun20Gauge        => Some((&[(2,6,0)],              0)), // /1D6/1D3 距離段階別
            Self::Shotgun16Gauge        => Some((&[(2,6,2)],              0)),
            Self::Shotgun12Gauge        => Some((&[(4,6,0)],              0)),
            Self::Shotgun12GaugeSemiAuto => Some((&[(4,6,0)],             0)),
            Self::Shotgun12GaugeSawedOff => Some((&[(4,6,0)],             0)),
            Self::BergmannMP18          => Some((&[(1,10,0)],             0)),
            Self::Thompson              => Some((&[(1,10,2)],             0)),
            Self::BrowningAutoRifle     => Some((&[(2,6,4)],              0)),
            Self::BrowningM1917         => Some((&[(2,6,4)],              0)),
            Self::BrenGun               => Some((&[(2,6,4)],              0)),
            Self::LewisGun              => Some((&[(2,6,4)],              0)),
            Self::Vickers303            => Some((&[(2,6,4)],              0)),
            Self::Custom(_)             => None,
        }
    }

    pub fn is_impalable(&self) -> bool {
        match self {
            Self::BowAndArrows   => true,
            Self::Crossbow       => true,
            Self::Garrote        => true,
            Self::HatchetSickle  => true,
            Self::KnifeLarge     => true,
            Self::KnifeMedium    => true,
            Self::KnifeSmall     => true,
            Self::Shuriken       => true,
            Self::Spear          => true,
            Self::SpearThrown    => true,
            Self::Auto22Short           => true,
            Self::Derringer25           => true,
            Self::Revolver32            => true,
            Self::Automatic32           => true,
            Self::LugerP08              => true,
            Self::Revolver45            => true,
            Self::Automatic45           => true,
            Self::BoltAction22          => true,
            Self::LeverAction30         => true,
            Self::MartiniHenry45        => true,
            Self::MoranAirRifle         => true,
            Self::LeeEnfield303         => true,
            Self::BoltAction3006        => true,
            Self::ElephantGun           => true,
            Self::BergmannMP18          => true,
            Self::Thompson              => true,
            Self::BrowningAutoRifle     => true,
            Self::BrowningM1917         => true,
            Self::BrenGun               => true,
            Self::LewisGun              => true,
            Self::Vickers303            => true,
            _ => false,
        }
    }

    /// ラウンドあたり攻撃回数。銃器の括弧内は速射(quick draw)
    pub fn attacks_per_round(&self) -> u8 {
        match self {
            Self::BowAndArrows           => 1,
            Self::BrassKnuckles          => 1,
            Self::Bullwhip               => 1,
            Self::BurningTorch           => 1,
            Self::Blackjack              => 1,
            Self::ClubLarge              => 1,
            Self::ClubSmall              => 1,
            Self::Crossbow               => 1, // 実際は1/2ラウンド
            Self::Garrote                => 1,
            Self::HatchetSickle          => 1,
            Self::KnifeLarge             => 1,
            Self::KnifeMedium            => 1,
            Self::KnifeSmall             => 1,
            Self::Nunchaku               => 1,
            Self::RockThrown             => 1,
            Self::Shuriken               => 2,
            Self::Spear                  => 1,
            Self::SpearThrown            => 1,
            Self::Auto22Short            => 1,
            Self::Derringer25            => 1,
            Self::Revolver32             => 1,
            Self::Automatic32            => 1,
            Self::LugerP08               => 1,
            Self::Revolver45             => 1,
            Self::Automatic45            => 1,
            Self::BoltAction22           => 1,
            Self::LeverAction30          => 1,
            Self::MartiniHenry45         => 1,
            Self::MoranAirRifle          => 1,
            Self::LeeEnfield303          => 1,
            Self::BoltAction3006         => 1,
            Self::ElephantGun            => 1,
            Self::Shotgun20Gauge         => 1,
            Self::Shotgun16Gauge         => 1,
            Self::Shotgun12Gauge         => 1,
            Self::Shotgun12GaugeSemiAuto => 1,
            Self::Shotgun12GaugeSawedOff => 1,
            Self::BergmannMP18           => 1,
            Self::Thompson               => 1,
            Self::BrowningAutoRifle      => 1,
            Self::BrowningM1917          => 1, // フルオート
            Self::BrenGun                => 1,
            Self::LewisGun               => 1, // フルオート
            Self::Vickers303             => 1, // フルオート
            Self::Custom(_)              => 1,
        }
    }

    /// 装填数 (magazine)。近接武器は None
    pub fn ammunition(&self) -> Option<u8> {
        match self {
            Self::BowAndArrows           => Some(1),
            Self::Crossbow               => Some(1),
            Self::Shuriken               => Some(1), // one use
            Self::Auto22Short            => Some(6),
            Self::Derringer25            => Some(1),
            Self::Revolver32             => Some(6),
            Self::Automatic32            => Some(8),
            Self::LugerP08               => Some(8),
            Self::Revolver45             => Some(6),
            Self::Automatic45            => Some(7),
            Self::BoltAction22           => Some(6),
            Self::LeverAction30          => Some(6),
            Self::MartiniHenry45         => Some(1),
            Self::MoranAirRifle          => Some(1),
            Self::LeeEnfield303          => Some(10),
            Self::BoltAction3006         => Some(5),
            Self::ElephantGun            => Some(2),
            Self::Shotgun20Gauge         => Some(2),
            Self::Shotgun16Gauge         => Some(2),
            Self::Shotgun12Gauge         => Some(2),
            Self::Shotgun12GaugeSemiAuto => Some(5),
            Self::Shotgun12GaugeSawedOff => Some(2),
            Self::BergmannMP18           => Some(32), // 20/30/32
            Self::Thompson               => Some(30), // 20/30/50
            Self::BrowningAutoRifle      => Some(20),
            Self::BrowningM1917          => Some(250),
            Self::BrenGun                => Some(30), // 30/100
            Self::LewisGun               => Some(47), // 47/97
            Self::Vickers303             => Some(250),
            _ => None,
        }
    }

    /// 故障値 (malfunction number)。故障なしは None
    pub fn malfunction(&self) -> Option<u8> {
        match self {
            Self::BowAndArrows           => Some(97),
            Self::Crossbow               => Some(96),
            Self::Shuriken               => Some(100),
            Self::Auto22Short            => Some(100),
            Self::Derringer25            => Some(100),
            Self::Revolver32             => Some(100),
            Self::Automatic32            => Some(99),
            Self::LugerP08               => Some(99),
            Self::Revolver45             => Some(100),
            Self::Automatic45            => Some(100),
            Self::BoltAction22           => Some(99),
            Self::LeverAction30          => Some(98),
            Self::MartiniHenry45         => Some(100),
            Self::MoranAirRifle          => Some(88),
            Self::LeeEnfield303          => Some(100),
            Self::BoltAction3006         => Some(100),
            Self::ElephantGun            => Some(100),
            Self::Shotgun20Gauge         => Some(100),
            Self::Shotgun16Gauge         => Some(100),
            Self::Shotgun12Gauge         => Some(100),
            Self::Shotgun12GaugeSemiAuto => Some(100),
            Self::Shotgun12GaugeSawedOff => Some(100),
            Self::BergmannMP18           => Some(96),
            Self::Thompson               => Some(96),
            Self::BrowningAutoRifle      => Some(100),
            Self::BrowningM1917          => Some(96),
            Self::BrenGun                => Some(96),
            Self::LewisGun               => Some(96),
            Self::Vickers303             => None, // N/A
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
    Custom(String),     // 自由記述（装甲点は別途入力）
}

impl Armor {
    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::ThickLeatherJacket, Lang::En(_)) => "Thick Leather Jacket",
            (Self::ThickLeatherJacket, Lang::Ja) => "厚い皮のジャケット",
            (Self::WwiHelmet,          Lang::En(_)) => "WWI Helmet",
            (Self::WwiHelmet,          Lang::Ja) => "第一次大戦型のヘルメット",
            (Self::Hardwood1In,        Lang::En(_)) => "1\" Hardwood",
            (Self::Hardwood1In,        Lang::Ja) => "3cmの堅い木",
            (Self::PresentUsHelmet,    Lang::En(_)) => "Present U.S. Helmet",
            (Self::PresentUsHelmet,    Lang::Ja) => "現代アメリカ軍のヘルメット",
            (Self::HeavyKevlarVest,    Lang::En(_)) => "Heavy Kevlar Vest",
            (Self::HeavyKevlarVest,    Lang::Ja) => "厚いケブラー製のベスト",
            (Self::MilitaryBodyArmor,  Lang::En(_)) => "Military Body Armor",
            (Self::MilitaryBodyArmor,  Lang::Ja) => "軍用ボディ・アーマー",
            (Self::BulletproofGlass,   Lang::En(_)) => "1.5\" Bulletproof Glass",
            (Self::BulletproofGlass,   Lang::Ja) => "4cmの防弾ガラス",
            (Self::SteelPlate1In,      Lang::En(_)) => "1\" Steel Plate",
            (Self::SteelPlate1In,      Lang::Ja) => "5cmの鋼鉄板",
            (Self::LargeSandbag,       Lang::En(_)) => "Large Sandbag",
            (Self::LargeSandbag,       Lang::Ja) => "大きなサンドバッグ",
            (Self::Custom(s),          _)        => s.as_str(),
        }
    }

    pub fn points(&self) -> Option<u8> {
        match self {
            Self::ThickLeatherJacket => Some(1),
            Self::WwiHelmet          => Some(2),
            Self::Hardwood1In        => Some(3),
            Self::PresentUsHelmet    => Some(5),
            Self::HeavyKevlarVest    => Some(8),
            Self::MilitaryBodyArmor  => Some(12),
            Self::BulletproofGlass   => Some(15),
            Self::SteelPlate1In      => Some(19),
            Self::LargeSandbag       => Some(20),
            Self::Custom(_)          => None,
        }
    }
}

// --- 収入と財産 (Wealth) ---
pub struct Wealth {
    pub spending_level: StandardOfLiving,
    pub cash:           String,   // e.g. "$20"
    pub assets:         String,
}

impl Wealth {
    pub fn label_spending_level(lang: Lang) -> 'static &str {
        match lang { Lang::En(_) => "Spending Level", Lang::Ja => "支出レベル" }
    }
    pub fn label_cash(lang: Lang) -> 'static &str {
        match lang { Lang::En(_) => "Cash", Lang::Ja => "現金" }
    }
    pub fn label_assets(lang: Lang) -> 'static &str {
        match lang { Lang::En(_) => "Assets", Lang::Ja => "資産" }
    }
}

// --- 所持品カテゴリ (Possession) ---
pub enum Possession {
    Weapon(Weapon),
    Armor(Armor),
    GearItem(String),
    Wealth(Wealth),
}

impl Possession {
    pub fn id(&self) -> u32 {
        Character::Possession.id() + match self {
            Self::Weapon(_)   => 0,
            Self::Armor(_)    => 1,
            Self::GearItem(_) => 2,
            Self::Wealth(_)   => 3,
        }
    }

    pub fn label(&self, lang: Lang) -> 'static &str {
        match (self, lang) {
            (Self::Weapon(_),   Lang::En(_)) => "Weapon",
            (Self::Weapon(_),   Lang::Ja) => "武器",
            (Self::Armor(_),    Lang::En(_)) => "Armor",
            (Self::Armor(_),    Lang::Ja) => "装甲",
            (Self::GearItem(_), Lang::En(_)) => "Equipment",
            (Self::GearItem(_), Lang::Ja) => "装備",
            (Self::Wealth(_),   Lang::En(_)) => "Wealth",
            (Self::Wealth(_),   Lang::Ja) => "収入と財産",
        }
    }

    pub fn decode() {
        // name: &str
    }
}

// ============================================================
// --- バックストーリー (Backstory) ---
// ============================================================

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
    pub fn id(&self) -> u32 {
        Character::Backstory.id() + match self {
            Self::KeyConnection(_)              => 0,
            Self::PersonalDescription           => 1,
            Self::IdeologyAndBeliefs            => 2,
            Self::SignificantPeople             => 3,
            Self::MeaningfulLocation            => 4,
            Self::TreasuredPossession          => 5,
            Self::Trait                         => 6,
            Self::InjuresAndScars               => 7,
            // todo
            Self::PhobiasAndManias              => 8,
            Self::ArcaneTomesAndSpells          => 9,
            Self::EncountersWithStrangeEntities => 10,
        }
    }

    pub fn label(&self, lang: Lang) -> 'static &str {
        match (self, lang) {
            (Self::KeyConnection(_),              Lang::En(_)) => "Key Connection",
            (Self::KeyConnection(_),              Lang::Ja) => "キーコネクション",
            (Self::PersonalDescription,           Lang::En(_)) => "Personal Description",
            (Self::PersonalDescription,           Lang::Ja) => "容姿の描写",
            (Self::IdeologyAndBeliefs,            Lang::En(_)) => "Ideology & Beliefs",
            (Self::IdeologyAndBeliefs,            Lang::Ja) => "イデオロギー・信念", // p40 原文が"&"なので／から・に修正
            (Self::SignificantPeople,             Lang::En(_)) => "Significant People",
            (Self::SignificantPeople,             Lang::Ja) => "重要な人物",
            (Self::MeaningfulLocation,            Lang::En(_)) => "Meaningful Location",
            (Self::MeaningfulLocation,            Lang::Ja) => "意味のある場所",
            (Self::TreasuredPossession,           Lang::En(_)) => "Treasured Possession",
            (Self::TreasuredPossession,           Lang::Ja) => "秘蔵の品",
            (Self::Trait,                         Lang::En(_)) => "Trait",
            (Self::Trait,                         Lang::Ja) => "特徴",
            (Self::InjuresAndScars,               Lang::En(_)) => "Injuries & Scars",
            (Self::InjuresAndScars,               Lang::Ja) => "負傷、傷跡",
            (Self::PhobiasAndManias,              Lang::En(_)) => "Phobias & Manias",
            (Self::PhobiasAndManias,              Lang::Ja) => "恐怖症とマニア",
            (Self::ArcaneTomesAndSpells,          Lang::En(_)) => "Arcane Tomes & Spells",
            (Self::ArcaneTomesAndSpells,          Lang::Ja) => "魔道書、呪文、アーティファクト",
            (Self::EncountersWithStrangeEntities, Lang::En(_)) => "Encounters with Strange Entities",
            (Self::EncountersWithStrangeEntities, Lang::Ja) => "遭遇した超自然の存在",
        }
    }
}

// ============================================================
// --- メモ (Memo) ---
// ============================================================

/// メモスロット。slot: 0..MAX_MEMO_SLOTS-1
/// encode/decode のバイト列レイアウト:
///   [title_len: u32 LE][title: utf-8][body: utf-8]
/// label()  → title（表示名）
/// display() → body（本文）
pub struct Memo {
    pub slot: usize,
}

pub const MAX_MEMO_SLOTS: usize = 8;

impl Memo {
    /// slot 0..MAX_MEMO_SLOTS-1 の一覧を返す
    pub fn list() -> [Memo; MAX_MEMO_SLOTS] {
        from_fn(|slot| Memo { slot })
    }

    /// DataStruct のキーとなる ID。Character::Memo の const id を直接参照。
    pub fn id(&self) -> u32 {
        Character::Memo.id() + self.slot as u32
    }

    /// [title_len: u32 LE][title: utf-8][body: utf-8]
    pub fn encode(title: &str, body: &str) -> Vec<u8> {
        let t = title.as_bytes();
        let b = body.as_bytes();
        let mut out = Vec::with_capacity(4 + t.len() + b.len());
        out.extend_from_slice(&(t.len() as u32).to_le_bytes());
        out.extend_from_slice(t);
        out.extend_from_slice(b);
        out
    }

    /// → (title, body)
    pub fn decode(bytes: &[u8]) -> (String, String) {
        let title_len = bytes.get(0..4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0) as usize;
        let title_start = 4;
        let title = bytes.get(title_start..title_start + title_len)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let body = bytes.get(title_start + title_len..)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        (title, body)
    }

    /// title（表示名）を返す。bytes が空の場合は "Note N" / "メモ N"。
    pub fn label(&self, bytes: &[u8], lang: Lang) -> String {
        let (title, _) = Self::decode(bytes);
        if title.is_empty() {
            match lang {
                Lang::En(_) => format!("Note {}",  self.slot + 1),
                Lang::Ja => format!("メモ {}", self.slot + 1),
            }
        } else {
            title
        }
    }

    /// body（本文）を返す。
    pub fn display(bytes: &[u8]) -> String {
        let (_, body) = Self::decode(bytes);
        body
    }
}