use alloc::{
    borrow::ToOwned,
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{
    array::from_fn,
    primitive::{i8, i32, u8},
};

use arbitrary_int::{i10, u7, u9};

use crate::{
    Language,
    data_struct::DataStruct,
    field::{Fields, Layout},
    object::{Predicate, SubjectTrait},
};

pub const SCHEMA_SIZE: u32 = 256;

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

// === Character::{Profile, Characteristic, Secondary Attribute, Skill, Posession, Backstory, Memo} ===

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
    pub const fn base_id(&self) -> u32 {
        match self {
            Self::Profile => 10,
            Self::Characteristic => 20,
            Self::SecondaryAttribute => 30,
            Self::Skill => 40,
            Self::Possession => 90,
            Self::Backstory => 100,
            Self::Memo => 110,
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

    pub fn list() -> &'static [Character] {
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

// === Profile::{Name, Birthppalce, Pronoun, Occupation, Residence, Age} ===

#[derive(Clone, Copy)]
pub enum Profile {
    Name,
    Birthpalce,
    Pronoun,
    Occupation,
    Residence,
    Age,
}

impl SubjectTrait for Profile {
    const SCHEMA_SIZE: u32 = SCHEMA_SIZE;

    fn ids(&self) -> &'static [u32] {
        const BASE: u32 = Character::Profile.base_id();
        match self {
            Self::Name => &[BASE + 0, BASE + 1],
            Self::Birthpalce => &[BASE + 2],
            Self::Pronoun => &[BASE + 3],
            Self::Occupation => &[BASE + 4, BASE + 5, BASE + 6],
            Self::Residence => &[BASE + 7],
            Self::Age => &[BASE + 8],
        }
    }

    fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Name, Language::En(_)) => "Name",
            (Self::Name, Language::Ja) => "名前",
            (Self::Birthpalce, Language::En(_)) => "Birthplace",
            (Self::Birthpalce, Language::Ja) => "出身",
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

    fn list() -> &'static [Self] {
        &[Self::Name, Self::Birthpalce, Self::Pronoun, Self::Occupation, Self::Residence, Self::Age]
    }
}

pub struct Name;

impl Name {
    pub fn display(name: &String, complement: &Option<String>) -> String {
        match complement {
            Some(c) if !c.is_empty() => format!("{name} ({c})"),
            _ => name.clone(),
        }
    }
}

impl Predicate<2> for Name {
    type Parsed = (String, Option<String>); // name, complement
    type Subject = Profile;
    const VARIANT: Profile = Profile::Name;

    fn parse(bytes: [Option<&[u8]>; 2]) -> Self::Parsed {
        let name = bytes[0].map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default();
        let complement = bytes[1].map(|b| String::from_utf8_lossy(b).into_owned());
        (name, complement)
    }

    fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 2] {
        [Some(value.0.as_bytes().to_vec()), value.1.as_deref().map(|c| c.as_bytes().to_vec())]
    }
}

#[derive(Clone, Copy)]
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

impl Predicate<3> for Occupation {
    type Parsed = (OccupationKind, Option<String>, Option<String>); // kind, custom_name, title
    type Subject = Profile;
    const VARIANT: Profile = Profile::Occupation;

    fn parse(bytes: [Option<&[u8]>; 3]) -> Self::Parsed {
        let kind_id = bytes[0].and_then(|b| b.first().copied()).unwrap_or(0);
        let custom_name = bytes[1].map(|b| String::from_utf8_lossy(b).into_owned());
        let title = bytes[2].map(|b| String::from_utf8_lossy(b).into_owned());
        (OccupationKind::detect(kind_id), custom_name, title)
    }

    fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 3] {
        let (kind, custom_name, title) = value;
        [
            Some(vec![kind.id()]),
            custom_name.as_deref().map(|v| v.as_bytes().to_vec()),
            title.as_deref().map(|t| t.as_bytes().to_vec()),
        ]
    }
}

pub struct Birthplace; // Birthplace: str

impl Predicate<1> for Birthplace {
    type Parsed = String;
    type Subject = Profile;
    const VARIANT: Profile = Profile::Birthpalce;

    fn parse(bytes: [Option<&[u8]>; 1]) -> Self::Parsed {
        bytes[0].map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default()
    }

    fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 1] {
        [Some(value.as_bytes().to_vec())]
    }
}

pub struct Pronoun; // Pronoun: str

impl Predicate<1> for Pronoun {
    type Parsed = String;
    type Subject = Profile;
    const VARIANT: Profile = Profile::Pronoun;

    fn parse(bytes: [Option<&[u8]>; 1]) -> Self::Parsed {
        bytes[0].map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default()
    }

    fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 1] {
        [Some(value.as_bytes().to_vec())]
    }
}

pub struct Residence; // Residence: str

impl Predicate<1> for Residence {
    type Parsed = String;
    type Subject = Profile;
    const VARIANT: Profile = Profile::Residence;

    fn parse(bytes: [Option<&[u8]>; 1]) -> Self::Parsed {
        bytes[0].map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default()
    }

    fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 1] {
        [Some(value.as_bytes().to_vec())]
    }
}

pub struct Age; // Age: u16

impl Predicate<1> for Age {
    type Parsed = u16;
    type Subject = Profile;
    const VARIANT: Profile = Profile::Age;

    fn parse(bytes: [Option<&[u8]>; 1]) -> Self::Parsed {
        bytes[0].and_then(|b| b.get(0..2)?.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0)
    }

    fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 1] {
        [Some(value.to_le_bytes().to_vec())]
    }
}

// === Characteristics (Strength, Constitution, Size, Dexterity, Appearance, Intelligence, Power, Education) ===

#[derive(Clone, Copy)]
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

    pub const fn id(&self) -> u32 {
        Character::Characteristic.base_id() + *self as u32
    }

    pub fn from_id(id: u8) -> Option<Self> {
        let offset = id as u32 - Character::Characteristic.base_id();
        match offset {
            1 => Some(Self::Strength),
            2 => Some(Self::Constitution),
            3 => Some(Self::Size),
            4 => Some(Self::Dexterity),
            5 => Some(Self::Appearance),
            6 => Some(Self::Intelligence),
            7 => Some(Self::Power),
            8 => Some(Self::Education),
            _ => None,
        }
    }

    pub fn read(&self, character: &DataStruct) -> (u16, i16, i16) {
        character
            .get(self.id())
            .ok()
            .map(|b| {
                let initial = b
                    .get(0..2)
                    .and_then(|x| x.try_into().ok())
                    .map(u16::from_le_bytes)
                    .unwrap_or(0);
                let change = b
                    .get(2..4)
                    .and_then(|x| x.try_into().ok())
                    .map(i16::from_le_bytes)
                    .unwrap_or(0);
                let modifier = b
                    .get(4..6)
                    .and_then(|x| x.try_into().ok())
                    .map(i16::from_le_bytes)
                    .unwrap_or(0);
                (initial, change, modifier)
            })
            .unwrap_or((0, 0, 0))
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        value: (u16, i16, i16),
    ) -> &'a mut DataStruct {
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
        match self {
            Self::Size | Self::Intelligence | Self::Education => {
                dice::roll(&[(2, 6, 6)]) as u16 * 5
            }
            _ => dice::roll(&[(3, 6, 0)]) as u16 * 5,
        }
    }
}

// === Secondary Attributes ===

#[derive(Clone, Copy)]
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
    pub const fn base_id(&self) -> u32 {
        Character::SecondaryAttribute.base_id() + *self as u32
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

pub struct HitPoints; // HitPoints: u8 | CON, SIZ -> u8

impl HitPoints {
    pub fn id() -> u32 {
        SecondaryAttribute::HitPoints.base_id()
    }

    pub fn read(character: &DataStruct) -> u8 {
        character.get(Self::id()).ok().and_then(|b| b.first().copied()).unwrap_or(0)
    }

    pub fn write(character: &mut DataStruct, value: u8) -> &mut DataStruct {
        let _ = character.set(Self::id(), &[value], None);
        character
    }

    pub fn derive(character: &DataStruct) -> u8 {
        let constitution = Characteristic::Constitution.sum(character);
        let size = Characteristic::Size.sum(character);
        ((constitution + size) / 10) as u8
    }
}

pub struct MagicPoints; // MagicPoints: u8 | POW -> u8

impl MagicPoints {
    pub fn id() -> u32 {
        SecondaryAttribute::MagicPoints.base_id()
    }

    pub fn read(character: &DataStruct) -> u8 {
        character.get(Self::id()).ok().and_then(|b| b.first().copied()).unwrap_or(0)
    }

    pub fn write(character: &mut DataStruct, value: u8) -> &mut DataStruct {
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
        character
            .get(SecondaryAttribute::Luck.base_id())
            .ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0)
    }

    pub fn write(character: &mut DataStruct, value: u8) -> &mut DataStruct {
        let _ = character.set(SecondaryAttribute::Luck.base_id(), &[value], None);
        character
    }

    pub fn roll() -> u8 {
        (dice::roll(&[(3, 6, 0)]) * 5) as u8
    }
}

pub struct Sanity; // Sanity: u8 | POW -> u8

impl Sanity {
    pub fn read(character: &DataStruct) -> u8 {
        character
            .get(SecondaryAttribute::Sanity.base_id())
            .ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0)
    }

    pub fn write(character: &mut DataStruct, value: u8) -> &mut DataStruct {
        let _ = character.set(SecondaryAttribute::Sanity.base_id(), &[value], None);
        character
    }

    pub fn derive(character: &DataStruct) -> u8 {
        Characteristic::Power.read(character).0 as u8
    }
}

pub struct Build; // Build: i8 | STR, SIZ -> i8

impl Build {
    pub fn read(character: &DataStruct) -> i8 {
        character
            .get(SecondaryAttribute::Build.base_id())
            .ok()
            .and_then(|b| b.first().copied())
            .map(|b| b as i8)
            .unwrap_or(0)
    }

    pub fn write(character: &mut DataStruct, value: i8) -> &mut DataStruct {
        let _ = character.set(SecondaryAttribute::Build.base_id(), &[value as u8], None);
        character
    }

    pub fn derive(character: &DataStruct) -> i8 {
        let strength = Characteristic::Strength.sum(character);
        let size = Characteristic::Size.sum(character);
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

pub struct DamageBonus; // DamageBonus: Dice | Build -> Dice

impl DamageBonus {
    pub fn read(character: &DataStruct) -> Dice {
        Self::derive(character)
    }

    pub fn display(character: &DataStruct) -> String {
        dice::display(&[Self::read(character)])
    }

    pub fn derive(character: &DataStruct) -> Dice {
        match Build::read(character) {
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

pub struct MoveRate; // MoveRate: u8 | STR, DEX, SIZ, Age -> u8

impl MoveRate {
    pub fn read(character: &DataStruct) -> u8 {
        character
            .get(SecondaryAttribute::MoveRate.base_id())
            .ok()
            .and_then(|b| b.first().copied())
            .unwrap_or(0)
    }

    pub fn write(character: &mut DataStruct, value: u8) -> &mut DataStruct {
        let _ = character.set(SecondaryAttribute::MoveRate.base_id(), &[value], None);
        character
    }

    pub fn derive(character: &DataStruct) -> u8 {
        let str = Characteristic::Strength.sum(character);
        let dex = Characteristic::Dexterity.sum(character);
        let siz = Characteristic::Size.sum(character);
        let base: i32 = if str > siz && dex > siz {
            9
        } else if str < siz && dex < siz {
            7
        } else {
            8
        };
        let age_penalty: i32 = match Age::read(character) {
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

pub struct OccupationSkillPoints; // OccupationSkillPoints: (Characteristic, Characteristic) | (Characteristic, Characteristic), Profile -> (u16, u16)

impl OccupationSkillPoints {
    pub fn read(character: &DataStruct) -> Option<(Characteristic, Characteristic)> {
        let b = character.get(SecondaryAttribute::OccupationSkillPoints.base_id()).ok()?;
        let c1 = Characteristic::from_id(*b.first()?)?;
        let c2 = Characteristic::from_id(*b.get(1)?)?;
        Some((c1, c2))
    }

    pub fn write(
        character: &mut DataStruct,
        value: (Characteristic, Characteristic),
    ) -> &mut DataStruct {
        let _ = character.set(
            SecondaryAttribute::OccupationSkillPoints.base_id(),
            &[value.0.id() as u8, value.1.id() as u8],
            None,
        );
        character
    }

    pub fn display(
        characteristic_tuple: Option<(Characteristic, Characteristic)>,
        language: Language,
    ) -> String {
        let Some((c1, c2)) = characteristic_tuple else {
            return String::new();
        };
        if c1.id() == c2.id() {
            format!("{}×4", c1.display(language))
        } else {
            format!("{}×2+{}×2", c1.display(language), c2.display(language))
        }
    }

    pub fn derive(character: &DataStruct) -> (u16, u16) {
        let Some((c1, c2)) = Self::read(character) else {
            return (0, 0);
        };
        let used = 0u16;
        let (c1_initial, _, c1_modifier) = c1.read(character);
        let (c2_initial, _, c2_modifier) = c2.read(character);
        let total =
            (c1_initial as i32 + c1_modifier as i32 + c2_initial as i32 + c2_modifier as i32) * 2;
        (used, total.max(0) as u16)
    }
}

pub struct InterestSkillPoints; // InterestSkillPoints: u16 | INT -> (u16, u16)

impl InterestSkillPoints {
    pub fn derive(character: &DataStruct) -> (u16, u16) {
        let used = 0u16;
        let (initial, _, modifier) = Characteristic::Intelligence.read(character);
        let total = ((initial as i32 + modifier as i32) * 2).max(0) as u16;
        (used, total)
    }
}

// === Skill, ArtAndCraft, Fighting, Firearms, LanguageOther, Survival p.54 ===

#[derive(Clone, PartialEq, Eq)]
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

    pub const fn base_id(&self) -> u32 {
        const BASE: u32 = Character::Skill.base_id();
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

impl core::marker::ConstParamTy_ for Skill {}

const OCCUPATION_POINTS: usize = 0;
const INTEREST_POINTS: usize = 1;
const CHANGE: usize = 2;
const MODIFIER: usize = 3;
const PERCENT: usize = 4;

const SKILL_LAYOUT: Layout<(u9, u9, i10, i10)> = Layout::new();
const SKILL_WITH_PERCENT_LAYOUT: Layout<(u9, u9, i10, i10, u7)> = Layout::new();

fn read_raw<T: Fields>(character: &DataStruct, id: u32, layout: &Layout<T>) -> u64 {
    character.get(id).ok().and_then(|b| layout.decode(b)).unwrap_or(0)
}

fn write_raw<T: Fields>(character: &mut DataStruct, id: u32, layout: &Layout<T>, raw: u64) {
    let (word, len) = layout.encode(raw);
    let _ = character.set(id, &word[..len], None);
}

fn read_text(character: &DataStruct, id: u32) -> String {
    character.get(id).ok().map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default()
}

fn read_ids(character: &DataStruct, list_id: u32, index: usize) -> [u32; 2] {
    character.get_indirect::<1, 2>(list_id, [index])[0].unwrap_or([0, 0])
}

fn write_ids(character: &mut DataStruct, list_id: u32, index: usize, ids: [u32; 2]) {
    let _ = character.set_indirect(list_id, [(index, ids)]);
}

pub trait SkillTrait<const S: Skill> {
    const SKILL: Skill = S;
    const BASE_ID: u32 = S.base_id();
    const BASE_PERCENT: u16 = S.base_percent();

    // -> occupation_points, interest_points, change, modifier
    fn read(&self, character: &DataStruct) -> (u9, u9, i10, i10) {
        let raw = read_raw(character, Self::BASE_ID, &SKILL_LAYOUT);
        SKILL_LAYOUT.unpack(raw)
    }

    fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack((occupation_points, interest_points, change, modifier));
        write_raw(character, Self::BASE_ID, &SKILL_LAYOUT, raw);
        character
    }

    // -> skill_name, specialization_name
    fn as_editable_string(&self) -> (String, String) {
        (S.display(Language::Ja).to_owned(), String::new())
    }

    // -> base, occupation_points, interest_points, change, modifier, sum
    fn as_editable_numeric(
        &self,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> (u16, u9, u9, i10, i10, i32) {
        let sum = Self::BASE_PERCENT as i32
            + occupation_points.value() as i32
            + interest_points.value() as i32
            + change.value() as i32
            + modifier.value() as i32;
        (Self::BASE_PERCENT, occupation_points, interest_points, change, modifier, sum)
    }

    fn as_immutable_string(&self) -> String {
        S.display(Language::Ja).to_owned()
    }
}

pub struct Accounting;
impl SkillTrait<{ Skill::Accounting }> for Accounting {}
pub struct Anthropology;
impl SkillTrait<{ Skill::Anthropology }> for Anthropology {}
pub struct Appraise;
impl SkillTrait<{ Skill::Appraise }> for Appraise {}
pub struct Archaeology;
impl SkillTrait<{ Skill::Archaeology }> for Archaeology {}
pub struct Charm;
impl SkillTrait<{ Skill::Charm }> for Charm {}
pub struct Climb;
impl SkillTrait<{ Skill::Climb }> for Climb {}
pub struct ComputerUse;
impl SkillTrait<{ Skill::ComputerUse }> for ComputerUse {}
pub struct Disguise;
impl SkillTrait<{ Skill::Disguise }> for Disguise {}
pub struct DriveAuto;
impl SkillTrait<{ Skill::DriveAuto }> for DriveAuto {}
pub struct ElectricalRepair;
impl SkillTrait<{ Skill::ElectricalRepair }> for ElectricalRepair {}
pub struct Electronics;
impl SkillTrait<{ Skill::Electronics }> for Electronics {}
pub struct FastTalk;
impl SkillTrait<{ Skill::FastTalk }> for FastTalk {}
pub struct FirstAid;
impl SkillTrait<{ Skill::FirstAid }> for FirstAid {}
pub struct History;
impl SkillTrait<{ Skill::History }> for History {}
pub struct Intimidate;
impl SkillTrait<{ Skill::Intimidate }> for Intimidate {}
pub struct Jump;
impl SkillTrait<{ Skill::Jump }> for Jump {}
pub struct Law;
impl SkillTrait<{ Skill::Law }> for Law {}
pub struct LibraryUse;
impl SkillTrait<{ Skill::LibraryUse }> for LibraryUse {}
pub struct Listen;
impl SkillTrait<{ Skill::Listen }> for Listen {}
pub struct Locksmith;
impl SkillTrait<{ Skill::Locksmith }> for Locksmith {}
pub struct MechanicalRepair;
impl SkillTrait<{ Skill::MechanicalRepair }> for MechanicalRepair {}
pub struct Medicine;
impl SkillTrait<{ Skill::Medicine }> for Medicine {}
pub struct NaturalWorld;
impl SkillTrait<{ Skill::NaturalWorld }> for NaturalWorld {}
pub struct Navigate;
impl SkillTrait<{ Skill::Navigate }> for Navigate {}
pub struct Occult;
impl SkillTrait<{ Skill::Occult }> for Occult {}
pub struct Persuade;
impl SkillTrait<{ Skill::Persuade }> for Persuade {}
pub struct Psychoanalysis;
impl SkillTrait<{ Skill::Psychoanalysis }> for Psychoanalysis {}
pub struct Psychology;
impl SkillTrait<{ Skill::Psychology }> for Psychology {}
pub struct Ride;
impl SkillTrait<{ Skill::Ride }> for Ride {}
pub struct SleightOfHand;
impl SkillTrait<{ Skill::SleightOfHand }> for SleightOfHand {}
pub struct SpotHidden;
impl SkillTrait<{ Skill::SpotHidden }> for SpotHidden {}
pub struct Stealth;
impl SkillTrait<{ Skill::Stealth }> for Stealth {}
pub struct Swim;
impl SkillTrait<{ Skill::Swim }> for Swim {}
pub struct Throw;
impl SkillTrait<{ Skill::Throw }> for Throw {}
pub struct Track;
impl SkillTrait<{ Skill::Track }> for Track {}

pub struct Dodge;
impl SkillTrait<{ Skill::Dodge }> for Dodge {}

pub struct LanguageOwn;

impl LanguageOwn {
    const BASE_ID: u32 = Skill::LanguageOwn.base_id();

    // -> occupation_points, interest_points, change, modifier, name
    pub fn read(&self, character: &DataStruct) -> (u16, u16, i16, i16, String) {
        let raw = read_raw(character, Self::BASE_ID, &SKILL_LAYOUT);
        let name = read_text(character, Self::BASE_ID + 1);
        (
            SKILL_LAYOUT.get(raw, OCCUPATION_POINTS),
            SKILL_LAYOUT.get(raw, INTEREST_POINTS),
            SKILL_LAYOUT.get(raw, CHANGE),
            SKILL_LAYOUT.get(raw, MODIFIER),
            name,
        )
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
        name: &str,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack_raw(&[
            occupation_points as u64,
            interest_points as u64,
            change as u64,
            modifier as u64,
        ]);
        write_raw(character, Self::BASE_ID, &SKILL_LAYOUT, raw);
        let _ = character.set(Self::BASE_ID + 1, name.as_bytes(), None);
        character
    }

    pub fn as_editable_numeric(
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
    ) -> (u16, u16, u16, i16, i16, i32) {
        const BASE: u16 = 1;
        let sum = BASE as i32
            + occupation_points as i32
            + interest_points as i32
            + change as i32
            + modifier as i32;
        (BASE, occupation_points, interest_points, change, modifier, sum)
    }

    // -> skill_name, name
    pub fn as_editable_string<'a>(name: &'a str, language: Language) -> (&'static str, &'a str) {
        (Skill::LanguageOwn.display(language), name)
    }

    pub fn as_immutable_string(name: &str, language: Language) -> String {
        format!("{} ({})", Skill::LanguageOwn.display(language), name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub const fn base_id(&self) -> u32 {
        Skill::ArtAndCraft.base_id() + *self as u32
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

impl core::marker::ConstParamTy_ for ArtAndCraft {}

pub trait ArtAndCraftTrait<const A: ArtAndCraft> {
    const SKILL: Skill = Skill::ArtAndCraft;
    const SPECIFICATION: ArtAndCraft = A;
    const BASE_ID: u32 = A.base_id();
    const BASE_PERCENT: u16 = Skill::ArtAndCraft.base_percent();

    // -> occupation_points, interest_points, change, modifier
    fn read(&self, character: &DataStruct) -> (u9, u9, i10, i10) {
        let raw = read_raw(character, Self::BASE_ID, &SKILL_LAYOUT);
        SKILL_LAYOUT.unpack(raw)
    }

    fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack((occupation_points, interest_points, change, modifier));
        write_raw(character, Self::BASE_ID, &SKILL_LAYOUT, raw);
        character
    }

    // -> skill_name, specialization_name
    fn as_editable_string(&self, language: Language) -> (String, String) {
        (Skill::ArtAndCraft.display(language).to_owned(), A.display(language).to_owned())
    }

    // -> base, occupation_points, interest_points, change, modifier, sum
    fn as_editable_numeric(
        &self,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> (u16, u9, u9, i10, i10, i32) {
        let sum = Self::BASE_PERCENT as i32
            + occupation_points.value() as i32
            + interest_points.value() as i32
            + change.value() as i32
            + modifier.value() as i32;
        (Self::BASE_PERCENT, occupation_points, interest_points, change, modifier, sum)
    }
}

pub struct Acting;
impl ArtAndCraftTrait<{ ArtAndCraft::Acting }> for Acting {}
pub struct Barber;
impl ArtAndCraftTrait<{ ArtAndCraft::Barber }> for Barber {}
pub struct Calligraphy;
impl ArtAndCraftTrait<{ ArtAndCraft::Calligraphy }> for Calligraphy {}
pub struct Carpentry;
impl ArtAndCraftTrait<{ ArtAndCraft::Carpentry }> for Carpentry {}
pub struct Cook;
impl ArtAndCraftTrait<{ ArtAndCraft::Cook }> for Cook {}
pub struct Dancing;
impl ArtAndCraftTrait<{ ArtAndCraft::Dancing }> for Dancing {}
pub struct FineArt;
impl ArtAndCraftTrait<{ ArtAndCraft::FineArt }> for FineArt {}
pub struct Forgery;
impl ArtAndCraftTrait<{ ArtAndCraft::Forgery }> for Forgery {}
pub struct Photography;
impl ArtAndCraftTrait<{ ArtAndCraft::Photography }> for Photography {}
pub struct Pottery;
impl ArtAndCraftTrait<{ ArtAndCraft::Pottery }> for Pottery {}
pub struct Sculpting;
impl ArtAndCraftTrait<{ ArtAndCraft::Sculpting }> for Sculpting {}
pub struct Writing;
impl ArtAndCraftTrait<{ ArtAndCraft::Writing }> for Writing {}

pub struct ArtAndCraftCustom(pub u8);

impl ArtAndCraftCustom {
    const LIST_ID: u32 = ArtAndCraft::Custom.base_id();
    pub const fn list_id() -> u32 {
        Self::LIST_ID
    }

    // -> occupation_points, interest_points, change, modifier, name
    pub fn read(&self, character: &DataStruct) -> (u16, u16, i16, i16, String) {
        let [numeric_id, name_id] = read_ids(character, Self::LIST_ID, self.0 as usize);
        let raw = read_raw(character, numeric_id, &SKILL_LAYOUT);
        let name = read_text(character, name_id);
        (
            SKILL_LAYOUT.get(raw, OCCUPATION_POINTS),
            SKILL_LAYOUT.get(raw, INTEREST_POINTS),
            SKILL_LAYOUT.get(raw, CHANGE),
            SKILL_LAYOUT.get(raw, MODIFIER),
            name,
        )
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        ids: [u32; 2],
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
        name: &str,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack_raw(&[
            occupation_points as u64,
            interest_points as u64,
            change as u64,
            modifier as u64,
        ]);
        write_ids(character, Self::LIST_ID, self.0 as usize, ids);
        write_raw(character, ids[0], &SKILL_LAYOUT, raw);
        let _ = character.set(ids[1], name.as_bytes(), None);
        character
    }

    pub fn as_editable_numeric(
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
    ) -> (u16, u16, u16, i16, i16, i32) {
        const BASE: u16 = 5;
        let sum = BASE as i32
            + occupation_points as i32
            + interest_points as i32
            + change as i32
            + modifier as i32;
        (BASE, occupation_points, interest_points, change, modifier, sum)
    }

    // -> skill_name, specialization_name
    pub fn as_editable_string<'a>(name: &'a str, language: Language) -> (&'static str, &'a str) {
        (Skill::ArtAndCraft.display(language), name)
    }

    pub fn as_immutable_string(name: &str, language: Language) -> String {
        format!("{} ({})", Skill::ArtAndCraft.display(language), name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

    pub const fn id(&self, base: u32) -> u32 {
        base + *self as u32
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

impl core::marker::ConstParamTy_ for Fighting {}

pub trait FightingTrait<const F: Fighting> {
    const SKILL: Skill = Skill::Fighting;
    const SPECIALIZATION: Fighting = F;
    const BASE_ID: u32 = F.id(Skill::Fighting.base_id());
    const BASE_PERCENT: u16 = F.base_percent();

    // -> occupation_points, interest_points, change, modifier
    fn read(&self, character: &DataStruct) -> (u9, u9, i10, i10) {
        let raw = read_raw(character, Self::BASE_ID, &SKILL_LAYOUT);
        SKILL_LAYOUT.unpack(raw)
    }

    fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack((occupation_points, interest_points, change, modifier));
        write_raw(character, Self::BASE_ID, &SKILL_LAYOUT, raw);
        character
    }

    // -> skill_name, specialization_name
    fn as_editable_string(&self, language: Language) -> (&'static str, &'static str) {
        (Skill::Fighting.display(language), F.display(language))
    }

    // -> base, occupation_points, interest_points, change, modifier, sum
    fn as_editable_numeric(
        &self,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> (u16, u9, u9, i10, i10, i10) {
        let sum = Self::BASE_PERCENT as i32
            + occupation_points.value() as i32
            + interest_points.value() as i32
            + change.value() as i32
            + modifier.value() as i32;
        (
            Self::BASE_PERCENT,
            occupation_points,
            interest_points,
            change,
            modifier,
            i10::new(sum as i16),
        )
    }
}

pub struct Axe;
impl FightingTrait<{ Fighting::Axe }> for Axe {}
pub struct Brawl;
impl FightingTrait<{ Fighting::Brawl }> for Brawl {}
pub struct Chainsaw;
impl FightingTrait<{ Fighting::Chainsaw }> for Chainsaw {}
pub struct Flail;
impl FightingTrait<{ Fighting::Flail }> for Flail {}
pub struct Garrote;
impl FightingTrait<{ Fighting::Garrote }> for Garrote {}
pub struct Spear;
impl FightingTrait<{ Fighting::Spear }> for Spear {}
pub struct Sword;
impl FightingTrait<{ Fighting::Sword }> for Sword {}
pub struct Whip;
impl FightingTrait<{ Fighting::Whip }> for Whip {}

pub struct FightingCustom(pub u8);

impl FightingCustom {
    const LIST_ID: u32 = Skill::Fighting.base_id() + 9;
    pub const fn list_id() -> u32 {
        Self::LIST_ID
    }

    fn numeric_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2 - 1
    }
    fn name_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2
    }

    // -> occupation_points, interest_points, change, modifier, name
    pub fn read(&self, character: &DataStruct) -> (u16, u16, u16, i16, i16, String) {
        let [numeric_id, name_id] = read_ids(character, Self::LIST_ID, self.0 as usize);
        let raw = read_raw(character, numeric_id, &SKILL_WITH_PERCENT_LAYOUT);
        let name = read_text(character, name_id);
        (
            SKILL_WITH_PERCENT_LAYOUT.get(raw, PERCENT),
            SKILL_WITH_PERCENT_LAYOUT.get(raw, OCCUPATION_POINTS),
            SKILL_WITH_PERCENT_LAYOUT.get(raw, INTEREST_POINTS),
            SKILL_WITH_PERCENT_LAYOUT.get(raw, CHANGE),
            SKILL_WITH_PERCENT_LAYOUT.get(raw, MODIFIER),
            name,
        )
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        ids: [u32; 2],
        base_percent: u16,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
        name: &str,
    ) -> &'a mut DataStruct {
        let raw = SKILL_WITH_PERCENT_LAYOUT.pack_raw(&[
            occupation_points as u64,
            interest_points as u64,
            change as u64,
            modifier as u64,
            base_percent as u64,
        ]);
        write_ids(character, Self::LIST_ID, self.0 as usize, ids);
        write_raw(character, ids[0], &SKILL_WITH_PERCENT_LAYOUT, raw);
        let _ = character.set(ids[1], name.as_bytes(), None);
        character
    }

    // -> base_percent, occupation_points, interest_points, change, modifier, sum
    pub fn as_editable_numeric(
        base_percent: u16,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
    ) -> (u16, u16, u16, i16, i16, i32) {
        let sum = base_percent as i32
            + occupation_points as i32
            + interest_points as i32
            + change as i32
            + modifier as i32;
        (base_percent, occupation_points, interest_points, change, modifier, sum)
    }

    // -> skill_name, specialization_name
    pub fn as_editable_string<'a>(name: &'a str, language: Language) -> (&'static str, &'a str) {
        (Skill::Fighting.display(language), name)
    }

    pub fn as_immutable_string(name: &str, language: Language) -> String {
        format!("{} ({})", Skill::Fighting.display(language), name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

    pub const fn id(&self, base: u32) -> u32 {
        base + *self as u32
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

impl core::marker::ConstParamTy_ for Firearms {}

pub trait FirearmsTrait<const F: Firearms> {
    const SKILL: Skill = Skill::Firearms;
    const SPECIALIZATION: Firearms = F;
    const BASE_ID: u32 = F.id(Skill::Firearms.base_id());
    const BASE_PERCENT: u16 = F.base_percent();

    fn read(&self, character: &DataStruct) -> (u9, u9, i10, i10) {
        let raw = read_raw(character, Self::BASE_ID, &SKILL_LAYOUT);
        SKILL_LAYOUT.unpack(raw)
    }

    fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack((occupation_points, interest_points, change, modifier));
        write_raw(character, Self::BASE_ID, &SKILL_LAYOUT, raw);
        character
    }

    // -> skill_name, specialization_label
    fn as_editable_string(&self, language: Language) -> (&'static str, &'static str) {
        (Skill::Firearms.display(language), F.display(language))
    }

    // -> base, occupation_points, interest_points, change, modifier, sum
    fn as_editable_numeric(
        &self,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> (u16, u9, u9, i10, i10, i10) {
        let sum = Self::BASE_PERCENT as i32
            + occupation_points.value() as i32
            + interest_points.value() as i32
            + change.value() as i32
            + modifier.value() as i32;
        (
            Self::BASE_PERCENT,
            occupation_points,
            interest_points,
            change,
            modifier,
            i10::new(sum as i16),
        )
    }
}

pub struct FirearmsBow;
impl FirearmsTrait<{ Firearms::Bow }> for FirearmsBow {}
pub struct FlameThrower;
impl FirearmsTrait<{ Firearms::FlameThrower }> for FlameThrower {}
pub struct Handgun;
impl FirearmsTrait<{ Firearms::Handgun }> for Handgun {}
pub struct HeavyWeapons;
impl FirearmsTrait<{ Firearms::HeavyWeapons }> for HeavyWeapons {}
pub struct MachineGun;
impl FirearmsTrait<{ Firearms::MachineGun }> for MachineGun {}
pub struct RifleShotgun;
impl FirearmsTrait<{ Firearms::RifleShotgun }> for RifleShotgun {}
pub struct SubmachineGun;
impl FirearmsTrait<{ Firearms::SubmachineGun }> for SubmachineGun {}

pub struct FirearmsCustom(pub u8);

impl FirearmsCustom {
    const LIST_ID: u32 = Skill::Firearms.base_id() + 8;
    pub const fn list_id() -> u32 {
        Self::LIST_ID
    }

    fn numeric_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2 - 1
    }
    fn name_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2
    }

    // -> occupation_points, interest_points, change, modifier, name
    pub fn read(&self, character: &DataStruct) -> (u16, u16, u16, i16, i16, String) {
        let [numeric_id, name_id] = read_ids(character, Self::LIST_ID, self.0 as usize);
        let raw = read_raw(character, numeric_id, &SKILL_WITH_PERCENT_LAYOUT);
        let name = read_text(character, name_id);
        (
            SKILL_WITH_PERCENT_LAYOUT.get(raw, PERCENT),
            SKILL_WITH_PERCENT_LAYOUT.get(raw, OCCUPATION_POINTS),
            SKILL_WITH_PERCENT_LAYOUT.get(raw, INTEREST_POINTS),
            SKILL_WITH_PERCENT_LAYOUT.get(raw, CHANGE),
            SKILL_WITH_PERCENT_LAYOUT.get(raw, MODIFIER),
            name,
        )
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        ids: [u32; 2],
        base_percent: u16,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
        name: &str,
    ) -> &'a mut DataStruct {
        let raw = SKILL_WITH_PERCENT_LAYOUT.pack_raw(&[
            occupation_points as u64,
            interest_points as u64,
            change as u64,
            modifier as u64,
            base_percent as u64,
        ]);
        write_ids(character, Self::LIST_ID, self.0 as usize, ids);
        write_raw(character, ids[0], &SKILL_WITH_PERCENT_LAYOUT, raw);
        let _ = character.set(ids[1], name.as_bytes(), None);
        character
    }

    // -> base_percent, occupation_points, interest_points, change, modifier, sum
    pub fn as_editable_numeric(
        base_percent: u16,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
    ) -> (u16, u16, u16, i16, i16, i32) {
        let sum = base_percent as i32
            + occupation_points as i32
            + interest_points as i32
            + change as i32
            + modifier as i32;
        (base_percent, occupation_points, interest_points, change, modifier, sum)
    }

    // -> skill_name, specialization_name
    pub fn as_editable_string<'a>(name: &'a str, language: Language) -> (&'static str, &'a str) {
        (Skill::Firearms.display(language), name)
    }

    pub fn as_immutable_string(name: &str, language: Language) -> String {
        format!("{} ({})", Skill::Firearms.display(language), name)
    }
}

pub struct LanguageOther(pub u8);

impl LanguageOther {
    const LIST_ID: u32 = Skill::LanguageOther.base_id();
    pub const fn list_id() -> u32 {
        Self::LIST_ID
    }
    const BASE_PERCENT: u16 = 1;

    fn numeric_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2 - 1
    }
    fn name_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2
    }

    // -> occupation_points, interest_points, change, modifier, name
    pub fn read(&self, character: &DataStruct) -> (u16, u16, i16, i16, String) {
        let [numeric_id, name_id] = read_ids(character, Self::LIST_ID, self.0 as usize);
        let raw = read_raw(character, numeric_id, &SKILL_LAYOUT);
        let name = read_text(character, name_id);
        (
            SKILL_LAYOUT.get(raw, OCCUPATION_POINTS),
            SKILL_LAYOUT.get(raw, INTEREST_POINTS),
            SKILL_LAYOUT.get(raw, CHANGE),
            SKILL_LAYOUT.get(raw, MODIFIER),
            name,
        )
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        ids: [u32; 2],
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
        name: &str,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack_raw(&[
            occupation_points as u64,
            interest_points as u64,
            change as u64,
            modifier as u64,
        ]);
        write_ids(character, Self::LIST_ID, self.0 as usize, ids);
        write_raw(character, ids[0], &SKILL_LAYOUT, raw);
        let _ = character.set(ids[1], name.as_bytes(), None);
        character
    }

    pub fn as_editable_numeric(
        &self,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
    ) -> (u16, u16, u16, i16, i16, i32) {
        let sum = Self::BASE_PERCENT as i32
            + occupation_points as i32
            + interest_points as i32
            + change as i32
            + modifier as i32;
        (Self::BASE_PERCENT, occupation_points, interest_points, change, modifier, sum)
    }

    // -> skill_name, language_name
    pub fn as_editable_string<'a>(name: &'a str, language: Language) -> (&'static str, &'a str) {
        (Skill::LanguageOther.display(language), name)
    }

    pub fn as_immutable_string(name: &str, language: Language) -> String {
        format!("{} ({})", Skill::LanguageOther.display(language), name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

    pub const fn id(&self, base: u32) -> u32 {
        base + *self as u32
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

impl core::marker::ConstParamTy_ for Pilot {}

pub trait PilotTrait<const P: Pilot> {
    const SKILL: Skill = Skill::Pilot;
    const SPECIALIZATION: Pilot = P;
    const BASE_ID: u32 = P.id(Skill::Pilot.base_id());
    const BASE_PERCENT: u16 = 1;

    fn read(&self, character: &DataStruct) -> (u9, u9, i10, i10) {
        let raw = read_raw(character, Self::BASE_ID, &SKILL_LAYOUT);
        SKILL_LAYOUT.unpack(raw)
    }

    fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack((occupation_points, interest_points, change, modifier));
        write_raw(character, Self::BASE_ID, &SKILL_LAYOUT, raw);
        character
    }

    fn as_editable_string(&self, language: Language) -> (&'static str, &'static str) {
        (Skill::Pilot.display(language), P.display(language))
    }

    fn as_editable_numeric(
        &self,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> (u16, u9, u9, i10, i10, i10) {
        let sum = Self::BASE_PERCENT as i32
            + occupation_points.value() as i32
            + interest_points.value() as i32
            + change.value() as i32
            + modifier.value() as i32;
        (
            Self::BASE_PERCENT,
            occupation_points,
            interest_points,
            change,
            modifier,
            i10::new(sum as i16),
        )
    }
}

pub struct PilotBoat;
impl PilotTrait<{ Pilot::Boat }> for PilotBoat {}
pub struct PilotSteamShip;
impl PilotTrait<{ Pilot::SteamShip }> for PilotSteamShip {}
pub struct PilotSailboat;
impl PilotTrait<{ Pilot::Sailboat }> for PilotSailboat {}
pub struct PilotCivilProp;
impl PilotTrait<{ Pilot::CivilProp }> for PilotCivilProp {}
pub struct PilotBalloon;
impl PilotTrait<{ Pilot::Balloon }> for PilotBalloon {}
pub struct PilotDirigible;
impl PilotTrait<{ Pilot::Dirigible }> for PilotDirigible {}
pub struct PilotCivilJet;
impl PilotTrait<{ Pilot::CivilJet }> for PilotCivilJet {}
pub struct PilotAirliner;
impl PilotTrait<{ Pilot::Airliner }> for PilotAirliner {}
pub struct PilotJetFighter;
impl PilotTrait<{ Pilot::JetFighter }> for PilotJetFighter {}
pub struct PilotHelicopter;
impl PilotTrait<{ Pilot::Helicopter }> for PilotHelicopter {}

pub struct PilotCustom(pub u8);

impl PilotCustom {
    const LIST_ID: u32 = Skill::Pilot.base_id() + 11;
    pub const fn list_id() -> u32 {
        Self::LIST_ID
    }

    fn numeric_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2 - 1
    }
    fn name_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2
    }

    // -> occupation_points, interest_points, change, modifier, name
    pub fn read(&self, character: &DataStruct) -> (u16, u16, i16, i16, String) {
        let raw = read_raw(character, self.numeric_id(), &SKILL_LAYOUT);
        let name = read_text(character, self.name_id());
        (
            SKILL_LAYOUT.get(raw, OCCUPATION_POINTS),
            SKILL_LAYOUT.get(raw, INTEREST_POINTS),
            SKILL_LAYOUT.get(raw, CHANGE),
            SKILL_LAYOUT.get(raw, MODIFIER),
            name,
        )
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
        name: &str,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack_raw(&[
            occupation_points as u64,
            interest_points as u64,
            change as u64,
            modifier as u64,
        ]);
        write_raw(character, self.numeric_id(), &SKILL_LAYOUT, raw);
        let _ = character.set(self.name_id(), name.as_bytes(), None);
        character
    }

    pub fn as_editable_numeric(
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
    ) -> (u16, u16, u16, i16, i16, i32) {
        const BASE: u16 = 1;
        let sum = BASE as i32
            + occupation_points as i32
            + interest_points as i32
            + change as i32
            + modifier as i32;
        (BASE, occupation_points, interest_points, change, modifier, sum)
    }

    // -> skill_name, specialization_name
    pub fn as_editable_string<'a>(name: &'a str, language: Language) -> (&'static str, &'a str) {
        (Skill::Pilot.display(language), name)
    }

    pub fn as_immutable_string(name: &str, language: Language) -> String {
        format!("{} ({})", Skill::Pilot.display(language), name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

    pub const fn id(&self, base: u32) -> u32 {
        match self {
            Self::None => unreachable!(),
            _ => base + *self as u32,
        }
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

impl core::marker::ConstParamTy_ for Science {}

pub trait ScienceTrait<const S: Science> {
    const SKILL: Skill = Skill::Science;
    const SPECIALIZATION: Science = S;
    const BASE_ID: u32 = S.id(Skill::Science.base_id());
    const BASE_PERCENT: u16 = 1;

    fn read(&self, character: &DataStruct) -> (u9, u9, i10, i10) {
        let raw = read_raw(character, Self::BASE_ID, &SKILL_LAYOUT);
        SKILL_LAYOUT.unpack(raw)
    }

    fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack((occupation_points, interest_points, change, modifier));
        write_raw(character, Self::BASE_ID, &SKILL_LAYOUT, raw);
        character
    }

    fn as_editable_string(&self, language: Language) -> (&'static str, Option<&'static str>) {
        (Skill::Science.display(language), S.display(language))
    }

    fn as_editable_numeric(
        &self,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> (u16, u9, u9, i10, i10, i10) {
        let sum = Self::BASE_PERCENT as i32
            + occupation_points.value() as i32
            + interest_points.value() as i32
            + change.value() as i32
            + modifier.value() as i32;
        (
            Self::BASE_PERCENT,
            occupation_points,
            interest_points,
            change,
            modifier,
            i10::new(sum as i16),
        )
    }
}

pub struct ScienceAstronomy;
impl ScienceTrait<{ Science::Astronomy }> for ScienceAstronomy {}
pub struct ScienceBiology;
impl ScienceTrait<{ Science::Biology }> for ScienceBiology {}
pub struct ScienceBotany;
impl ScienceTrait<{ Science::Botany }> for ScienceBotany {}
pub struct ScienceChemistry;
impl ScienceTrait<{ Science::Chemistry }> for ScienceChemistry {}
pub struct ScienceCryptography;
impl ScienceTrait<{ Science::Cryptography }> for ScienceCryptography {}
pub struct ScienceEngineering;
impl ScienceTrait<{ Science::Engineering }> for ScienceEngineering {}
pub struct ScienceForensics;
impl ScienceTrait<{ Science::Forensics }> for ScienceForensics {}
pub struct ScienceGeology;
impl ScienceTrait<{ Science::Geology }> for ScienceGeology {}
pub struct ScienceMathematics;
impl ScienceTrait<{ Science::Mathematics }> for ScienceMathematics {}
pub struct ScienceMeteorology;
impl ScienceTrait<{ Science::Meteorology }> for ScienceMeteorology {}
pub struct SciencePharmacy;
impl ScienceTrait<{ Science::Pharmacy }> for SciencePharmacy {}
pub struct SciencePhysics;
impl ScienceTrait<{ Science::Physics }> for SciencePhysics {}
pub struct ScienceZoology;
impl ScienceTrait<{ Science::Zoology }> for ScienceZoology {}

pub struct ScienceCustom(pub u8);

impl ScienceCustom {
    const LIST_ID: u32 = Skill::Science.base_id() + 14;
    pub const fn list_id() -> u32 {
        Self::LIST_ID
    }

    fn numeric_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2 - 1
    }
    fn name_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2
    }

    // -> occupation_points, interest_points, change, modifier, name
    pub fn read(&self, character: &DataStruct) -> (u16, u16, i16, i16, String) {
        let raw = read_raw(character, self.numeric_id(), &SKILL_LAYOUT);
        let name = read_text(character, self.name_id());
        (
            SKILL_LAYOUT.get(raw, OCCUPATION_POINTS),
            SKILL_LAYOUT.get(raw, INTEREST_POINTS),
            SKILL_LAYOUT.get(raw, CHANGE),
            SKILL_LAYOUT.get(raw, MODIFIER),
            name,
        )
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
        name: &str,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack_raw(&[
            occupation_points as u64,
            interest_points as u64,
            change as u64,
            modifier as u64,
        ]);
        write_raw(character, self.numeric_id(), &SKILL_LAYOUT, raw);
        let _ = character.set(self.name_id(), name.as_bytes(), None);
        character
    }

    pub fn as_editable_numeric(
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
    ) -> (u16, u16, u16, i16, i16, i32) {
        const BASE: u16 = 1;
        let sum = BASE as i32
            + occupation_points as i32
            + interest_points as i32
            + change as i32
            + modifier as i32;
        (BASE, occupation_points, interest_points, change, modifier, sum)
    }

    // -> skill_name, specialization_name
    pub fn as_editable_string<'a>(name: &'a str, language: Language) -> (&'static str, &'a str) {
        (Skill::Science.display(language), name)
    }

    pub fn as_immutable_string(name: &str, language: Language) -> String {
        format!("{} ({})", Skill::Science.display(language), name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

    pub const fn id(&self, base: u32) -> u32 {
        base + *self as u32
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

impl core::marker::ConstParamTy_ for Survival {}

pub trait SurvivalTrait<const S: Survival> {
    const SKILL: Skill = Skill::Survival;
    const SPECIALIZATION: Survival = S;
    const BASE_ID: u32 = S.id(Skill::Survival.base_id());
    const BASE_PERCENT: u16 = 10;

    fn read(&self, character: &DataStruct) -> (u9, u9, i10, i10) {
        let raw = read_raw(character, Self::BASE_ID, &SKILL_LAYOUT);
        SKILL_LAYOUT.unpack(raw)
    }

    fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack((occupation_points, interest_points, change, modifier));
        write_raw(character, Self::BASE_ID, &SKILL_LAYOUT, raw);
        character
    }

    fn as_editable_string(&self, language: Language) -> (&'static str, &'static str) {
        (Skill::Survival.display(language), S.display(language))
    }

    fn as_editable_numeric(
        &self,
        occupation_points: u9,
        interest_points: u9,
        change: i10,
        modifier: i10,
    ) -> (u16, u9, u9, i10, i10, i10) {
        let sum = Self::BASE_PERCENT as i32
            + occupation_points.value() as i32
            + interest_points.value() as i32
            + change.value() as i32
            + modifier.value() as i32;
        (
            Self::BASE_PERCENT,
            occupation_points,
            interest_points,
            change,
            modifier,
            i10::new(sum as i16),
        )
    }
}

pub struct SurvivalArctic;
impl SurvivalTrait<{ Survival::Arctic }> for SurvivalArctic {}
pub struct SurvivalDesert;
impl SurvivalTrait<{ Survival::Desert }> for SurvivalDesert {}
pub struct SurvivalSea;
impl SurvivalTrait<{ Survival::Sea }> for SurvivalSea {}

pub struct SurvivalCustom(pub u8);

impl SurvivalCustom {
    const LIST_ID: u32 = Skill::Survival.base_id() + 4;
    pub const fn list_id() -> u32 {
        Self::LIST_ID
    }

    fn numeric_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2 - 1
    }
    fn name_id(&self) -> u32 {
        Self::LIST_ID + self.0 as u32 * 2
    }

    // -> occupation_points, interest_points, change, modifier, name
    pub fn read(&self, character: &DataStruct) -> (u16, u16, i16, i16, String) {
        let raw = read_raw(character, self.numeric_id(), &SKILL_LAYOUT);
        let name = read_text(character, self.name_id());
        (
            SKILL_LAYOUT.get(raw, OCCUPATION_POINTS),
            SKILL_LAYOUT.get(raw, INTEREST_POINTS),
            SKILL_LAYOUT.get(raw, CHANGE),
            SKILL_LAYOUT.get(raw, MODIFIER),
            name,
        )
    }

    pub fn write<'a>(
        &self,
        character: &'a mut DataStruct,
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
        name: &str,
    ) -> &'a mut DataStruct {
        let raw = SKILL_LAYOUT.pack_raw(&[
            occupation_points as u64,
            interest_points as u64,
            change as u64,
            modifier as u64,
        ]);
        write_raw(character, self.numeric_id(), &SKILL_LAYOUT, raw);
        let _ = character.set(self.name_id(), name.as_bytes(), None);
        character
    }

    pub fn as_editable_numeric(
        occupation_points: u16,
        interest_points: u16,
        change: i16,
        modifier: i16,
    ) -> (u16, u16, u16, i16, i16, i32) {
        const BASE: u16 = 10;
        let sum = BASE as i32
            + occupation_points as i32
            + interest_points as i32
            + change as i32
            + modifier as i32;
        (BASE, occupation_points, interest_points, change, modifier, sum)
    }

    // -> skill_name, specialization_name
    pub fn as_editable_string<'a>(name: &'a str, language: Language) -> (&'static str, &'a str) {
        (Skill::Survival.display(language), name)
    }

    pub fn as_immutable_string(name: &str, language: Language) -> String {
        format!("{} ({})", Skill::Survival.display(language), name)
    }
}

// === Possession p.44 ===

#[derive(Clone, Copy)]
pub enum Possession {
    Weapon = 1,
    Armor,
    Equipment,
    Wealth,
}

impl Possession {
    pub fn id(&self) -> u32 {
        Character::Possession.base_id() + *self as u32
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
        Character::Backstory.base_id()
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

///   [title_len: u32 LE][title: utf-8][body: utf-8]
pub struct Memo {
    pub slot: usize,
}

pub const MAX_MEMO_SLOTS: usize = 8;

impl Memo {
    pub fn list() -> [Memo; MAX_MEMO_SLOTS] {
        from_fn(|slot| Memo { slot })
    }

    pub fn id(&self) -> u32 {
        Character::Memo.base_id() + self.slot as u32
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
        let title_len =
            bytes.get(0..4).and_then(|b| b.try_into().ok()).map(u32::from_le_bytes).unwrap_or(0)
                as usize;
        let title_start = 4;
        let title = bytes
            .get(title_start..title_start + title_len)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let body = bytes
            .get(title_start + title_len..)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        (title, body)
    }

    pub fn display(bytes: &[u8]) -> (String, String) {
        Self::decode(bytes)
    }
}

#[cfg(test)]
mod skill_storage_tests {
    use alloc::{string::ToString, vec::Vec};

    use arbitrary_int::{i10, u9};

    use super::*;
    use crate::Rng;

    fn character() -> DataStruct {
        DataStruct::new(1, 0.0, SCHEMA_SIZE)
    }

    #[test]
    fn language_own_round_trips_with_name() {
        let mut c = character();
        LanguageOwn.write(&mut c, 400, 300, -400, 400, "Japanese");
        assert_eq!(LanguageOwn.read(&c), (400, 300, -400, 400, "Japanese".to_string()));
    }

    #[test]
    fn art_and_craft_custom_round_trips_via_indirect_ids() {
        let mut c = character();
        ArtAndCraftCustom(0).write(&mut c, [1000, 1001], 400, 1, -1, 400, "Origami");
        ArtAndCraftCustom(1).write(&mut c, [1002, 1003], 2, 300, 5, -400, "Ikebana");
        assert_eq!(ArtAndCraftCustom(0).read(&c), (400, 1, -1, 400, "Origami".to_string()));
        assert_eq!(ArtAndCraftCustom(1).read(&c), (2, 300, 5, -400, "Ikebana".to_string()));
    }

    #[test]
    fn art_and_craft_custom_unwritten_index_reads_empty() {
        let mut c = character();
        ArtAndCraftCustom(0).write(&mut c, [1000, 1001], 1, 1, 1, 1, "x");
        assert_eq!(ArtAndCraftCustom(5).read(&c), (0, 0, 0, 0, String::new()));
    }

    #[test]
    fn fighting_custom_keeps_base_percent() {
        let mut c = character();
        FightingCustom(0).write(&mut c, [1000, 1001], 127, 400, 300, -400, 400, "Kendo");
        assert_eq!(FightingCustom(0).read(&c), (127, 400, 300, -400, 400, "Kendo".to_string()));
    }

    #[test]
    fn firearms_custom_keeps_base_percent() {
        let mut c = character();
        FirearmsCustom(0).write(&mut c, [1000, 1001], 99, 400, 0, 0, 0, "Crossbow");
        assert_eq!(FirearmsCustom(0).read(&c), (99, 400, 0, 0, 0, "Crossbow".to_string()));
        FirearmsCustom(1).write(&mut c, [1002, 1003], 100, 0, 0, -400, -1, "x");
        assert_eq!(FirearmsCustom(1).read(&c), (100, 0, 0, -400, -1, "x".to_string()));
    }

    #[test]
    fn pilot_custom_uses_numeric_and_name_ids() {
        let mut c = character();
        PilotCustom(1).write(&mut c, 400, 7, -7, 400, "Rocket");
        assert_eq!(PilotCustom(1).read(&c), (400, 7, -7, 400, "Rocket".to_string()));
    }

    fn random_skill_values(rng: &mut Rng) -> (u9, u9, i10, i10) {
        let extreme = |rng: &mut Rng, low: i32, high: i32| match rng.below(4) {
            0 => low,
            1 => high,
            _ => low + rng.below((high - low + 1) as usize) as i32,
        };
        (
            u9::new(extreme(rng, 0, 511) as u16),
            u9::new(extreme(rng, 0, 511) as u16),
            i10::new(extreme(rng, -512, 511) as i16),
            i10::new(extreme(rng, -512, 511) as i16),
        )
    }

    fn written(c: &DataStruct, skill: &Skill) -> [i32; 4] {
        let (occupation, interest, change, modifier) = match skill {
            Skill::Accounting => Accounting.read(c),
            Skill::Anthropology => Anthropology.read(c),
            Skill::Appraise => Appraise.read(c),
            _ => Archaeology.read(c),
        };
        [
            occupation.value() as i32,
            interest.value() as i32,
            change.value() as i32,
            modifier.value() as i32,
        ]
    }

    #[test]
    fn random_skill_writes_stay_in_their_own_slot_and_fit_the_layout() {
        let skills = [Skill::Accounting, Skill::Anthropology, Skill::Appraise, Skill::Archaeology];
        for seed in 0..2000 {
            let mut rng = Rng::new(seed);
            let mut c = character();
            let mut model = [[0i32; 4]; 4];
            for step in 0..12 {
                let index = rng.below(skills.len());
                let (occupation, interest, change, modifier) = random_skill_values(&mut rng);
                match skills[index] {
                    Skill::Accounting => {
                        Accounting.write(&mut c, occupation, interest, change, modifier)
                    }
                    Skill::Anthropology => {
                        Anthropology.write(&mut c, occupation, interest, change, modifier)
                    }
                    Skill::Appraise => {
                        Appraise.write(&mut c, occupation, interest, change, modifier)
                    }
                    _ => Archaeology.write(&mut c, occupation, interest, change, modifier),
                };
                model[index] = [
                    occupation.value() as i32,
                    interest.value() as i32,
                    change.value() as i32,
                    modifier.value() as i32,
                ];
                for (position, skill) in skills.iter().enumerate() {
                    assert_eq!(
                        written(&c, skill),
                        model[position],
                        "seed {seed} step {step} position {position}"
                    );
                    if let Ok(bytes) = c.get(skill.base_id()) {
                        assert_eq!(bytes.len(), SKILL_LAYOUT.bytes(), "seed {seed}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_skill_owns_a_distinct_ascending_storage_id() {
        let ids: Vec<u32> = Skill::list().iter().map(Skill::base_id).collect();
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_skill_value_shorter_than_its_layout_reads_as_zero() {
        let mut c = character();
        for length in 0..SKILL_LAYOUT.bytes() {
            let _ = c.set(Skill::Accounting.base_id(), &[0xff; 8][..length], None);
            let (occupation, interest, change, modifier) = Accounting.read(&c);
            assert_eq!(
                (occupation.value(), interest.value(), change.value(), modifier.value()),
                (0, 0, 0, 0),
                "length {length}"
            );
        }
    }
}
