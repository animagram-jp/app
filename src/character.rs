use crate::list::VariableList;
use crate::datetime;

pub struct Instance {
    pub data: VariableList<u8>,
}

impl Instance {
    pub fn new() -> Self {
        Self { data: VariableList::new() }
    }
}

// --- 芸術/製作 (Art/Craft) 専門分野 ---
// ルールブック掲載例。他は Custom で自由記入。
enum ArtCraftSpec {
    Acting,       // 演劇
    Barber,       // 理容
    Calligraphy,  // 書道
    Carpentry,    // 大工仕事
    Cobbling,     // 靴製造
    Cook,         // 料理
    Dancing,      // 踊り
    FineArt,      // 絵画
    Forgery,      // 文書偽造
    Photography,  // 写真術
    Pottery,      // 陶芸
    Sculpting,    // 彫刻
    Writing,      // 執筆
    Custom(String),
}

// --- 近接戦闘 (Fighting) 専門分野 ---
// ルールブックで初期値が個別定義済み。
enum FightingSpec {
    Axe,          // 斧           15%
    Brawl,        // 格闘         25%
    Chainsaw,     // チェーンソー  10%
    Flail,        // フレイル      10%  (ヌンチャク・モーニングスター等)
    Garrote,      // 絞殺ひも      15%
    Spear,        // 槍           20%
    Sword,        // 刀剣          20%
    Whip,         // 鞭            05%  (ボーラ含む)
    Custom(String),
}

// --- 射撃 (Firearms) 専門分野 ---
// ルールブックで初期値が個別定義済み。
enum FirearmsSpec {
    Bow,           // 弓                   15%
    Handgun,       // 拳銃                 20%
    HeavyWeapons,  // 重火器               10%
    MachineGun,    // 機関銃               10%
    RifleShotgun,  // ライフル/ショットガン  25%
    SubmachineGun, // サブマシンガン         15%
    Custom(String),
}

// --- ほかの言語 (Language Other) 専門分野 ---
// 言語名を自由記入。母国語 (LanguageOwn) は専門分野なし (初期値 = EDU)。
enum LanguageSpec {
    Custom(String),
}

// --- 操縦 (Pilot) 専門分野 ---
// ルールブック掲載例。他は Custom で自由記入。
// 1920s: Balloon / Dirigible / CivilProp / Boat / SteamShip / Sailboat
// Modern: CivilProp / CivilJet / Airliner / JetFighter / Helicopter / Boat / SteamShip / Sailboat
enum PilotSpec {
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
    Custom(String),
}

// --- 科学 (Science) 専門分野 ---
// ルールブック掲載例。他は Custom で自由記入。
// ※ 考古学 (Archaeology) は独立技能のため対象外。
enum ScienceSpec {
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
    Custom(String),
}

// --- サバイバル (Survival) 専門分野 ---
// ルールブック掲載例。他は Custom で自由記入。
enum SurvivalSpec {
    Arctic,  // 北極/寒冷地
    Desert,  // 砂漠
    Sea,     // 海上
    Custom(String),
}

enum Skill {
    // 専門分野なし
    LibraryUse,
    Medicine,
    Psychology,

    // 専門分野あり（ルールブック定義済み選択肢 + 自由記入）
    ArtCraft(ArtCraftSpec),
    Fighting(FightingSpec),
    Firearms(FirearmsSpec),
    LanguageOther(LanguageSpec),
    Pilot(PilotSpec),
    Science(ScienceSpec),
    Survival(SurvivalSpec),

    // 技能名+専門分野 完全自由記入（キャラシ空白欄に対応）
    Custom { name: String, spec: Option<String> },
}

// ============================================================
// --- CoC 7th 導出値・判定カテゴリ ---
// ============================================================

// --- ビルド (Build) ---
// STR + SIZ の合計値から決定される離散段階。DamageBonusDice と 1対1 対応する。
enum BuildRank {
    NegTwo,   // -2  (STR+SIZ:   2- 64)
    NegOne,   // -1  (STR+SIZ:  65- 84)
    Zero,     //  0  (STR+SIZ:  85-124)
    PosOne,   // +1  (STR+SIZ: 125-164)
    PosTwo,   // +2  (STR+SIZ: 165-204)
    PosThree, // +3  (STR+SIZ: 205-284)
    PosFour,  // +4  (STR+SIZ: 285-364)
    PosFive,  // +5  (STR+SIZ: 365+   )
}

impl BuildRank {
    pub fn from_str_siz(sum: u16) -> Self {
        match sum {
              2..= 64 => Self::NegTwo,
             65..= 84 => Self::NegOne,
             85..=124 => Self::Zero,
            125..=164 => Self::PosOne,
            165..=204 => Self::PosTwo,
            205..=284 => Self::PosThree,
            285..=364 => Self::PosFour,
            _         => Self::PosFive,
        }
    }

    pub fn int_value(&self) -> i8 {
        match self {
            Self::NegTwo   => -2,
            Self::NegOne   => -1,
            Self::Zero     =>  0,
            Self::PosOne   =>  1,
            Self::PosTwo   =>  2,
            Self::PosThree =>  3,
            Self::PosFour  =>  4,
            Self::PosFive  =>  5,
        }
    }

    pub fn damage_bonus(&self) -> DamageBonusDice {
        match self {
            Self::NegTwo   => DamageBonusDice::NegTwo,
            Self::NegOne   => DamageBonusDice::NegOne,
            Self::Zero     => DamageBonusDice::None,
            Self::PosOne   => DamageBonusDice::PosOnD4,
            Self::PosTwo   => DamageBonusDice::PosOnD6,
            Self::PosThree => DamageBonusDice::PosTwD6,
            Self::PosFour  => DamageBonusDice::PosThrD6,
            Self::PosFive  => DamageBonusDice::PosForD6,
        }
    }
}

// --- ダメージボーナス (DamageBonus) ---
// ダイス式のため整数で表現できない。BuildRank と 1対1 対応する。
enum DamageBonusDice {
    NegTwo,   // -2    (Build -2)
    NegOne,   // -1    (Build -1)
    None,     // なし   (Build  0)
    PosOnD4,  // +1D4  (Build +1)
    PosOnD6,  // +1D6  (Build +2)
    PosTwD6,  // +2D6  (Build +3)
    PosThrD6, // +3D6  (Build +4)
    PosForD6, // +4D6  (Build +5)
}

// --- 移動率基準値 (MoveBase) ---
// STR・DEX と SIZ の大小比較から決定される。年齢修正は AgeCategory で別途管理する。
enum MoveBase {
    Seven, // 7: STR <  SIZ かつ DEX <  SIZ
    Eight, // 8: STR >= SIZ または DEX >= SIZ（どちらか一方のみが超える）
    Nine,  // 9: STR >  SIZ かつ DEX >  SIZ
}

impl MoveBase {
    pub fn from_str_dex_siz(str_val: u16, dex: u16, siz: u16) -> Self {
        match (str_val > siz, dex > siz) {
            (false, false) => Self::Seven,
            (true,  true)  => Self::Nine,
            _              => Self::Eight,
        }
    }

    pub fn int_value(&self) -> u8 {
        match self {
            Self::Seven => 7,
            Self::Eight => 8,
            Self::Nine  => 9,
        }
    }
}

// --- 生活水準 (Standard of Living) ---
// 信用 (Credit Rating) の値から決定される区分。
enum StandardOfLiving {
    Pauper,    // 惨め      (CR: 0     )
    Poor,      // 貧乏      (CR: 1-  9 )
    Average,   // 平均      (CR: 10- 49)
    Wealthy,   // 裕福      (CR: 50- 89)
    Rich,      // 金持ち    (CR: 90- 98)
    SuperRich, // 超大金持ち (CR: 99    )
}

impl StandardOfLiving {
    pub fn from_cr(cr: u16) -> Self {
        match cr {
             0        => Self::Pauper,
             1..= 9   => Self::Poor,
            10..= 49  => Self::Average,
            50..= 89  => Self::Wealthy,
            90..= 98  => Self::Rich,
            _         => Self::SuperRich,
        }
    }
}

// --- 年齢カテゴリ (AgeCategory) ---
// キャラクター作成時の能力値修正ルールの区分。
// Teen のみ STR+SIZ からの差し引きで、それ以外は STR/CON/DEX からの合計差し引き。
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

    // MOV への減算値
    pub fn mov_penalty(&self) -> u8 {
        match self {
            Self::Teen    => 0,
            Self::Young   => 0,
            Self::Middle  => 1,
            Self::Senior  => 2,
            Self::Elderly => 3,
            Self::Old     => 4,
            Self::Ancient => 5,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    
    // --- system ---
    Identity: u32,
    Timestamp::Created: u64, // datetime.rs
    Timestamp::Updated: u64, // datetime.rs

    // --- primary but not required ---
    Occupation: enum<str>|str,
    Age::Initial: u8,
    Age::Change: u8,

    // --- primary and required (for all roll, not required to users) ---
    Strength::Initial: u16,
    Strength::Change: u16,
    Constitution::Initial: u16,
    Constitution::Change: u16,
    Size,
    Dexterity,
    Appearance,
    Intelligence,
    Power,
    Education,
    Luck,

    // --- derived ---
    Sanity,
    DamageBonus,
    Build,
    HitPoints,
    MagicPoints,
    Mobility,

    // --- スキルポイント ---
    OccupationSkillPoints,
    InterestSkillPoints,

    // --- スキル ---
    Accounting,
    Anthropology,
    Archaeology,
    Appraise,
    ArtCraft(Specializations),
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
    Fighting(Specializations),
    FightingOther,
    Firearms(Specializations),
    FirstAid,
    History,
    Intimidate,
    Jump,
    LanguageOther(Specializations),
    LanguageOwn(Specializations),
    Law,
    LibraryUse,
    Listen,
    Locksmith,
    MechRepair,
    NaturalWorld,
    Navigate,
    Occult,
    Persuade,
    Pilot(Specializations),
    Psychoanalysis,
    Psychology,
    Ride,
    Science(Specializations),
    SleightOfHand,
    SpotHidden,
    Stealth,
    Survival(Specializations),
    Swim,
    Throw,
    Track,

    // --- バックストーリー ---
    KeyConnection,
    PersonalDescription,
    IdeologyAndBeliefs,
    SignificantPeople,
    MeaningfulLocation,
    TreasuredPossessions,
    Trait,
    PhobiasAndManias,
    ArcaneTomesAndSpells,
}

impl Model {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn dom_id(self) -> &'static str {
        match self {
            Self::Identity               => "identity",
            Self::Timestamp              => "timestamp",
            Self::Occupation             => "occupation",
            Self::Age                    => "age",
            Self::Strength               => "strength",
            Self::Constitution           => "constitution",
            Self::Size                   => "size",
            Self::Dexterity              => "dexterity",
            Self::Appearance             => "appearance",
            Self::Intelligence           => "intelligence",
            Self::Power                  => "power",
            Self::Education              => "education",
            Self::Luck                   => "luck",
            Self::Sanity                 => "sanity",
            Self::DamageBonus            => "damage_bonus",
            Self::Build                  => "build",
            Self::HitPoints              => "hit_points",
            Self::MagicPoints            => "magic_points",
            Self::Mobility               => "mobility",
            Self::OccupationSkillPoints  => "occupation_skill_points",
            Self::InterestSkillPoints    => "interest_skill_points",
            Self::Accounting             => "accounting",
            Self::Anthropology           => "anthropology",
            Self::Archaeology            => "archaeology",
            Self::Appraise               => "appraise",
            Self::ArtCraft               => "art_craft",
            Self::Charm                  => "charm",
            Self::Climb                  => "climb",
            Self::ComputerUse            => "computer_use",
            Self::CreditRating           => "credit_rating",
            Self::CthulhuMythos          => "cthulhu_mythos",
            Self::Disguise               => "disguise",
            Self::Dodge                  => "dodge",
            Self::DriveAuto              => "drive_auto",
            Self::ElecRepair             => "elec_repair",
            Self::Electronics            => "electronics",
            Self::FastTalk               => "fast_talk",
            Self::FightingBrawl          => "fighting_brawl",
            Self::FightingOther          => "fighting_other",
            Self::FirearmsHandgun        => "firearms_handgun",
            Self::FirearmsRifleShotgun   => "firearms_rifle_shotgun",
            Self::FirearmsOther          => "firearms_other",
            Self::FirstAid               => "first_aid",
            Self::History                => "history",
            Self::Intimidate             => "intimidate",
            Self::Jump                   => "jump",
            Self::LanguageOther          => "language_other",
            Self::LanguageOwn            => "language_own",
            Self::Law                    => "law",
            Self::LibraryUse             => "library_use",
            Self::Listen                 => "listen",
            Self::Locksmith              => "locksmith",
            Self::MechRepair             => "mech_repair",
            Self::Medicine               => "medicine",
            Self::NaturalWorld           => "natural_world",
            Self::Navigate               => "navigate",
            Self::Occult                 => "occult",
            Self::Persuade               => "persuade",
            Self::Pilot                  => "pilot",
            Self::Psychoanalysis         => "psychoanalysis",
            Self::Psychology             => "psychology",
            Self::Ride                   => "ride",
            Self::Science                => "science",
            Self::SleightOfHand          => "sleight_of_hand",
            Self::SpotHidden             => "spot_hidden",
            Self::Stealth                => "stealth",
            Self::Survival               => "survival",
            Self::Swim                   => "swim",
            Self::Throw                  => "throw",
            Self::Track                  => "track",
            Self::KeyConnection          => "key_connection",
            Self::PersonalDescription    => "personal_description",
            Self::IdeologyAndBeliefs     => "ideology_and_beliefs",
            Self::SignificantPeople      => "significant_people",
            Self::MeaningfulLocation     => "meaningful_location",
            Self::TreasuredPossessions   => "treasured_possessions",
            Self::Trait                  => "trait",
            Self::PhobiasAndManias       => "phobias_and_manias",
            Self::ArcaneTomesAndSpells   => "arcane_tomes_and_spells",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OccupationKind {
    Athlete, Doctor, Engineer, Entertainer, Activist, Professor, Police, Detective, Artist,
    Antiquarian, Author, MilitaryOfficer, Librarian, Journalist, PrivateInvestigator,
    Clergy, Parapsychologist, Dilettante, Missionary, TribeMember, Farmer,
    Pilot, Hacker, Criminal, Soldier, Lawyer, Drifter, Musician,
}

pub mod schema {
    use super::{Instance, Model, OccupationKind};
    use crate::list::ListError;
    use crate::Lang;

    // --- public get/set（app.rsなどclient向け） ---

    pub fn get(instance: &Instance, field: Model) -> Result<u16, ListError> {
        get_u16(instance, field)
    }

    pub fn set(instance: &mut Instance, field: Model, v: u16) -> Result<(), ListError> {
        set_u16(instance, field, v)
    }

    pub fn get_text(instance: &Instance, field: Model) -> Result<String, ListError> {
        get_str(instance, field)
    }

    pub fn set_text(instance: &mut Instance, field: Model, v: &str) -> Result<(), ListError> {
        set_str(instance, field, v)
    }

    // --- 低レベル get/set ---

    fn id(field: Model) -> usize {
        field.index()
    }

    fn get_u16(instance: &Instance, field: Model) -> Result<u16, ListError> {
        let bytes = instance.data.get(&id(field))?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn set_u16(instance: &mut Instance, field: Model, value: u16) -> Result<(), ListError> {
        instance.data.upsert(&id(field), &value.to_le_bytes())?;
        Ok(())
    }

    fn get_str(instance: &Instance, field: Model) -> Result<String, ListError> {
        let bytes = instance.data.get(&id(field))?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    fn set_str(instance: &mut Instance, field: Model, value: &str) -> Result<(), ListError> {
        instance.data.upsert(&id(field), value.as_bytes())?;
        Ok(())
    }

    // --- Attribute グルーピング ---

    pub enum Attribute {
        Characteristic,
        Derived,
        Skill,
        Backstory,
    }

    pub fn attribute(attr: Attribute) -> &'static [Model] {
        match attr {
            Attribute::Characteristic => &[
                Model::Strength, Model::Constitution, Model::Size, Model::Dexterity,
                Model::Appearance, Model::Intelligence, Model::Power, Model::Education,
                Model::Luck,
            ],
            Attribute::Derived => &[
                Model::Sanity, Model::HitPoints, Model::MagicPoints,
                Model::Dodge, Model::LanguageOwn,
            ],
            Attribute::Skill => &[
                Model::Accounting, Model::Anthropology, Model::Archaeology, Model::Appraise,
                Model::ArtCraft, Model::Charm, Model::Climb, Model::ComputerUse,
                Model::CreditRating, Model::CthulhuMythos, Model::Disguise, Model::Dodge,
                Model::DriveAuto, Model::ElecRepair, Model::Electronics, Model::FastTalk,
                Model::FightingBrawl, Model::FightingOther, Model::FirearmsHandgun,
                Model::FirearmsRifleShotgun, Model::FirearmsOther, Model::FirstAid,
                Model::History, Model::Intimidate, Model::Jump, Model::LanguageOther,
                Model::LanguageOwn, Model::Law, Model::LibraryUse, Model::Listen,
                Model::Locksmith, Model::MechRepair, Model::Medicine, Model::NaturalWorld,
                Model::Navigate, Model::Occult, Model::Persuade, Model::Pilot,
                Model::Psychoanalysis, Model::Psychology, Model::Ride, Model::Science,
                Model::SleightOfHand, Model::SpotHidden, Model::Stealth, Model::Survival,
                Model::Swim, Model::Throw, Model::Track,
            ],
            Attribute::Backstory => &[
                Model::KeyConnection, Model::PersonalDescription, Model::IdeologyAndBeliefs,
                Model::SignificantPeople, Model::MeaningfulLocation, Model::TreasuredPossessions,
                Model::Trait, Model::PhobiasAndManias, Model::ArcaneTomesAndSpells,
            ],
        }
    }

    // --- 導出値の計算・書き込み ---

    pub fn create(field: Model, instance: &mut Instance) -> Result<(), ListError> {
        match field {
            Model::HitPoints => {
                let v = (get_u16(instance, Model::Constitution)? + get_u16(instance, Model::Size)?) / 10;
                set_u16(instance, Model::HitPoints, v)
            }
            Model::MagicPoints => {
                let v = get_u16(instance, Model::Power)? / 5;
                set_u16(instance, Model::MagicPoints, v)
            }
            Model::Sanity => {
                let v = get_u16(instance, Model::Power)?;
                set_u16(instance, Model::Sanity, v)
            }
            Model::Dodge => {
                let v = get_u16(instance, Model::Dexterity)? / 2;
                set_u16(instance, Model::Dodge, v)
            }
            Model::LanguageOwn => {
                let v = get_u16(instance, Model::Education)?;
                set_u16(instance, Model::LanguageOwn, v)
            }
            _ => Ok(()),
        }
    }

    // Derivedフィールドを一括再計算する
    pub fn update(instance: &mut Instance) -> Result<(), ListError> {
        for &field in attribute(Attribute::Derived) {
            create(field, instance)?;
        }
        Ok(())
    }

    // 全能力値をロールしてinstanceに書き込み、導出値も更新する
    pub fn roll_characteristics(instance: &mut Instance) -> Result<(), ListError> {
        for &field in attribute(Attribute::Characteristic) {
            let v = roll_characteristic(field);
            set_u16(instance, field, v)?;
        }
        update(instance)
    }

    // SIZ / INT / EDU は (2d6+6)×5、それ以外は 3d6×5
    pub fn roll_characteristic(field: Model) -> u16 {
        match field {
            Model::Size | Model::Intelligence | Model::Education =>
                (crate::n_d_n(2, 6) + 6) as u16 * 5,
            _ =>
                crate::n_d_n(3, 6) as u16 * 5,
        }
    }

    // --- スキル ---

    pub mod skill {
        use super::super::Model;
        use super::*;

        // 固定初期値（u16）。0はderived or 任意入力を示す。
        pub fn base_value(field: Model) -> u16 {
            match field {
                Model::Accounting           => 5,
                Model::Anthropology         => 1,
                Model::Archaeology          => 1,
                Model::Appraise             => 5,
                Model::ArtCraft             => 5,
                Model::Charm                => 15,
                Model::Climb                => 20,
                Model::ComputerUse          => 5,
                Model::CreditRating         => 0,
                Model::CthulhuMythos        => 0,
                Model::Disguise             => 5,
                Model::Dodge                => 0, // derived: DEX / 2
                Model::DriveAuto            => 20,
                Model::ElecRepair           => 10,
                Model::Electronics          => 1,
                Model::FastTalk             => 5,
                Model::FightingBrawl        => 25,
                Model::FightingOther        => 0,
                Model::FirearmsHandgun      => 20,
                Model::FirearmsRifleShotgun => 25,
                Model::FirearmsOther        => 0,
                Model::FirstAid             => 30,
                Model::History              => 5,
                Model::Intimidate           => 15,
                Model::Jump                 => 20,
                Model::LanguageOther        => 1,
                Model::LanguageOwn          => 0, // derived: EDU
                Model::Law                  => 5,
                Model::LibraryUse           => 20,
                Model::Listen               => 20,
                Model::Locksmith            => 1,
                Model::MechRepair           => 10,
                Model::Medicine             => 1,
                Model::NaturalWorld         => 10,
                Model::Navigate             => 10,
                Model::Occult               => 5,
                Model::Persuade             => 10,
                Model::Pilot                => 1,
                Model::Psychoanalysis       => 1,
                Model::Psychology           => 10,
                Model::Ride                 => 5,
                Model::Science              => 1,
                Model::SleightOfHand        => 10,
                Model::SpotHidden           => 25,
                Model::Stealth              => 20,
                Model::Survival             => 10,
                Model::Swim                 => 20,
                Model::Throw                => 20,
                Model::Track                => 10,
                _ => panic!("{:?} is not a skill field", field),
            }
        }

        // 専門分野ごとの初期値。
        // 0 は「任意入力（Keeper・プレイヤーが決定）」を示す。
        // Fighting / Firearms は専門分野ごとに値が異なる。
        // その他の専門分野技能は全選択肢で親技能の初期値と同じ値を返す。

        pub fn art_craft_spec_base_value(_spec: &super::super::ArtCraftSpec) -> u16 {
            5 // 全専門分野共通
        }

        pub fn fighting_spec_base_value(spec: &super::super::FightingSpec) -> u16 {
            use super::super::FightingSpec::*;
            match spec {
                Axe       => 15,
                Brawl     => 25,
                Chainsaw  => 10,
                Flail     => 10,
                Garrote   => 15,
                Spear     => 20,
                Sword     => 20,
                Whip      =>  5,
                Custom(_) =>  0,
            }
        }

        pub fn firearms_spec_base_value(spec: &super::super::FirearmsSpec) -> u16 {
            use super::super::FirearmsSpec::*;
            match spec {
                Bow           => 15,
                Handgun       => 20,
                HeavyWeapons  => 10,
                MachineGun    => 10,
                RifleShotgun  => 25,
                SubmachineGun => 15,
                Custom(_)     =>  0,
            }
        }

        pub fn language_spec_base_value(_spec: &super::super::LanguageSpec) -> u16 {
            1 // 全言語共通
        }

        pub fn pilot_spec_base_value(_spec: &super::super::PilotSpec) -> u16 {
            1 // 全専門分野共通
        }

        pub fn science_spec_base_value(_spec: &super::super::ScienceSpec) -> u16 {
            1 // 全専門分野共通
        }

        pub fn survival_spec_base_value(_spec: &super::super::SurvivalSpec) -> u16 {
            10 // 全専門分野共通
        }

        pub fn get(instance: &Instance, field: Model) -> Result<u16, ListError> {
            get_u16(instance, field)
        }

        pub fn set(instance: &mut Instance, field: Model, v: u16) -> Result<(), ListError> {
            set_u16(instance, field, v)
        }
    }

    // --- ラベル ---

    pub fn label(field: Model, lang: Lang) -> &'static str {
        match (field, lang) {
            // --- 基本情報 ---
            (Model::Identity,   _)        => "ID",
            (Model::Timestamp,  Lang::Ja) => "yyyy年mm月dd日",
            (Model::Timestamp,  Lang::En) => "yyyy-mm-dd",
            (Model::Occupation, Lang::Ja) => "職業",
            (Model::Occupation, Lang::En) => "Occupation",
            (Model::Age,        Lang::Ja) => "年齢",
            (Model::Age,        Lang::En) => "Age",

            // --- 能力値 ---
            (Model::Strength,     _) => "STR",
            (Model::Constitution, _) => "CON",
            (Model::Size,         _) => "SIZ",
            (Model::Dexterity,    _) => "DEX",
            (Model::Appearance,   _) => "APP",
            (Model::Intelligence, _) => "INT",
            (Model::Power,        _) => "POW",
            (Model::Education,    _) => "EDU",
            (Model::Luck,  Lang::Ja) => "幸運",
            (Model::Luck,  Lang::En) => "Luck",

            // --- 導出ステータス ---
            (Model::Sanity,      Lang::Ja) => "正気度",
            (Model::Sanity,      Lang::En) => "SAN",
            (Model::DamageBonus, Lang::Ja) => "ダメージボーナス",
            (Model::DamageBonus, Lang::En) => "Damage Bonus",
            (Model::Build,       Lang::Ja) => "ビルド",
            (Model::Build,       Lang::En) => "Build",
            (Model::HitPoints,   Lang::Ja) => "耐久力",
            (Model::HitPoints,   Lang::En) => "HP",
            (Model::MagicPoints, Lang::Ja) => "マジックポイント",
            (Model::MagicPoints, Lang::En) => "MP",
            (Model::Mobility,    Lang::Ja) => "移動率",
            (Model::Mobility,    Lang::En) => "MOV",

            // --- スキルポイント ---
            (Model::OccupationSkillPoints, Lang::Ja) => "職業技能ポイント",
            (Model::OccupationSkillPoints, Lang::En) => "Occupation Skill Points",
            (Model::InterestSkillPoints,   Lang::Ja) => "興味技能ポイント",
            (Model::InterestSkillPoints,   Lang::En) => "Interest Skill Points",

            // --- スキル ---
            (Model::Accounting,           Lang::Ja) => "経理",
            (Model::Accounting,           Lang::En) => "Accounting",
            (Model::Anthropology,         Lang::Ja) => "人類学",
            (Model::Anthropology,         Lang::En) => "Anthropology",
            (Model::Archaeology,          Lang::Ja) => "考古学",
            (Model::Archaeology,          Lang::En) => "Archaeology",
            (Model::Appraise,             Lang::Ja) => "鑑定",
            (Model::Appraise,             Lang::En) => "Appraise",
            (Model::ArtCraft,             Lang::Ja) => "芸術/製作",
            (Model::ArtCraft,             Lang::En) => "Art/Craft",
            (Model::Charm,                Lang::Ja) => "魅惑",
            (Model::Charm,                Lang::En) => "Charm",
            (Model::Climb,                Lang::Ja) => "登攀",
            (Model::Climb,                Lang::En) => "Climb",
            (Model::ComputerUse,          Lang::Ja) => "コンピューター",
            (Model::ComputerUse,          Lang::En) => "Computer Use",
            (Model::CreditRating,         Lang::Ja) => "信用",
            (Model::CreditRating,         Lang::En) => "Credit Rating",
            (Model::CthulhuMythos,        Lang::Ja) => "クトゥルフ神話",
            (Model::CthulhuMythos,        Lang::En) => "Cthulhu Mythos",
            (Model::Disguise,             Lang::Ja) => "変装",
            (Model::Disguise,             Lang::En) => "Disguise",
            (Model::Dodge,                Lang::Ja) => "回避",
            (Model::Dodge,                Lang::En) => "Dodge",
            (Model::DriveAuto,            Lang::Ja) => "運転（自動車）",
            (Model::DriveAuto,            Lang::En) => "Drive Auto",
            (Model::ElecRepair,           Lang::Ja) => "電気修理",
            (Model::ElecRepair,           Lang::En) => "Elec. Repair",
            (Model::Electronics,          Lang::Ja) => "電子工学",
            (Model::Electronics,          Lang::En) => "Electronics",
            (Model::FastTalk,             Lang::Ja) => "言いくるめ",
            (Model::FastTalk,             Lang::En) => "Fast Talk",
            (Model::FightingBrawl,        Lang::Ja) => "近接戦闘（格闘）",
            (Model::FightingBrawl,        Lang::En) => "Fighting (Brawl)",
            (Model::FightingOther,        Lang::Ja) => "近接戦闘（その他）",
            (Model::FightingOther,        Lang::En) => "Fighting (Other)",
            (Model::FirearmsHandgun,      Lang::Ja) => "射撃（拳銃）",
            (Model::FirearmsHandgun,      Lang::En) => "Firearms (Handgun)",
            (Model::FirearmsRifleShotgun, Lang::Ja) => "射撃（ライフル/ショットガン）",
            (Model::FirearmsRifleShotgun, Lang::En) => "Firearms (Rifle/Shotgun)",
            (Model::FirearmsOther,        Lang::Ja) => "射撃（その他）",
            (Model::FirearmsOther,        Lang::En) => "Firearms (Other)",
            (Model::FirstAid,             Lang::Ja) => "応急手当",
            (Model::FirstAid,             Lang::En) => "First Aid",
            (Model::History,              Lang::Ja) => "歴史",
            (Model::History,              Lang::En) => "History",
            (Model::Intimidate,           Lang::Ja) => "威圧",
            (Model::Intimidate,           Lang::En) => "Intimidate",
            (Model::Jump,                 Lang::Ja) => "跳躍",
            (Model::Jump,                 Lang::En) => "Jump",
            (Model::LanguageOther,        Lang::Ja) => "ほかの言語",
            (Model::LanguageOther,        Lang::En) => "Language (Other)",
            (Model::LanguageOwn,          Lang::Ja) => "母国語",
            (Model::LanguageOwn,          Lang::En) => "Language (Own)",
            (Model::Law,                  Lang::Ja) => "法律",
            (Model::Law,                  Lang::En) => "Law",
            (Model::LibraryUse,           Lang::Ja) => "図書館",
            (Model::LibraryUse,           Lang::En) => "Library Use",
            (Model::Listen,               Lang::Ja) => "聞き耳",
            (Model::Listen,               Lang::En) => "Listen",
            (Model::Locksmith,            Lang::Ja) => "鍵開け",
            (Model::Locksmith,            Lang::En) => "Locksmith",
            (Model::MechRepair,           Lang::Ja) => "機械修理",
            (Model::MechRepair,           Lang::En) => "Mech. Repair",
            (Model::Medicine,             Lang::Ja) => "医学",
            (Model::Medicine,             Lang::En) => "Medicine",
            (Model::NaturalWorld,         Lang::Ja) => "自然",
            (Model::NaturalWorld,         Lang::En) => "Natural World",
            (Model::Navigate,             Lang::Ja) => "ナビゲート",
            (Model::Navigate,             Lang::En) => "Navigate",
            (Model::Occult,               Lang::Ja) => "オカルト",
            (Model::Occult,               Lang::En) => "Occult",
            (Model::Persuade,             Lang::Ja) => "説得",
            (Model::Persuade,             Lang::En) => "Persuade",
            (Model::Pilot,                Lang::Ja) => "操縦",
            (Model::Pilot,                Lang::En) => "Pilot",
            (Model::Psychoanalysis,       Lang::Ja) => "精神分析",
            (Model::Psychoanalysis,       Lang::En) => "Psychoanalysis",
            (Model::Psychology,           Lang::Ja) => "心理学",
            (Model::Psychology,           Lang::En) => "Psychology",
            (Model::Ride,                 Lang::Ja) => "乗馬",
            (Model::Ride,                 Lang::En) => "Ride",
            (Model::Science,              Lang::Ja) => "科学",
            (Model::Science,              Lang::En) => "Science",
            (Model::SleightOfHand,        Lang::Ja) => "手さばき",
            (Model::SleightOfHand,        Lang::En) => "Sleight of Hand",
            (Model::SpotHidden,           Lang::Ja) => "目星",
            (Model::SpotHidden,           Lang::En) => "Spot Hidden",
            (Model::Stealth,              Lang::Ja) => "隠密",
            (Model::Stealth,              Lang::En) => "Stealth",
            (Model::Survival,             Lang::Ja) => "サバイバル",
            (Model::Survival,             Lang::En) => "Survival",
            (Model::Swim,                 Lang::Ja) => "水泳",
            (Model::Swim,                 Lang::En) => "Swim",
            (Model::Throw,                Lang::Ja) => "投擲",
            (Model::Throw,                Lang::En) => "Throw",
            (Model::Track,                Lang::Ja) => "追跡",
            (Model::Track,                Lang::En) => "Track",

            // --- バックストーリー ---
            (Model::KeyConnection,        Lang::Ja) => "キーコネクション",
            (Model::KeyConnection,        Lang::En) => "Key Connection",
            (Model::PersonalDescription,  Lang::Ja) => "個人的な記述",
            (Model::PersonalDescription,  Lang::En) => "Personal Description",
            (Model::IdeologyAndBeliefs,   Lang::Ja) => "イデオロギーと信念",
            (Model::IdeologyAndBeliefs,   Lang::En) => "Ideology & Beliefs",
            (Model::SignificantPeople,    Lang::Ja) => "重要な人物",
            (Model::SignificantPeople,    Lang::En) => "Significant People",
            (Model::MeaningfulLocation,   Lang::Ja) => "思い出の場所",
            (Model::MeaningfulLocation,   Lang::En) => "Meaningful Location",
            (Model::TreasuredPossessions, Lang::Ja) => "大切な持ち物",
            (Model::TreasuredPossessions, Lang::En) => "Treasured Possessions",
            (Model::Trait,                Lang::Ja) => "特徴・癖",
            (Model::Trait,                Lang::En) => "Trait",
            (Model::PhobiasAndManias,     Lang::Ja) => "恐怖症とマニア",
            (Model::PhobiasAndManias,     Lang::En) => "Phobias & Manias",
            (Model::ArcaneTomesAndSpells, Lang::Ja) => "魔道書と呪文",
            (Model::ArcaneTomesAndSpells, Lang::En) => "Arcane Tomes & Spells",
        }
    }

    // --- 専門分野ラベル ---
    // Custom(_) は動的な文字列のため "" を返す。呼び出し側でinner Stringを直接使用すること。

    pub fn art_craft_spec_label(spec: &super::ArtCraftSpec, lang: Lang) -> &'static str {
        use super::ArtCraftSpec::*;
        match (spec, lang) {
            (Acting,      Lang::Ja) => "演劇",
            (Acting,      Lang::En) => "Acting",
            (Barber,      Lang::Ja) => "理容",
            (Barber,      Lang::En) => "Barber",
            (Calligraphy, Lang::Ja) => "書道",
            (Calligraphy, Lang::En) => "Calligraphy",
            (Carpentry,   Lang::Ja) => "大工仕事",
            (Carpentry,   Lang::En) => "Carpentry",
            (Cobbling,    Lang::Ja) => "靴製造",
            (Cobbling,    Lang::En) => "Cobbling",
            (Cook,        Lang::Ja) => "料理",
            (Cook,        Lang::En) => "Cook",
            (Dancing,     Lang::Ja) => "踊り",
            (Dancing,     Lang::En) => "Dancing",
            (FineArt,     Lang::Ja) => "絵画",
            (FineArt,     Lang::En) => "Fine Art",
            (Forgery,     Lang::Ja) => "文書偽造",
            (Forgery,     Lang::En) => "Forgery",
            (Photography, Lang::Ja) => "写真術",
            (Photography, Lang::En) => "Photography",
            (Pottery,     Lang::Ja) => "陶芸",
            (Pottery,     Lang::En) => "Pottery",
            (Sculpting,   Lang::Ja) => "彫刻",
            (Sculpting,   Lang::En) => "Sculpting",
            (Writing,     Lang::Ja) => "執筆",
            (Writing,     Lang::En) => "Writing",
            (Custom(_),   _)        => "",
        }
    }

    pub fn fighting_spec_label(spec: &super::FightingSpec, lang: Lang) -> &'static str {
        use super::FightingSpec::*;
        match (spec, lang) {
            (Axe,       Lang::Ja) => "斧",
            (Axe,       Lang::En) => "Axe",
            (Brawl,     Lang::Ja) => "格闘",
            (Brawl,     Lang::En) => "Brawl",
            (Chainsaw,  Lang::Ja) => "チェーンソー",
            (Chainsaw,  Lang::En) => "Chainsaw",
            (Flail,     Lang::Ja) => "フレイル",
            (Flail,     Lang::En) => "Flail",
            (Garrote,   Lang::Ja) => "絞殺ひも",
            (Garrote,   Lang::En) => "Garrote",
            (Spear,     Lang::Ja) => "槍",
            (Spear,     Lang::En) => "Spear",
            (Sword,     Lang::Ja) => "刀剣",
            (Sword,     Lang::En) => "Sword",
            (Whip,      Lang::Ja) => "鞭",
            (Whip,      Lang::En) => "Whip",
            (Custom(_), _)        => "",
        }
    }

    pub fn firearms_spec_label(spec: &super::FirearmsSpec, lang: Lang) -> &'static str {
        use super::FirearmsSpec::*;
        match (spec, lang) {
            (Bow,           Lang::Ja) => "弓",
            (Bow,           Lang::En) => "Bow",
            (Handgun,       Lang::Ja) => "拳銃",
            (Handgun,       Lang::En) => "Handgun",
            (HeavyWeapons,  Lang::Ja) => "重火器",
            (HeavyWeapons,  Lang::En) => "Heavy Weapons",
            (MachineGun,    Lang::Ja) => "機関銃",
            (MachineGun,    Lang::En) => "Machine Gun",
            (RifleShotgun,  Lang::Ja) => "ライフル/ショットガン",
            (RifleShotgun,  Lang::En) => "Rifle/Shotgun",
            (SubmachineGun, Lang::Ja) => "サブマシンガン",
            (SubmachineGun, Lang::En) => "Submachine Gun",
            (Custom(_),     _)        => "",
        }
    }

    pub fn pilot_spec_label(spec: &super::PilotSpec, lang: Lang) -> &'static str {
        use super::PilotSpec::*;
        match (spec, lang) {
            // --- 両時代共通 ---
            (Boat,       Lang::Ja) => "ボート",
            (Boat,       Lang::En) => "Boat",
            (SteamShip,  Lang::Ja) => "汽船",
            (SteamShip,  Lang::En) => "Steam Ship",
            (Sailboat,   Lang::Ja) => "帆船",
            (Sailboat,   Lang::En) => "Sailboat",
            (CivilProp,  Lang::Ja) => "民間プロペラ機",
            (CivilProp,  Lang::En) => "Civil Prop",
            // --- 1920s のみ ---
            (Balloon,    Lang::Ja) => "気球",
            (Balloon,    Lang::En) => "Balloon",
            (Dirigible,  Lang::Ja) => "飛行船",
            (Dirigible,  Lang::En) => "Dirigible",
            // --- Modern (1990s) のみ ---
            (CivilJet,   Lang::Ja) => "民間ジェット機",
            (CivilJet,   Lang::En) => "Civil Jet",
            (Airliner,   Lang::Ja) => "定期旅客機",
            (Airliner,   Lang::En) => "Airliner",
            (JetFighter, Lang::Ja) => "ジェット戦闘機",
            (JetFighter, Lang::En) => "Jet Fighter",
            (Helicopter, Lang::Ja) => "ヘリコプター",
            (Helicopter, Lang::En) => "Helicopter",
            (Custom(_),  _)        => "",
        }
    }

    pub fn science_spec_label(spec: &super::ScienceSpec, lang: Lang) -> &'static str {
        use super::ScienceSpec::*;
        match (spec, lang) {
            (Astronomy,    Lang::Ja) => "天文学",
            (Astronomy,    Lang::En) => "Astronomy",
            (Biology,      Lang::Ja) => "生物学",
            (Biology,      Lang::En) => "Biology",
            (Botany,       Lang::Ja) => "植物学",
            (Botany,       Lang::En) => "Botany",
            (Chemistry,    Lang::Ja) => "化学",
            (Chemistry,    Lang::En) => "Chemistry",
            (Cryptography, Lang::Ja) => "暗号学",
            (Cryptography, Lang::En) => "Cryptography",
            (Engineering,  Lang::Ja) => "工学",
            (Engineering,  Lang::En) => "Engineering",
            (Forensics,    Lang::Ja) => "法医学",
            (Forensics,    Lang::En) => "Forensics",
            (Geology,      Lang::Ja) => "地質学",
            (Geology,      Lang::En) => "Geology",
            (Mathematics,  Lang::Ja) => "数学",
            (Mathematics,  Lang::En) => "Mathematics",
            (Meteorology,  Lang::Ja) => "気象学",
            (Meteorology,  Lang::En) => "Meteorology",
            (Pharmacy,     Lang::Ja) => "薬学",
            (Pharmacy,     Lang::En) => "Pharmacy",
            (Physics,      Lang::Ja) => "物理学",
            (Physics,      Lang::En) => "Physics",
            (Zoology,      Lang::Ja) => "動物学",
            (Zoology,      Lang::En) => "Zoology",
            (Custom(_),    _)        => "",
        }
    }

    pub fn survival_spec_label(spec: &super::SurvivalSpec, lang: Lang) -> &'static str {
        use super::SurvivalSpec::*;
        match (spec, lang) {
            (Arctic,    Lang::Ja) => "北極",
            (Arctic,    Lang::En) => "Arctic",
            (Desert,    Lang::Ja) => "砂漠",
            (Desert,    Lang::En) => "Desert",
            (Sea,       Lang::Ja) => "海上",
            (Sea,       Lang::En) => "Sea",
            (Custom(_), _)        => "",
        }
    }

    // --- 導出値・判定カテゴリ ラベル ---

    pub fn build_rank_label(rank: super::BuildRank, lang: Lang) -> &'static str {
        use super::BuildRank::*;
        match (rank, lang) {
            (NegTwo,   _) => "-2",
            (NegOne,   _) => "-1",
            (Zero,     _) => "0",
            (PosOne,   _) => "+1",
            (PosTwo,   _) => "+2",
            (PosThree, _) => "+3",
            (PosFour,  _) => "+4",
            (PosFive,  _) => "+5",
        }
    }

    pub fn damage_bonus_dice_label(dice: super::DamageBonusDice, lang: Lang) -> &'static str {
        use super::DamageBonusDice::*;
        match (dice, lang) {
            (NegTwo,   _)        => "-2",
            (NegOne,   _)        => "-1",
            (None,     Lang::Ja) => "なし",
            (None,     Lang::En) => "None",
            (PosOnD4,  _)        => "+1D4",
            (PosOnD6,  _)        => "+1D6",
            (PosTwD6,  _)        => "+2D6",
            (PosThrD6, _)        => "+3D6",
            (PosForD6, _)        => "+4D6",
        }
    }

    pub fn move_base_label(mov: super::MoveBase, _lang: Lang) -> &'static str {
        use super::MoveBase::*;
        match mov {
            Seven => "7",
            Eight => "8",
            Nine  => "9",
        }
    }

    pub fn standard_of_living_label(sol: super::StandardOfLiving, lang: Lang) -> &'static str {
        use super::StandardOfLiving::*;
        match (sol, lang) {
            (Pauper,    Lang::Ja) => "惨め",
            (Pauper,    Lang::En) => "Pauper",
            (Poor,      Lang::Ja) => "貧乏",
            (Poor,      Lang::En) => "Poor",
            (Average,   Lang::Ja) => "平均",
            (Average,   Lang::En) => "Average",
            (Wealthy,   Lang::Ja) => "裕福",
            (Wealthy,   Lang::En) => "Wealthy",
            (Rich,      Lang::Ja) => "金持ち",
            (Rich,      Lang::En) => "Rich",
            (SuperRich, Lang::Ja) => "超大金持ち",
            (SuperRich, Lang::En) => "Super Rich",
        }
    }

    pub fn age_category_label(cat: super::AgeCategory, lang: Lang) -> &'static str {
        use super::AgeCategory::*;
        match (cat, lang) {
            (Teen,    Lang::Ja) => "10代 (15-19)",
            (Teen,    Lang::En) => "Teen (15-19)",
            (Young,   Lang::Ja) => "若年 (20-39)",
            (Young,   Lang::En) => "Young Adult (20-39)",
            (Middle,  Lang::Ja) => "中年 (40-49)",
            (Middle,  Lang::En) => "Middle-Aged (40-49)",
            (Senior,  Lang::Ja) => "熟年 (50-59)",
            (Senior,  Lang::En) => "Senior (50-59)",
            (Elderly, Lang::Ja) => "高齢 (60-69)",
            (Elderly, Lang::En) => "Elderly (60-69)",
            (Old,     Lang::Ja) => "老年 (70-79)",
            (Old,     Lang::En) => "Old (70-79)",
            (Ancient, Lang::Ja) => "超高齢 (80+)",
            (Ancient, Lang::En) => "Very Old (80+)",
        }
    }

    pub fn occupation_label(kind: OccupationKind, lang: Lang) -> &'static str {
        match (kind, lang) {
            (OccupationKind::Athlete, Lang::Ja) => "アスリート",
            (OccupationKind::Athlete, Lang::En) => "Athlete",
            (OccupationKind::Doctor,  Lang::Ja) => "医師",
            (OccupationKind::Doctor,  Lang::En) => "Doctor",
            _ => todo!("occupation label not yet defined"),
        }
    }
}
