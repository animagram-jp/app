use core::array::from_fn;
use crate::Lang;
use crate::list::ListError;
use crate::data_struct::DataStruct;

// ============================================================
// --- ダイス (Dice) ---
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
// --- キャラクター (Character) ---
// ============================================================

pub enum Character {
    Profile,
    Characteristic,
    OtherAttribute,
    Skill,
    Possession,
    Backstory,
    Memo,
}

impl Character {
    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Profile,        Lang::En) => "Profile",
            (Self::Profile,        Lang::Ja) => "プロフィール",
            (Self::Characteristic, Lang::En) => "Characteristics",
            (Self::Characteristic, Lang::Ja) => "能力値",
            (Self::OtherAttribute, Lang::En) => "Other Attributes",
            (Self::OtherAttribute, Lang::Ja) => "ほかの属性",
            (Self::Skill,          Lang::En) => "Skills",
            (Self::Skill,          Lang::Ja) => "技能",
            (Self::Possession,      Lang::En) => "Gear & Possessions",
            (Self::Possession,      Lang::Ja) => "装備と所持品",
            (Self::Backstory,      Lang::En) => "Backstory",
            (Self::Backstory,      Lang::Ja) => "バックストーリー",
            (Self::Memo,           Lang::En) => "Memo",
            (Self::Memo,           Lang::Ja) => "メモ",
        }
    }
    pub const fn id(&self) -> u32 {
        match self {
            Self::Profile        =>  10,  //  10- 15 (6件)
            Self::Characteristic =>  20,  //  20- 28 (9件)
            Self::OtherAttribute =>  30,  //  30- 37 (8件)
            Self::Skill          =>  40,  //  40- 86 (47件)
            Self::Possession     =>  90,  //  90-... (拡張余地)
            Self::Backstory      => 100,  // 100-109 (10件)
            Self::Memo           => 110,  // 110      (1件)
        }
    }
}

// ============================================================
// --- プロフィール (Name, Birthppalce, Pronoun, Occupation, Residence, Age) ---
// ============================================================

#[derive(Clone, Copy)]
pub enum Profile {
    Name, // todo: 「名前」と「Option(呼び方)」の二値構成に拡充。labelは format!"{} ({})"。※Option=noneなら()も出さない
    Birthpalce,
    Pronoun,
    Occupation, // todo: 「ルール上の職業」と「Option(肩書 title)」の二値構成に拡充。 labelは format!"{} ({})"。
    Residence,
    Age,
}

impl Profile {
    pub fn id(&self) -> u32 {
        Character::Profile.id() + match self {
            Self::Name       => 0,
            Self::Birthpalce => 1,
            Self::Pronoun    => 2,
            Self::Occupation => 3,
            Self::Residence  => 4,
            Self::Age        => 5,
        }
    }

    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Name, Lang::En) => "Name",
            (Self::Name, Lang::Ja) => "名前",
            (Self::Birthpalce, Lang::En) => "Birthplace",
            (Self::Birthpalce, Lang::Ja) => "出身",
            (Self::Pronoun, Lang::En) => "Pronoun",
            (Self::Pronoun, Lang::Ja) => "性別",
            (Self::Occupation, Lang::En) => "Occupation",
            (Self::Occupation, Lang::Ja) => "職業",
            (Self::Residence, Lang::En) => "Residence",
            (Self::Residence, Lang::Ja) => "住所",
            (Self::Age, Lang::En) => "Age",
            (Self::Age, Lang::Ja) => "年齢",
        }
    }

    pub fn encode(_name: &str, _alias: Option<&str>) -> Vec<u8> {
        todo!()
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

// --- 職業 (Occupation) --- p.38
pub enum Occupation {
    Activist,
    Antiquarian,
    Artist,
    Athlete,
    Author,
    Clergy,
    Criminal,
    Detective,
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
    PrivateInvestigator,
    Professor,
    Soldier,
    TribeMember,
    Custom(String),
}

impl Occupation {
    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Activist,           Lang::En) => "Activist",
            (Self::Activist,           Lang::Ja) => "活動家",
            (Self::Antiquarian,        Lang::En) => "Antiquarian",
            (Self::Antiquarian,        Lang::Ja) => "古物研究家",
            (Self::Artist,             Lang::En) => "Artist",
            (Self::Artist,             Lang::Ja) => "芸術家",
            (Self::Athlete,            Lang::En) => "Athlete",
            (Self::Athlete,            Lang::Ja) => "スポーツ選手",
            (Self::Author,             Lang::En) => "Author",
            (Self::Author,             Lang::Ja) => "作家",
            (Self::Clergy,             Lang::En) => "Clergy",
            (Self::Clergy,             Lang::Ja) => "聖職者",
            (Self::Criminal,           Lang::En) => "Criminal",
            (Self::Criminal,           Lang::Ja) => "犯罪者",
            (Self::Detective,          Lang::En) => "Detective",
            (Self::Detective,          Lang::Ja) => "刑事",
            (Self::Dilettante,         Lang::En) => "Dilettante",
            (Self::Dilettante,         Lang::Ja) => "ディレッタント",
            (Self::Doctor,             Lang::En) => "Doctor",
            (Self::Doctor,             Lang::Ja) => "医師",
            (Self::Drifter,            Lang::En) => "Drifter",
            (Self::Drifter,            Lang::Ja) => "放浪者",
            (Self::Engineer,           Lang::En) => "Engineer",
            (Self::Engineer,           Lang::Ja) => "技術者",
            (Self::Entertainer,        Lang::En) => "Entertainer",
            (Self::Entertainer,        Lang::Ja) => "芸能人",
            (Self::Farmer,             Lang::En) => "Farmer",
            (Self::Farmer,             Lang::Ja) => "農民",
            (Self::Hacker,             Lang::En) => "Hacker",
            (Self::Hacker,             Lang::Ja) => "ハッカー",
            (Self::Journalist,         Lang::En) => "Journalist",
            (Self::Journalist,         Lang::Ja) => "ジャーナリスト",
            (Self::Lawyer,             Lang::En) => "Lawyer",
            (Self::Lawyer,             Lang::Ja) => "弁護士",
            (Self::Librarian,          Lang::En) => "Librarian",
            (Self::Librarian,          Lang::Ja) => "司書",
            (Self::MilitaryOfficer,    Lang::En) => "Military Officer",
            (Self::MilitaryOfficer,    Lang::Ja) => "士官",
            (Self::Missionary,         Lang::En) => "Missionary",
            (Self::Missionary,         Lang::Ja) => "伝道者",
            (Self::Musician,           Lang::En) => "Musician",
            (Self::Musician,           Lang::Ja) => "音楽家",
            (Self::Parapsychologist,   Lang::En) => "Parapsychologist",
            (Self::Parapsychologist,   Lang::Ja) => "超心理学者",
            (Self::Pilot,              Lang::En) => "Pilot",
            (Self::Pilot,              Lang::Ja) => "パイロット",
            (Self::Police,             Lang::En) => "Police",
            (Self::Police,             Lang::Ja) => "警察官",
            (Self::PrivateInvestigator,Lang::En) => "Private Investigator",
            (Self::PrivateInvestigator,Lang::Ja) => "私立探偵",
            (Self::Professor,          Lang::En) => "Professor",
            (Self::Professor,          Lang::Ja) => "教授",
            (Self::Soldier,            Lang::En) => "Soldier",
            (Self::Soldier,            Lang::Ja) => "兵士",
            (Self::TribeMember,        Lang::En) => "Tribe Member",
            (Self::TribeMember,        Lang::Ja) => "トライブ・メンバー",
            (Self::Custom(s),          _)        => s.as_str(),
        }
    }
}

// ============================================================
// --- 能力値 (Characteristic) ---
// ============================================================

// --- 能力値 (Characteristic) --- p.28
#[derive(Clone, Copy)]
pub enum Characteristic {
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
    pub fn id(&self) -> u32 {
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

    /// [initial: u16 LE][change: i16 LE][modifier: i16 LE] → 6バイト
    pub fn encode(initial: u16, change: i16, modifier: i16) -> Vec<u8> {
        let mut b = Vec::with_capacity(6);
        b.extend_from_slice(&initial.to_le_bytes());
        b.extend_from_slice(&change.to_le_bytes());
        b.extend_from_slice(&modifier.to_le_bytes());
        b
    }

    /// 6バイト → (initial, change, modifier)
    pub fn decode(bytes: &[u8]) -> (u16, i16, i16) {
        let initial  = bytes.get(0..2).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0);
        let change   = bytes.get(2..4).and_then(|b| b.try_into().ok()).map(i16::from_le_bytes).unwrap_or(0);
        let modifier = bytes.get(4..6).and_then(|b| b.try_into().ok()).map(i16::from_le_bytes).unwrap_or(0);
        (initial, change, modifier)
    }

    pub fn value(&self, data: &DataStruct) -> i32 {
        let (initial, change, modifier) = data.get(self.id())
            .map(|b| Self::decode(b))
            .unwrap_or((0, 0, 0));
        (initial as i32 + change as i32 + modifier as i32).max(1)
    }

    pub fn label(&self, lang: Lang) -> &str {
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

    pub fn generate(&self) -> u16 {
        // SIZ / INT / EDU は (2d6+6)×5、それ以外は 3d6×5
        match self {
            Self::Size | Self::Intelligence | Self::Education =>
                dice::roll(&[(2, 6, 6)]) as u16 * 5,
            _ => dice::roll(&[(3, 6, 0)]) as u16 * 5,
        }
    }
}


// ============================================================
// --- ほかの属性 (Other Attribute) ---
// ============================================================

pub enum OtherAttribute {
    HitPoints,
    MagicPoints,
    Luck,
    Sanity,
    Build,
    DamageBonus,
    MoveRate,
    OccupationSkillPoints,
    InterestSkillPoints,
}

impl OtherAttribute {
    pub fn id(&self) -> u32 {
        Character::OtherAttribute.id() + match self {
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

    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::HitPoints,                    _) => "HP",
            (Self::MagicPoints,                  _) => "MP",
            (Self::Luck,                  Lang::En) => "Luck",
            (Self::Luck,                  Lang::Ja) => "幸運",
            (Self::Sanity,                Lang::En) => "Sanity",
            (Self::Sanity,                Lang::Ja) => "正気度",
            (Self::Build,                 Lang::En) => "Build",
            (Self::Build,                 Lang::Ja) => "ビルド",
            (Self::DamageBonus,           Lang::En) => "Damage Bonus",
            (Self::DamageBonus,           Lang::Ja) => "ダメージボーナス",
            (Self::MoveRate,              Lang::En) => "Move Rate",
            (Self::MoveRate,              Lang::Ja) => "移動率 (MOV)",
            (Self::OccupationSkillPoints, Lang::En) => "Occupation Skill Points",
            (Self::OccupationSkillPoints, Lang::Ja) => "職業技能ポイント",
            (Self::InterestSkillPoints,   Lang::En) => "Interest Skill Points",
            (Self::InterestSkillPoints,   Lang::Ja) => "興味技能ポイント",
        }
    }

    pub fn display(&self, data: &DataStruct) -> String {
        match self {
            Self::Build => {
                let build = data.get(self.id()).ok()
                    .and_then(|b| b.first())
                    .map(|&b| b as i8)
                    .unwrap_or(0);
                build.to_string()
            }
            Self::DamageBonus => {
                let db: DamageBonusTuple = data.get(self.id())
                    .map(|b| {
                        let count    = b.first().copied().map(|v| v as i8).unwrap_or(0);
                        let sides    = b.get(1).copied().unwrap_or(0);
                        let modifier = b.get(2).copied().map(|v| v as i8).unwrap_or(0);
                        (count, sides, modifier)
                    })
                    .unwrap_or((0, 0, 0));
                dice::display(&[db])
            }
            _ => String::new(),
        }
    }

    pub fn derive(&self, character: &DataStruct) -> Vec<u8> {
        match self {
            Self::HitPoints => {
                let constitution = Characteristic::Constitution.value(character);
                let size         = Characteristic::Size.value(character);
                let val          = ((constitution + size) / 10) as u8;
                vec![val]
            }
            Self::MagicPoints => {
                let power = Characteristic::Power.value(character);
                let val   = (power / 5) as u8;
                vec![val]
            }
            Self::Sanity => {
                let power = Characteristic::Power.value(character);
                vec![power as u8]
            }
            Self::Build => {
                let strength = Characteristic::Strength.value(character);
                let size     = Characteristic::Size.value(character);
                let build: i8 = match strength + size {
                    2..= 64 => -2,
                   65..= 84 => -1,
                   85..=124 =>  0,
                  125..=164 =>  1,
                  165..=204 =>  2,
                  205..=284 =>  3,
                  285..=364 =>  4,
                  365..=444 =>  5,
                  445..=524 =>  6,
                  // 525..=605 => 7 / +1D6 で80単位で移行も段階変化。
                  _         =>  6,
                };
                vec![build as u8]
            }
            Self::DamageBonus => {
                let build = character.get(Self::Build.id()).ok()
                    .and_then(|b| b.first())
                    .map(|&b| b as i8)
                    .unwrap_or(0);
                let (count, sides, modifier): DamageBonusTuple = match build {
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
                };
                vec![count as u8, sides, modifier as u8]
            }
            Self::MoveRate => {
                let str = Characteristic::Strength.value(character);
                let dex = Characteristic::Dexterity.value(character);
                let siz = Characteristic::Size.value(character);
                let base: i32 = if str > siz && dex > siz { 9 }
                           else if str < siz && dex < siz  { 7 }
                           else                             { 8 };
                let age = character.get(Profile::Age.id()).ok()
                    .and_then(|b| b.first())
                    .copied()
                    .unwrap_or(0);
                let age_penalty: i32 = match age {
                    40..=49 => 1,
                    50..=59 => 2,
                    60..=69 => 3,
                    70..=79 => 4,
                    80..    => 5,
                    _       => 0,
                };
                vec![(base - age_penalty).max(0) as u8]
            }
            _ => vec![],
        }
    }
}

// ============================================================
// --- Derived ---
// ============================================================

// --- 生活水準 (Standard of Living) ---
pub enum StandardOfLiving {
    Pauper,
    Poor,
    Average,
    Wealthy,
    Rich,
    SuperRich,
}

impl StandardOfLiving {
    pub fn display(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Pauper,    Lang::Ja) => "無一文",
            (Self::Pauper,    Lang::En) => "Pauper",
            (Self::Poor,      Lang::Ja) => "貧乏",
            (Self::Poor,      Lang::En) => "Poor",
            (Self::Average,   Lang::Ja) => "平均",
            (Self::Average,   Lang::En) => "Average",
            (Self::Wealthy,   Lang::Ja) => "裕福",
            (Self::Wealthy,   Lang::En) => "Wealthy",
            (Self::Rich,      Lang::Ja) => "富豪",
            (Self::Rich,      Lang::En) => "Rich",
            (Self::SuperRich, Lang::Ja) => "大富豪",
            (Self::SuperRich, Lang::En) => "Super Rich",
        }
    }
}

// --- 年齢カテゴリ (AgeCategory) ---
enum AgeCategory {
    Teen,    // 15-19: STR/SIZ合計-5、EDU-5、幸運再ロール（高い方）
    Young,   // 20-39: EDU改善1回、修正なし
    Middle,  // 40-49: EDU改善2回、STR/CON/DEX合計-5、 APP-5、 MOV-1
    Senior,  // 50-59: EDU改善3回、STR/CON/DEX合計-10、APP-10、MOV-2
    Elderly, // 60-69: EDU改善4回、STR/CON/DEX合計-20、APP-15、MOV-3
    Old,     // 70-79: EDU改善4回、STR/CON/DEX合計-40、APP-20、MOV-4
    Ancient, // 80+  : EDU改善4回、STR/CON/DEX合計-80、APP-25、MOV-5
}

impl AgeCategory {
    pub fn from_age(age: u8) -> Self {
        match age {
            15..=19 => Self::Teen,
            20..=39 => Self::Young,
            40..=49 => Self::Middle,
            50..=59 => Self::Senior,
            60..=69 => Self::Elderly,
            70..=79 => Self::Old,
            _       => Self::Ancient,
        }
    }

    // STR/CON/DEX から合計で差し引く点数（Teen は STR/SIZ から差し引く）
    pub fn phys_deduction(&self) -> u8 {
        match self {
            Self::Teen    =>  5, // STR+SIZ から差し引く（Teen 専用ルール）
            Self::Young   =>  0,
            Self::Middle  =>  5,
            Self::Senior  => 10,
            Self::Elderly => 20,
            Self::Old     => 40,
            Self::Ancient => 80,
        }
    }

    // APP からの固定減算値
    pub fn app_deduction(&self) -> u8 {
        match self {
            Self::Teen    =>  0,
            Self::Young   =>  0,
            Self::Middle  =>  5,
            Self::Senior  => 10,
            Self::Elderly => 15,
            Self::Old     => 20,
            Self::Ancient => 25,
        }
    }

    // EDU 改善チェック回数（成功すれば EDU +1D10、上限 99）
    pub fn edu_improvement_checks(&self) -> u8 {
        match self {
            Self::Teen    => 0,
            Self::Young   => 1,
            Self::Middle  => 2,
            Self::Senior  => 3,
            Self::Elderly => 4,
            Self::Old     => 4,
            Self::Ancient => 4,
        }
    }

    // Teen のみ特殊ルール（STR/SIZ差し引き・EDU-5・幸運再ロール）
    pub fn is_teen(&self) -> bool {
        matches!(self, Self::Teen)
    }

    pub fn display(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Teen,    Lang::Ja) => "10代 (15-19)",
            (Self::Teen,    Lang::En) => "Teen (15-19)",
            (Self::Young,   Lang::Ja) => "若年 (20-39)",
            (Self::Young,   Lang::En) => "Young Adult (20-39)",
            (Self::Middle,  Lang::Ja) => "中年 (40-49)",
            (Self::Middle,  Lang::En) => "Middle-Aged (40-49)",
            (Self::Senior,  Lang::Ja) => "熟年 (50-59)",
            (Self::Senior,  Lang::En) => "Senior (50-59)",
            (Self::Elderly, Lang::Ja) => "老年 (60-69)",
            (Self::Elderly, Lang::En) => "Elderly (60-69)",
            (Self::Old,     Lang::Ja) => "高齢 (70-79)",
            (Self::Old,     Lang::En) => "Old (70-79)",
            (Self::Ancient, Lang::Ja) => "超高齢 (80+)",
            (Self::Ancient, Lang::En) => "Very Old (80+)",
        }
    }
}

// ============================================================
// --- スキル (Skill) ---
// ============================================================

// --- 芸術/製作 専門分野 (Art/Craft Specialization)  --- p.62
#[derive(Clone)]
pub enum ArtCraftSpec {
    None,
    Acting,       // 演劇
    Barber,       // 理容
    Calligraphy,  // 書道
    Carpentry,    // 大工仕事
    Cook,         // 料理
    Dancing,      // ダンス
    FineArt,      // 絵画
    Forgery,      // 文書偽造
    Writing,      // 執筆
    Photography,  // 写真術
    Pottery,      // 陶芸
    Sculpting,    // 彫刻
    Custom1(String), Custom2(String), Custom3(String), Custom4(String),
}

impl ArtCraftSpec {
    pub fn list() -> &'static [Self] {
        &[
            Self::Acting, Self::Barber, Self::Calligraphy, Self::Writing, Self::Carpentry,
            Self::Cook, Self::Dancing, Self::FineArt,
            Self::Forgery, Self::Photography, Self::Pottery, Self::Sculpting,
        ]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::None        => unreachable!(),
            Self::Acting      =>  0,
            Self::Barber      =>  1,
            Self::Calligraphy =>  2,
            Self::Writing     =>  3,
            Self::Carpentry   =>  4,
            Self::Cook        =>  5,
            Self::Dancing     =>  6,
            Self::FineArt     =>  7,
            Self::Forgery     =>  8,
            Self::Photography =>  9,
            Self::Pottery     => 10,
            Self::Sculpting   => 11,
            Self::Custom1(_)  => 12,
            Self::Custom2(_)  => 13,
            Self::Custom3(_)  => 14,
            Self::Custom4(_)  => 15,
        }
    }

    pub fn base_value(&self) -> u16 { 5 }

    pub fn label(&self, lang: Lang) -> Option<&str> {
        match (self, lang) {
            (Self::None,        _)        => None,
            (Self::Acting,      Lang::Ja) => Some("演劇"),
            (Self::Acting,      Lang::En) => Some("Acting"),
            (Self::Barber,      Lang::Ja) => Some("理容"),
            (Self::Barber,      Lang::En) => Some("Barber"),
            (Self::Calligraphy, Lang::Ja) => Some("書道"),
            (Self::Calligraphy, Lang::En) => Some("Calligraphy"),
            (Self::Carpentry,   Lang::Ja) => Some("大工仕事"),
            (Self::Carpentry,   Lang::En) => Some("Carpentry"),
            (Self::Cook,        Lang::Ja) => Some("料理"),
            (Self::Cook,        Lang::En) => Some("Cook"),
            (Self::Dancing,     Lang::Ja) => Some("ダンス"),
            (Self::Dancing,     Lang::En) => Some("Dancing"),
            (Self::FineArt,     Lang::Ja) => Some("絵画"),
            (Self::FineArt,     Lang::En) => Some("Fine Art"),
            (Self::Forgery,     Lang::Ja) => Some("文書偽造"),
            (Self::Writing,     Lang::Ja) => Some("執筆"),
            (Self::Writing,     Lang::En) => Some("Writing"),
            (Self::Forgery,     Lang::En) => Some("Forgery"),
            (Self::Photography, Lang::Ja) => Some("写真術"),
            (Self::Photography, Lang::En) => Some("Photography"),
            (Self::Pottery,     Lang::Ja) => Some("陶芸"),
            (Self::Pottery,     Lang::En) => Some("Pottery"),
            (Self::Sculpting,   Lang::Ja) => Some("彫刻"),
            (Self::Sculpting,   Lang::En) => Some("Sculpting"),
            (Self::Custom1(s) | Self::Custom2(s) | Self::Custom3(s) | Self::Custom4(s), _) => Some(s.as_str()),
        }
    }
}

// --- 近接戦闘 専門分野 (Fighting Specialization) --- p.61
#[derive(Clone)]
pub enum FightingSpec {
    None,
    Axe,          // 斧          15%
    Brawl,        // 格闘        25%
    Chainsaw,     // チェーンソー  10%
    Flail,        // フレイル     10%
    Garrote,      // 絞殺ひも     15%
    Spear,        // 槍          20%
    Sword,        // 刀剣        20%
    Whip,         // 鞭          05%
    Custom1 { name: String, base_value: u16 },
    Custom2 { name: String, base_value: u16 },
    Custom3 { name: String, base_value: u16 },
    Custom4 { name: String, base_value: u16 },
}


impl FightingSpec {
    pub fn list() -> &'static [Self] {
        &[Self::Axe, Self::Brawl, Self::Chainsaw, Self::Flail,
          Self::Garrote, Self::Spear, Self::Sword, Self::Whip]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::None           => unreachable!(),
            Self::Axe            => 0,
            Self::Brawl          => 1,
            Self::Chainsaw       => 2,
            Self::Flail          => 3,
            Self::Garrote        => 4,
            Self::Spear          => 5,
            Self::Sword          => 6,
            Self::Whip           => 7,
            Self::Custom1 { .. } => 8,
            Self::Custom2 { .. } => 9,
            Self::Custom3 { .. } => 10,
            Self::Custom4 { .. } => 11,
        }
    }

    pub fn base_value(&self) -> u16 {
        match self {
            Self::None                              =>  0,
            Self::Axe                               => 15,
            Self::Brawl                             => 25,
            Self::Chainsaw                          => 10,
            Self::Flail                             => 10,
            Self::Garrote                           => 15,
            Self::Spear                             => 20,
            Self::Sword                             => 20,
            Self::Whip                              =>  5,
            Self::Custom1 { base_value, .. }
            | Self::Custom2 { base_value, .. }
            | Self::Custom3 { base_value, .. }
            | Self::Custom4 { base_value, .. }      => *base_value,
        }
    }

    pub fn label(&self, lang: Lang) -> Option<&str> {
        match (self, lang) {
            (Self::None,     _)        => None,
            (Self::Axe,      Lang::Ja) => Some("斧"),
            (Self::Axe,      Lang::En) => Some("Axe"),
            (Self::Brawl,    Lang::Ja) => Some("格闘"),
            (Self::Brawl,    Lang::En) => Some("Brawl"),
            (Self::Chainsaw, Lang::Ja) => Some("チェーンソー"),
            (Self::Chainsaw, Lang::En) => Some("Chainsaw"),
            (Self::Flail,    Lang::Ja) => Some("フレイル"),
            (Self::Flail,    Lang::En) => Some("Flail"),
            (Self::Garrote,  Lang::Ja) => Some("絞殺ひも"),
            (Self::Garrote,  Lang::En) => Some("Garrote"),
            (Self::Spear,    Lang::Ja) => Some("槍"),
            (Self::Spear,    Lang::En) => Some("Spear"),
            (Self::Sword,    Lang::Ja) => Some("刀剣"),
            (Self::Sword,    Lang::En) => Some("Sword"),
            (Self::Whip,     Lang::Ja) => Some("鞭"),
            (Self::Whip,     Lang::En) => Some("Whip"),
            (Self::Custom1 { name, .. } | Self::Custom2 { name, .. }
            | Self::Custom3 { name, .. } | Self::Custom4 { name, .. }, _) => Some(name.as_str()),
        }
    }
}

// --- 射撃 専門分野 (Firearms Specialization) --- p.64
#[derive(Clone)]
pub enum FirearmsSpec {
    None,
    Bow,           // 弓                   15%
    Handgun,       // 拳銃                 20%
    HeavyWeapons,  // 重火器               10%
    MachineGun,    // 機関銃               10%
    RifleShotgun,  // ライフル/ショットガン  25%
    SubmachineGun, // サブマシンガン         15%
    Custom1 { name: String, base_value: u16 },
    Custom2 { name: String, base_value: u16 },
    Custom3 { name: String, base_value: u16 },
    Custom4 { name: String, base_value: u16 },
}

impl FirearmsSpec {
    pub fn list() -> &'static [Self] {
        &[Self::Bow, Self::Handgun, Self::HeavyWeapons,
          Self::MachineGun, Self::RifleShotgun, Self::SubmachineGun]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::None           => unreachable!(),
            Self::Bow            => 0,
            Self::Handgun        => 1,
            Self::HeavyWeapons   => 2,
            Self::MachineGun     => 3,
            Self::RifleShotgun   => 4,
            Self::SubmachineGun  => 5,
            Self::Custom1 { .. } => 6,
            Self::Custom2 { .. } => 7,
            Self::Custom3 { .. } => 8,
            Self::Custom4 { .. } => 9,
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
            Self::Custom1 { base_value, .. }
            | Self::Custom2 { base_value, .. }
            | Self::Custom3 { base_value, .. }
            | Self::Custom4 { base_value, .. }      => *base_value,
        }
    }

    pub fn label(&self, lang: Lang) -> Option<&str> {
        match (self, lang) {
            (Self::None,          _)        => None,
            (Self::Bow,           Lang::Ja) => Some("弓"),
            (Self::Bow,           Lang::En) => Some("Bow"),
            (Self::Handgun,       Lang::Ja) => Some("拳銃"),
            (Self::Handgun,       Lang::En) => Some("Handgun"),
            (Self::HeavyWeapons,  Lang::Ja) => Some("重火器"),
            (Self::HeavyWeapons,  Lang::En) => Some("Heavy Weapons"),
            (Self::MachineGun,    Lang::Ja) => Some("機関銃"),
            (Self::MachineGun,    Lang::En) => Some("Machine Gun"),
            (Self::RifleShotgun,  Lang::Ja) => Some("ライフル/ショットガン"),
            (Self::RifleShotgun,  Lang::En) => Some("Rifle/Shotgun"),
            (Self::SubmachineGun, Lang::Ja) => Some("サブマシンガン"),
            (Self::SubmachineGun, Lang::En) => Some("Submachine Gun"),
            (Self::Custom1 { name, .. } | Self::Custom2 { name, .. }
            | Self::Custom3 { name, .. } | Self::Custom4 { name, .. }, _) => Some(name.as_str()),
        }
    }
}

// --- ほかの言語 専門分野 (Language Other Specialization) ---
#[derive(Clone)]
pub enum LanguageSpec {
    Custom1(String), Custom2(String), Custom3(String), Custom4(String),
}

impl LanguageSpec {
    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::Custom1(_) => 0,
            Self::Custom2(_) => 1,
            Self::Custom3(_) => 2,
            Self::Custom4(_) => 3,
        }
    }

    pub fn label(&self, _lang: Lang) -> &str {
        match self {
            Self::Custom1(s) | Self::Custom2(s)
            | Self::Custom3(s) | Self::Custom4(s) => s.as_str(),
        }
    }
}

// --- 操縦 専門分野 (Pilot Specialization) --- p.67
#[derive(Clone)]
pub enum PilotSpec {
    None,
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
    Custom1(String), Custom2(String), Custom3(String), Custom4(String),
}

impl PilotSpec {
    pub fn list() -> &'static [Self] {
        &[Self::Boat, Self::SteamShip, Self::Sailboat, Self::CivilProp,
          Self::Balloon, Self::Dirigible, Self::CivilJet, Self::Airliner,
          Self::JetFighter, Self::Helicopter]
    }

    pub fn id(&self, base: u32) -> u32 {
        base + match self {
            Self::None       => unreachable!(),
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
            Self::Custom1(_) => 10,
            Self::Custom2(_) => 11,
            Self::Custom3(_) => 12,
            Self::Custom4(_) => 13,
        }
    }

    pub fn base_value(&self) -> u16 { 1 }

    pub fn label(&self, lang: Lang) -> Option<&str> {
        match (self, lang) {
            (Self::None,       _)        => None,
            // --- 両時代共通 ---
            (Self::Boat,       Lang::Ja) => Some("ボート"),
            (Self::Boat,       Lang::En) => Some("Boat"),
            (Self::SteamShip,  Lang::Ja) => Some("汽船"),
            (Self::SteamShip,  Lang::En) => Some("Steam Ship"),
            (Self::Sailboat,   Lang::Ja) => Some("帆船"),
            (Self::Sailboat,   Lang::En) => Some("Sailboat"),
            (Self::CivilProp,  Lang::Ja) => Some("民間プロペラ機"),
            (Self::CivilProp,  Lang::En) => Some("Civil Prop"),
            // --- 1920s のみ ---
            (Self::Balloon,    Lang::Ja) => Some("気球"),
            (Self::Balloon,    Lang::En) => Some("Balloon"),
            (Self::Dirigible,  Lang::Ja) => Some("飛行船"),
            (Self::Dirigible,  Lang::En) => Some("Dirigible"),
            // --- Modern (1990s) のみ ---
            (Self::CivilJet,   Lang::Ja) => Some("民間ジェット機"),
            (Self::CivilJet,   Lang::En) => Some("Civil Jet"),
            (Self::Airliner,   Lang::Ja) => Some("旅客機"),
            (Self::Airliner,   Lang::En) => Some("Airliner"),
            (Self::JetFighter, Lang::Ja) => Some("ジェット戦闘機"),
            (Self::JetFighter, Lang::En) => Some("Jet Fighter"),
            (Self::Helicopter, Lang::Ja) => Some("ヘリコプター"),
            (Self::Helicopter, Lang::En) => Some("Helicopter"),
            (Self::Custom1(s) | Self::Custom2(s)
            | Self::Custom3(s) | Self::Custom4(s), _) => Some(s.as_str()),
        }
    }
}

// --- 科学 専門分野 (Science Specialization) --- p.59
#[derive(Clone)]
pub enum ScienceSpec {
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
    Custom1(String), Custom2(String), Custom3(String), Custom4(String),
}


impl ScienceSpec {
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
            Self::Custom1(_)   => 13,
            Self::Custom2(_)   => 14,
            Self::Custom3(_)   => 15,
            Self::Custom4(_)   => 16,
        }
    }

    pub fn base_value(&self) -> u16 { 1 }

    pub fn label(&self, lang: Lang) -> Option<&str> {
        match (self, lang) {
            (Self::None,         _)        => None,
            (Self::Astronomy,    Lang::Ja) => Some("天文学"),
            (Self::Astronomy,    Lang::En) => Some("Astronomy"),
            (Self::Biology,      Lang::Ja) => Some("生物学"),
            (Self::Biology,      Lang::En) => Some("Biology"),
            (Self::Botany,       Lang::Ja) => Some("植物学"),
            (Self::Botany,       Lang::En) => Some("Botany"),
            (Self::Chemistry,    Lang::Ja) => Some("化学"),
            (Self::Chemistry,    Lang::En) => Some("Chemistry"),
            (Self::Cryptography, Lang::Ja) => Some("暗号学"),
            (Self::Cryptography, Lang::En) => Some("Cryptography"),
            (Self::Engineering,  Lang::Ja) => Some("工学"),
            (Self::Engineering,  Lang::En) => Some("Engineering"),
            (Self::Forensics,    Lang::Ja) => Some("法医学"),
            (Self::Forensics,    Lang::En) => Some("Forensics"),
            (Self::Geology,      Lang::Ja) => Some("地質学"),
            (Self::Geology,      Lang::En) => Some("Geology"),
            (Self::Mathematics,  Lang::Ja) => Some("数学"),
            (Self::Mathematics,  Lang::En) => Some("Mathematics"),
            (Self::Meteorology,  Lang::Ja) => Some("気象学"),
            (Self::Meteorology,  Lang::En) => Some("Meteorology"),
            (Self::Pharmacy,     Lang::Ja) => Some("薬学"),
            (Self::Pharmacy,     Lang::En) => Some("Pharmacy"),
            (Self::Physics,      Lang::Ja) => Some("物理学"),
            (Self::Physics,      Lang::En) => Some("Physics"),
            (Self::Zoology,      Lang::Ja) => Some("動物学"),
            (Self::Zoology,      Lang::En) => Some("Zoology"),
            (Self::Custom1(s) | Self::Custom2(s)
            | Self::Custom3(s) | Self::Custom4(s), _) => Some(s.as_str()),
        }
    }
}

// --- サバイバル 専門分野 (Survival Specialization) --- p.63
#[derive(Clone)]
pub enum SurvivalSpec {
    None,
    Arctic,
    Desert,
    Sea,
    Custom1(String), Custom2(String), Custom3(String), Custom4(String),
}

impl SurvivalSpec {
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
            (Self::Arctic,   Lang::Ja) => Some("極地"),
            (Self::Arctic,   Lang::En) => Some("Arctic"),
            (Self::Desert,   Lang::Ja) => Some("砂漠"),
            (Self::Desert,   Lang::En) => Some("Desert"),
            (Self::Sea,      Lang::Ja) => Some("海"),
            (Self::Sea,      Lang::En) => Some("Sea"),
            (Self::Custom1(s) | Self::Custom2(s)
            | Self::Custom3(s) | Self::Custom4(s), _) => Some(s.as_str()),
        }
    }
}

pub const SPEC_CUSTOM_SLOTS: usize = 4;

// --- スキル (Skill) --- p.54
#[derive(Clone)]
pub enum Skill {
    Accounting,
    Anthropology,
    Archaeology,
    Appraise,
    ArtCraft(ArtCraftSpec),
    Charm,
    Climb,
    ComputerUse,
    CreditRating,
    CthulhuMythos,
    Disguise,
    Dodge,
    DriveAuto,
    ElecRepair,
    Electronics,
    FastTalk,
    Fighting(FightingSpec),
    Firearms(FirearmsSpec),
    FirstAid,
    History,
    Intimidate,
    Jump,
    LanguageOther(LanguageSpec),
    LanguageOwn,
    Law,
    LibraryUse,
    Listen,
    Locksmith,
    MechRepair,
    Medicine,
    NaturalWorld,
    Navigate,
    Occult,
    Persuade,
    Pilot(PilotSpec),
    Psychoanalysis,
    Psychology,
    Ride,
    Science(ScienceSpec),
    SleightOfHand,
    SpotHidden,
    Stealth,
    Survival(SurvivalSpec),
    Swim,
    Throw,
    Track,
    // 技能名+専門分野 完全自由記入（キャラシ空白欄に対応）
    // slot: 0..SPEC_CUSTOM_SLOTS-1
    Custom { slot: usize, name: String, spec: Option<String> },
}

impl Skill {
    /// UIのrow管理用。spec持ちは代表1エントリ、Customスロットは1エントリ。
    /// IDマッピングには Skill::id() / SpecEnum::id() を直接使う。
    pub fn list() -> Vec<Skill> {
        vec![
            Self::Accounting,
            Self::Anthropology,
            Self::Archaeology,
            Self::Appraise,
            Self::ArtCraft(ArtCraftSpec::None),
            Self::Charm,
            Self::Climb,
            Self::ComputerUse,
            Self::CreditRating,
            Self::CthulhuMythos,
            Self::Disguise,
            Self::Dodge,
            Self::DriveAuto,
            Self::ElecRepair,
            Self::Electronics,
            Self::FastTalk,
            Self::Fighting(FightingSpec::None),
            Self::Firearms(FirearmsSpec::None),
            Self::FirstAid,
            Self::History,
            Self::Intimidate,
            Self::Jump,
            Self::LanguageOther(LanguageSpec::Custom1(String::new())), // 全部自由記入
            Self::LanguageOwn,
            Self::Law,
            Self::LibraryUse,
            Self::Listen,
            Self::Locksmith,
            Self::MechRepair,
            Self::Medicine,
            Self::NaturalWorld,
            Self::Navigate,
            Self::Occult,
            Self::Persuade,
            Self::Pilot(PilotSpec::None),
            Self::Psychoanalysis,
            Self::Psychology,
            Self::Ride,
            Self::Science(ScienceSpec::None),
            Self::SleightOfHand,
            Self::SpotHidden,
            Self::Stealth,
            Self::Survival(SurvivalSpec::None),
            Self::Swim,
            Self::Throw,
            Self::Track,
            Self::Custom { slot: 0, name: String::new(), spec: None },
        ]
    }

    pub fn id(&self) -> u32 {
        // spec有り: Specにオフセットを伝播してSpec内でIDを確定する
        // spec無し: base+100 以降に配置（Specオフセット域 0..100 と衝突しない）
        let base = Character::Skill.id();
        match self {
            Self::ArtCraft(spec)      => spec.id(base +   0),  //   0.. 16 (幅17)
            Self::Fighting(spec)      => spec.id(base +  17),  //  17.. 28 (幅12)
            Self::Firearms(spec)      => spec.id(base +  29),  //  29.. 38 (幅10)
            Self::LanguageOther(spec) => spec.id(base +  39),  //  39.. 42 (幅4)
            Self::Pilot(spec)         => spec.id(base +  43),  //  43.. 56 (幅14)
            Self::Science(spec)       => spec.id(base +  57),  //  57.. 73 (幅17)
            Self::Survival(spec)      => spec.id(base +  74),  //  74.. 80 (幅7)
            // spec無しスキル: 100 以降
            Self::Accounting          => base + 100,
            Self::Anthropology        => base + 101,
            Self::Archaeology         => base + 102,
            Self::Appraise            => base + 103,
            Self::Charm               => base + 104,
            Self::Climb               => base + 105,
            Self::ComputerUse         => base + 106,
            Self::CreditRating        => base + 107,
            Self::CthulhuMythos       => base + 108,
            Self::Disguise            => base + 109,
            Self::Dodge               => base + 110,
            Self::DriveAuto           => base + 111,
            Self::ElecRepair          => base + 112,
            Self::Electronics         => base + 113,
            Self::FastTalk            => base + 114,
            Self::FirstAid            => base + 115,
            Self::History             => base + 116,
            Self::Intimidate          => base + 117,
            Self::Jump                => base + 118,
            Self::LanguageOwn         => base + 119,
            Self::Law                 => base + 120,
            Self::LibraryUse          => base + 121,
            Self::Listen              => base + 122,
            Self::Locksmith           => base + 123,
            Self::MechRepair          => base + 124,
            Self::Medicine            => base + 125,
            Self::NaturalWorld        => base + 126,
            Self::Navigate            => base + 127,
            Self::Occult              => base + 128,
            Self::Persuade            => base + 129,
            Self::Psychoanalysis      => base + 130,
            Self::Psychology          => base + 131,
            Self::Ride                => base + 132,
            Self::SleightOfHand       => base + 133,
            Self::SpotHidden          => base + 134,
            Self::Stealth             => base + 135,
            Self::Swim                => base + 136,
            Self::Throw               => base + 137,
            Self::Track               => base + 138,
            // Custom: 140.. (SPEC_CUSTOM_SLOTS 個)
            Self::Custom { slot, .. } => base + 140 + *slot as u32,
        }
    }

    /// [specialization: u8][initial: u8][occupation: u16 LE][interest: u16 LE][change: i16 LE][modifier: i16 LE][input_len: u16 LE][input: utf8...]
    pub fn encode(specialization: u8, initial: u8, occupation: u16, interest: u16, change: i16, modifier: i16, input: Option<&str>) -> Vec<u8> {
        let input_bytes = input.unwrap_or("").as_bytes();
        let mut b = Vec::with_capacity(10 + input_bytes.len());
        b.push(specialization);
        b.push(initial);
        b.extend_from_slice(&occupation.to_le_bytes());
        b.extend_from_slice(&interest.to_le_bytes());
        b.extend_from_slice(&change.to_le_bytes());
        b.extend_from_slice(&modifier.to_le_bytes());
        b.extend_from_slice(&(input_bytes.len() as u16).to_le_bytes());
        b.extend_from_slice(input_bytes);
        b
    }

    /// → (specialization, initial, occupation, interest, change, modifier, input)
    pub fn decode(bytes: &[u8]) -> (u8, u8, u16, u16, i16, i16, String) {
        let specialization = bytes.first().copied().unwrap_or(0);
        let initial        = bytes.get(1).copied().unwrap_or(0);
        let occupation     = bytes.get(2..4).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0);
        let interest       = bytes.get(4..6).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0);
        let change         = bytes.get(6..8).and_then(|b| b.try_into().ok()).map(i16::from_le_bytes).unwrap_or(0);
        let modifier       = bytes.get(8..10).and_then(|b| b.try_into().ok()).map(i16::from_le_bytes).unwrap_or(0);
        let input_len      = bytes.get(10..12).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0) as usize;
        let input          = bytes.get(12..12 + input_len)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        (specialization, initial, occupation, interest, change, modifier, input)
    }

    pub fn value(&self, data: &DataStruct) -> i32 {
        let (_, _, occupation, interest, change, modifier, _) = data.get(self.id())
            .map(|b| Self::decode(b))
            .unwrap_or((0, 0, 0, 0, 0, 0, String::new()));
        (self.base_value() as i32 + occupation as i32 + interest as i32 + change as i32 + modifier as i32).max(1)
    }

    pub fn base_value(&self) -> u16 {
        match self {
            Self::Accounting           =>  5,
            Self::Anthropology         =>  1,
            Self::Archaeology          =>  1,
            Self::Appraise             =>  5,
            Self::ArtCraft(spec)       => spec.base_value(),
            Self::Charm                => 15,
            Self::Climb                => 20,
            Self::ComputerUse          =>  5,
            Self::CreditRating         =>  0,
            Self::CthulhuMythos        =>  0,
            Self::Disguise             =>  5,
            Self::Dodge                =>  0, // derived: DEX / 2
            Self::DriveAuto            => 20,
            Self::ElecRepair           => 10,
            Self::Electronics          =>  1,
            Self::FastTalk             =>  5,
            Self::Fighting(spec)       => spec.base_value(),
            Self::Firearms(spec)       => spec.base_value(),
            Self::FirstAid             => 30,
            Self::History              =>  5,
            Self::Intimidate           => 15,
            Self::Jump                 => 20,
            Self::LanguageOther(_)     =>  1,
            Self::LanguageOwn          =>  0, // derived: EDU
            Self::Law                  =>  5,
            Self::LibraryUse           => 20,
            Self::Listen               => 20,
            Self::Locksmith            =>  1,
            Self::MechRepair           => 10,
            Self::Medicine             =>  1,
            Self::NaturalWorld         => 10,
            Self::Navigate             => 10,
            Self::Occult               =>  5,
            Self::Persuade             => 10,
            Self::Pilot(spec)          => spec.base_value(),
            Self::Psychoanalysis       =>  1,
            Self::Psychology           => 10,
            Self::Ride                 =>  5,
            Self::Science(spec)        => spec.base_value(),
            Self::SleightOfHand        => 10,
            Self::SpotHidden           => 25,
            Self::Stealth              => 20,
            Self::Survival(spec)       => spec.base_value(),
            Self::Swim                 => 20,
            Self::Throw                => 20,
            Self::Track                => 10,
            Self::Custom { .. }        =>  0,
        }
    }

    pub fn sum(&self, occ: u16, int: u16, bonus: i32) -> i32 {
        self.base_value() as i32 + occ as i32 + int as i32 + bonus
    }

    // Characteristic依存で初期値が決まるスキルについて、依存先を返す。
    // 呼び出し側がCharacteristic値を取得し、スキルのbase_valueを上書きする責務を持つ。
    // Dodge: DEX/2、LanguageOwn: EDU そのまま。
    pub fn characteristic_base(&self) -> Option<(Characteristic, fn(u16) -> u16)> {
        match self {
            Self::Dodge     => Some((Characteristic::Dexterity, |dex| dex / 2)),
            Self::LanguageOwn => Some((Characteristic::Education, |edu| edu)),
            _ => None,
        }
    }

    pub fn label(&self, lang: Lang) -> String {
        match (self, lang) {
            (Self::Accounting,           Lang::Ja) => "経理".into(),
            (Self::Accounting,           Lang::En) => "Accounting".into(),
            (Self::Anthropology,         Lang::Ja) => "人類学".into(),
            (Self::Anthropology,         Lang::En) => "Anthropology".into(),
            (Self::Archaeology,          Lang::Ja) => "考古学".into(),
            (Self::Archaeology,          Lang::En) => "Archaeology".into(),
            (Self::Appraise,             Lang::Ja) => "鑑定".into(),
            (Self::Appraise,             Lang::En) => "Appraise".into(),
            (Self::ArtCraft(spec),       _)        => match spec.label(lang) { Some(s) => format!("芸術/製作 ({s})"), None => "芸術/製作".into() },
            (Self::Charm,                Lang::Ja) => "魅惑".into(),
            (Self::Charm,                Lang::En) => "Charm".into(),
            (Self::Climb,                Lang::Ja) => "登攀".into(),
            (Self::Climb,                Lang::En) => "Climb".into(),
            (Self::ComputerUse,          Lang::Ja) => "コンピューター".into(),
            (Self::ComputerUse,          Lang::En) => "Computer Use".into(),
            (Self::CreditRating,         Lang::Ja) => "信用".into(),
            (Self::CreditRating,         Lang::En) => "Credit Rating".into(),
            (Self::CthulhuMythos,        Lang::Ja) => "クトゥルフ神話".into(),
            (Self::CthulhuMythos,        Lang::En) => "Cthulhu Mythos".into(),
            (Self::Disguise,             Lang::Ja) => "変装".into(),
            (Self::Disguise,             Lang::En) => "Disguise".into(),
            (Self::Dodge,                Lang::Ja) => "回避".into(),
            (Self::Dodge,                Lang::En) => "Dodge".into(),
            (Self::DriveAuto,            Lang::Ja) => "運転（自動車）".into(),
            (Self::DriveAuto,            Lang::En) => "Drive Auto".into(),
            (Self::ElecRepair,           Lang::Ja) => "電気修理".into(),
            (Self::ElecRepair,           Lang::En) => "Elec. Repair".into(),
            (Self::Electronics,          Lang::Ja) => "電子工学".into(),
            (Self::Electronics,          Lang::En) => "Electronics".into(),
            (Self::FastTalk,             Lang::Ja) => "言いくるめ".into(),
            (Self::FastTalk,             Lang::En) => "Fast Talk".into(),
            (Self::Fighting(spec),       _)        => match spec.label(lang) { Some(s) => format!("近接戦闘 ({s})"), None => "近接戦闘".into() },
            (Self::Firearms(spec),       _)        => match spec.label(lang) { Some(s) => format!("射撃 ({s})"),    None => "射撃".into() },
            (Self::FirstAid,             Lang::Ja) => "応急手当".into(),
            (Self::FirstAid,             Lang::En) => "First Aid".into(),
            (Self::History,              Lang::Ja) => "歴史".into(),
            (Self::History,              Lang::En) => "History".into(),
            (Self::Intimidate,           Lang::Ja) => "威圧".into(),
            (Self::Intimidate,           Lang::En) => "Intimidate".into(),
            (Self::Jump,                 Lang::Ja) => "跳躍".into(),
            (Self::Jump,                 Lang::En) => "Jump".into(),
            (Self::LanguageOther(spec),  _)        => { let s = spec.label(lang); if s.is_empty() { "ほかの言語".into() } else { format!("ほかの言語 ({s})") } },
            (Self::LanguageOwn,          Lang::Ja) => "母国語".into(),
            (Self::LanguageOwn,          Lang::En) => "Language (Own)".into(),
            (Self::Law,                  Lang::Ja) => "法律".into(),
            (Self::Law,                  Lang::En) => "Law".into(),
            (Self::LibraryUse,           Lang::Ja) => "図書館".into(),
            (Self::LibraryUse,           Lang::En) => "Library Use".into(),
            (Self::Listen,               Lang::Ja) => "聞き耳".into(),
            (Self::Listen,               Lang::En) => "Listen".into(),
            (Self::Locksmith,            Lang::Ja) => "鍵開け".into(),
            (Self::Locksmith,            Lang::En) => "Locksmith".into(),
            (Self::MechRepair,           Lang::Ja) => "機械修理".into(),
            (Self::MechRepair,           Lang::En) => "Mech. Repair".into(),
            (Self::Medicine,             Lang::Ja) => "医学".into(),
            (Self::Medicine,             Lang::En) => "Medicine".into(),
            (Self::NaturalWorld,         Lang::Ja) => "自然".into(),
            (Self::NaturalWorld,         Lang::En) => "Natural World".into(),
            (Self::Navigate,             Lang::Ja) => "ナビゲート".into(),
            (Self::Navigate,             Lang::En) => "Navigate".into(),
            (Self::Occult,               Lang::Ja) => "オカルト".into(),
            (Self::Occult,               Lang::En) => "Occult".into(),
            (Self::Persuade,             Lang::Ja) => "説得".into(),
            (Self::Persuade,             Lang::En) => "Persuade".into(),
            (Self::Pilot(spec),          _)        => match spec.label(lang) { Some(s) => format!("操縦 ({s})"),      None => "操縦".into() },
            (Self::Psychoanalysis,       Lang::Ja) => "精神分析".into(),
            (Self::Psychoanalysis,       Lang::En) => "Psychoanalysis".into(),
            (Self::Psychology,           Lang::Ja) => "心理学".into(),
            (Self::Psychology,           Lang::En) => "Psychology".into(),
            (Self::Ride,                 Lang::Ja) => "乗馬".into(),
            (Self::Ride,                 Lang::En) => "Ride".into(),
            (Self::Science(spec),        _)        => match spec.label(lang) { Some(s) => format!("科学 ({s})"),       None => "科学".into() },
            (Self::SleightOfHand,        Lang::Ja) => "手さばき".into(),
            (Self::SleightOfHand,        Lang::En) => "Sleight of Hand".into(),
            (Self::SpotHidden,           Lang::Ja) => "目星".into(),
            (Self::SpotHidden,           Lang::En) => "Spot Hidden".into(),
            (Self::Stealth,              Lang::Ja) => "隠密".into(),
            (Self::Stealth,              Lang::En) => "Stealth".into(),
            (Self::Survival(spec),       _)        => match spec.label(lang) { Some(s) => format!("サバイバル ({s})"), None => "サバイバル".into() },
            (Self::Swim,                 Lang::Ja) => "水泳".into(),
            (Self::Swim,                 Lang::En) => "Swim".into(),
            (Self::Throw,                Lang::Ja) => "投擲".into(),
            (Self::Throw,                Lang::En) => "Throw".into(),
            (Self::Track,                Lang::Ja) => "追跡".into(),
            (Self::Track,                Lang::En) => "Track".into(),
            (Self::Custom { name, spec: Some(s), .. }, _) => format!("{} ({})", name, s),
            (Self::Custom { name, spec: None,    .. }, _) => name.clone(),
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
    // pub fn label(&self, lang: Lang) -> &'static str {
    //     match (self, lang) {
    //         (Self::Name,            Lang::En) => "Weapon",
    //         (Self::Name,            Lang::Ja) => "武器",
    //         (Self::Regular,         Lang::En) => "Regular",
    //         (Self::Regular,         Lang::Ja) => "レギュラー",
    //         (Self::Hard,            Lang::En) => "Hard",
    //         (Self::Hard,            Lang::Ja) => "ハード",
    //         (Self::Extreme,         Lang::En) => "Extreme",
    //         (Self::Extreme,         Lang::Ja) => "イクストリーム",
    //         (Self::Damage,          Lang::En) => "Damage",
    //         (Self::Damage,          Lang::Ja) => "ダメージ",
    //         (Self::Range,           Lang::En) => "Range",
    //         (Self::Range,           Lang::Ja) => "射程",
    //         (Self::AttacksPerRound, Lang::En) => "Attacks",
    //         (Self::AttacksPerRound, Lang::Ja) => "攻撃回数",
    //         (Self::Ammunition,      Lang::En) => "Ammo",
    //         (Self::Ammunition,      Lang::Ja) => "装弾数",
    //         (Self::Malfunction,     Lang::En) => "Malfunction",
    //         (Self::Malfunction,     Lang::Ja) => "故障",
    //     }
    // }

    pub fn display(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::BowAndArrows,           Lang::En) => "Bow and Arrows",
            (Self::BowAndArrows,           Lang::Ja) => "弓と矢",
            (Self::BrassKnuckles,          Lang::En) => "Brass Knuckles",
            (Self::BrassKnuckles,          Lang::Ja) => "ブラスナックル",
            (Self::Bullwhip,               Lang::En) => "Bullwhip",
            (Self::Bullwhip,               Lang::Ja) => "むち",
            (Self::BurningTorch,           Lang::En) => "Burning Torch",
            (Self::BurningTorch,           Lang::Ja) => "燃えているたいまつ",
            (Self::Blackjack,              Lang::En) => "Blackjack",
            (Self::Blackjack,              Lang::Ja) => "ブラックジャック",
            (Self::ClubLarge,              Lang::En) => "Club, Large",
            (Self::ClubLarge,              Lang::Ja) => "大きい棍棒",
            (Self::ClubSmall,              Lang::En) => "Club, Small",
            (Self::ClubSmall,              Lang::Ja) => "小さい棍棒",
            (Self::Crossbow,               Lang::En) => "Crossbow",
            (Self::Crossbow,               Lang::Ja) => "クロスボウ",
            (Self::Garrote,                Lang::En) => "Garrote",
            (Self::Garrote,                Lang::Ja) => "絞殺ひも",
            (Self::HatchetSickle,          Lang::En) => "Hatchet/Sickle",
            (Self::HatchetSickle,          Lang::Ja) => "手斧/小鎌",
            (Self::KnifeLarge,             Lang::En) => "Knife, Large",
            (Self::KnifeLarge,             Lang::Ja) => "大型ナイフ",
            (Self::KnifeMedium,            Lang::En) => "Knife, Medium",
            (Self::KnifeMedium,            Lang::Ja) => "中型ナイフ",
            (Self::KnifeSmall,             Lang::En) => "Knife, Small",
            (Self::KnifeSmall,             Lang::Ja) => "小型ナイフ",
            (Self::Nunchaku,               Lang::En) => "Nunchaku",
            (Self::Nunchaku,               Lang::Ja) => "ヌンチャク",
            (Self::RockThrown,             Lang::En) => "Rock, Thrown",
            (Self::RockThrown,             Lang::Ja) => "投石",
            (Self::Shuriken,               Lang::En) => "Shuriken",
            (Self::Shuriken,               Lang::Ja) => "手裏剣",
            (Self::Spear,                  Lang::En) => "Spear",
            (Self::Spear,                  Lang::Ja) => "騎兵槍",
            (Self::SpearThrown,            Lang::En) => "Spear, Thrown",
            (Self::SpearThrown,            Lang::Ja) => "投げ槍",
            (Self::Auto22Short,            Lang::En) => ".22 Short Automatic",
            (Self::Auto22Short,            Lang::Ja) => ".22ショートオートマチック",
            (Self::Derringer25,            Lang::En) => ".25 Derringer",
            (Self::Derringer25,            Lang::Ja) => ".25デリンジャー",
            (Self::Revolver32,             Lang::En) => ".32 Revolver",
            (Self::Revolver32,             Lang::Ja) => ".32リボルバー",
            (Self::Automatic32,            Lang::En) => ".32 Automatic",
            (Self::Automatic32,            Lang::Ja) => ".32オートマチック",
            (Self::LugerP08,               Lang::En) => "Model P08 Luger",
            (Self::LugerP08,               Lang::Ja) => "P08ルガー",
            (Self::Revolver45,             Lang::En) => ".45 Revolver",
            (Self::Revolver45,             Lang::Ja) => ".45リボルバー",
            (Self::Automatic45,            Lang::En) => ".45 Automatic",
            (Self::Automatic45,            Lang::Ja) => ".45オートマチック",
            (Self::BoltAction22,           Lang::En) => ".22 Bolt-Action Rifle",
            (Self::BoltAction22,           Lang::Ja) => ".22ボルトアクションライフル",
            (Self::LeverAction30,          Lang::En) => ".30 Lever-Action Carbine",
            (Self::LeverAction30,          Lang::Ja) => ".30レバーアクションカービン",
            (Self::MartiniHenry45,         Lang::En) => ".45 Martini-Henry Rifle",
            (Self::MartiniHenry45,         Lang::Ja) => ".45マルティニ・ヘンリー",
            (Self::MoranAirRifle,          Lang::En) => "Col. Moran's Air Rifle",
            (Self::MoranAirRifle,          Lang::Ja) => "モラン大佐の空気銃",
            (Self::LeeEnfield303,          Lang::En) => ".303 Lee-Enfield",
            (Self::LeeEnfield303,          Lang::Ja) => ".303リー・エンフィールド",
            (Self::BoltAction3006,         Lang::En) => ".30-06 Bolt-Action Rifle",
            (Self::BoltAction3006,         Lang::Ja) => ".30-06ボルトアクションライフル",
            (Self::ElephantGun,            Lang::En) => "Elephant Gun",
            (Self::ElephantGun,            Lang::Ja) => "エレファントガン",
            (Self::Shotgun20Gauge,         Lang::En) => "20-gauge Shotgun",
            (Self::Shotgun20Gauge,         Lang::Ja) => "20ゲージショットガン",
            (Self::Shotgun16Gauge,         Lang::En) => "16-gauge Shotgun",
            (Self::Shotgun16Gauge,         Lang::Ja) => "16ゲージショットガン",
            (Self::Shotgun12Gauge,         Lang::En) => "12-gauge Shotgun",
            (Self::Shotgun12Gauge,         Lang::Ja) => "12ゲージショットガン",
            (Self::Shotgun12GaugeSemiAuto, Lang::En) => "12-gauge Shotgun (semi-auto)",
            (Self::Shotgun12GaugeSemiAuto, Lang::Ja) => "12ゲージショットガン(半自動)",
            (Self::Shotgun12GaugeSawedOff, Lang::En) => "12-gauge Shotgun (sawed off)",
            (Self::Shotgun12GaugeSawedOff, Lang::Ja) => "12ゲージショットガン(短銃身)",
            (Self::BergmannMP18,           Lang::En) => "Bergmann MP18",
            (Self::BergmannMP18,           Lang::Ja) => "ベルグマンMP18",
            (Self::Thompson,               Lang::En) => "Thompson",
            (Self::Thompson,               Lang::Ja) => "トンプソン",
            (Self::BrowningAutoRifle,      Lang::En) => "Browning Automatic Rifle M1918",
            (Self::BrowningAutoRifle,      Lang::Ja) => "ブローニング自動小銃M1918",
            (Self::BrowningM1917,          Lang::En) => ".30 Browning M1917A1",
            (Self::BrowningM1917,          Lang::Ja) => ".30ブローニングM1917A1",
            (Self::BrenGun,                Lang::En) => "Bren Gun",
            (Self::BrenGun,                Lang::Ja) => "ブレンガン",
            (Self::LewisGun,               Lang::En) => "Mark I Lewis Gun",
            (Self::LewisGun,               Lang::Ja) => "ルイス軽機関銃Mk.I",
            (Self::Vickers303,             Lang::En) => "Vickers .303 Machine Gun",
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
    /// ダメージボーナスの実値は呼び出し側が `OtherAttribute::DamageBonus` から取得して加算する。
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
            (Self::ThickLeatherJacket, Lang::En) => "Thick Leather Jacket",
            (Self::ThickLeatherJacket, Lang::Ja) => "厚い皮のジャケット",
            (Self::WwiHelmet,          Lang::En) => "WWI Helmet",
            (Self::WwiHelmet,          Lang::Ja) => "第一次大戦型のヘルメット",
            (Self::Hardwood1In,        Lang::En) => "1\" Hardwood",
            (Self::Hardwood1In,        Lang::Ja) => "3cmの堅い木",
            (Self::PresentUsHelmet,    Lang::En) => "Present U.S. Helmet",
            (Self::PresentUsHelmet,    Lang::Ja) => "現代アメリカ軍のヘルメット",
            (Self::HeavyKevlarVest,    Lang::En) => "Heavy Kevlar Vest",
            (Self::HeavyKevlarVest,    Lang::Ja) => "厚いケブラー製のベスト",
            (Self::MilitaryBodyArmor,  Lang::En) => "Military Body Armor",
            (Self::MilitaryBodyArmor,  Lang::Ja) => "軍用ボディ・アーマー",
            (Self::BulletproofGlass,   Lang::En) => "1.5\" Bulletproof Glass",
            (Self::BulletproofGlass,   Lang::Ja) => "4cmの防弾ガラス",
            (Self::SteelPlate1In,      Lang::En) => "1\" Steel Plate",
            (Self::SteelPlate1In,      Lang::Ja) => "5cmの鋼鉄板",
            (Self::LargeSandbag,       Lang::En) => "Large Sandbag",
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
    pub fn label_spending_level(lang: Lang) -> &'static str {
        match lang { Lang::En => "Spending Level", Lang::Ja => "支出レベル" }
    }
    pub fn label_cash(lang: Lang) -> &'static str {
        match lang { Lang::En => "Cash", Lang::Ja => "現金" }
    }
    pub fn label_assets(lang: Lang) -> &'static str {
        match lang { Lang::En => "Assets", Lang::Ja => "資産" }
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

    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Weapon(_),   Lang::En) => "Weapon",
            (Self::Weapon(_),   Lang::Ja) => "武器",
            (Self::Armor(_),    Lang::En) => "Armor",
            (Self::Armor(_),    Lang::Ja) => "装甲",
            (Self::GearItem(_), Lang::En) => "Equipment",
            (Self::GearItem(_), Lang::Ja) => "装備",
            (Self::Wealth(_),   Lang::En) => "Wealth",
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

    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::KeyConnection(_),              Lang::En) => "Key Connection",
            (Self::KeyConnection(_),              Lang::Ja) => "キーコネクション",
            (Self::PersonalDescription,           Lang::En) => "Personal Description",
            (Self::PersonalDescription,           Lang::Ja) => "容姿の描写",
            (Self::IdeologyAndBeliefs,            Lang::En) => "Ideology & Beliefs",
            (Self::IdeologyAndBeliefs,            Lang::Ja) => "イデオロギー・信念", // p40 原文が"&"なので／から・に修正
            (Self::SignificantPeople,             Lang::En) => "Significant People",
            (Self::SignificantPeople,             Lang::Ja) => "重要な人物",
            (Self::MeaningfulLocation,            Lang::En) => "Meaningful Location",
            (Self::MeaningfulLocation,            Lang::Ja) => "意味のある場所",
            (Self::TreasuredPossession,           Lang::En) => "Treasured Possession",
            (Self::TreasuredPossession,           Lang::Ja) => "秘蔵の品",
            (Self::Trait,                         Lang::En) => "Trait",
            (Self::Trait,                         Lang::Ja) => "特徴",
            (Self::InjuresAndScars,               Lang::En) => "Injuries & Scars",
            (Self::InjuresAndScars,               Lang::Ja) => "負傷、傷跡",
            (Self::PhobiasAndManias,              Lang::En) => "Phobias & Manias",
            (Self::PhobiasAndManias,              Lang::Ja) => "恐怖症とマニア",
            (Self::ArcaneTomesAndSpells,          Lang::En) => "Arcane Tomes & Spells",
            (Self::ArcaneTomesAndSpells,          Lang::Ja) => "魔道書、呪文、アーティファクト",
            (Self::EncountersWithStrangeEntities, Lang::En) => "Encounters with Strange Entities",
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
        core::array::from_fn(|slot| Memo { slot })
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
                Lang::En => format!("Note {}",  self.slot + 1),
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