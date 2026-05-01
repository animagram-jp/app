use crate::list::VariableList;

pub struct Instance {
    pub data: VariableList<u8>,
}

impl Instance {
    pub fn new() -> Self {
        Self { data: VariableList::new() }
    }
}

// model: 全フィールドをflatに列挙する識別子。VariableListのidentityとして直接使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    // --- 基本情報 ---
    Identity,
    Timestamp,
    Occupation,
    Age,

    // --- 能力値 ---
    Strength,
    Constitution,
    Size,
    Dexterity,
    Appearance,
    Intelligence,
    Power,
    Education,
    Luck,

    // --- 導出ステータス ---
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
    ArtCraft,
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
    FightingBrawl,
    FightingOther,
    FirearmsHandgun,
    FirearmsRifleShotgun,
    FirearmsOther,
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
    MechRepair,
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
}

// 職業識別子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OccupationKind {
    Athlete, Doctor, Engineer, Entertainer, Activist, Professor, Police, Detective, Artist,
    Antiquarian, Author, MilitaryOfficer, Librarian, Journalist, PrivateInvestigator,
    Clergy, Parapsychologist, Dilettante, Missionary, TribeMember, Farmer,
    Pilot, Hacker, Criminal, Soldier, Lawyer, Drifter, Musician
}

// display: instance/model/schemaと独立した表示ロジック
pub mod display {
    use super::{Model, OccupationKind};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Lang {
        En,
        Ja,
    }

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

// schema: Modelのグルーピングと導出ロジックを担う
// Instance = VariableList<u8>。数値はu16 little-endian 2バイト、文字列はUTF-8バイト列。
// get -> Result<T, ListError>: NotExistはフィールド未セットを意味する。
pub mod schema {
    use super::{Instance, Model};
    use crate::list::ListError;

    fn id(field: Model) -> usize {
        field.index()
    }

    // u16: little-endian 2バイトでget/set
    fn get_u16(instance: &Instance, field: Model) -> Result<u16, ListError> {
        let bytes = instance.data.get(&id(field))?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn set_u16(instance: &mut Instance, field: Model, value: u16) -> Result<(), ListError> {
        let bytes = value.to_le_bytes();
        instance.data.upsert(&id(field), &bytes)?;
        Ok(())
    }

    // 文字列: UTF-8バイト列でget/set
    fn get_str(instance: &Instance, field: Model) -> Result<String, ListError> {
        let bytes = instance.data.get(&id(field))?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    fn set_str(instance: &mut Instance, field: Model, value: &str) -> Result<(), ListError> {
        instance.data.upsert(&id(field), value.as_bytes())?;
        Ok(())
    }

    // --- 能力値 ---

    pub mod strength {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Strength) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Strength, v) }
    }

    pub mod constitution {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Constitution) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Constitution, v) }
    }

    pub mod size {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Size) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Size, v) }
    }

    pub mod dexterity {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Dexterity) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Dexterity, v) }
    }

    pub mod appearance {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Appearance) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Appearance, v) }
    }

    pub mod intelligence {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Intelligence) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Intelligence, v) }
    }

    pub mod power {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Power) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Power, v) }
    }

    pub mod education {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Education) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Education, v) }
    }

    pub mod luck {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Luck) }
        pub fn set(instance: &mut Instance, v: u16) -> Result<(), ListError> { set_u16(instance, Model::Luck, v) }
    }

    // --- 導出ステータス ---

    pub mod hit_points {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::HitPoints) }
        pub fn derive(instance: &Instance) -> Result<u16, ListError> {
            Ok((constitution::get(instance)? + size::get(instance)?) / 10)
        }
        pub fn set(instance: &mut Instance) -> Result<(), ListError> {
            let v = derive(instance)?;
            set_u16(instance, Model::HitPoints, v)
        }
    }

    pub mod magic_points {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::MagicPoints) }
        pub fn derive(instance: &Instance) -> Result<u16, ListError> {
            Ok(power::get(instance)? / 5)
        }
        pub fn set(instance: &mut Instance) -> Result<(), ListError> {
            let v = derive(instance)?;
            set_u16(instance, Model::MagicPoints, v)
        }
    }

    pub mod sanity {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Sanity) }
        // SAN初期値 = POW。能力値は×5済みなのでそのまま使う。
        pub fn derive(instance: &Instance) -> Result<u16, ListError> {
            power::get(instance)
        }
        pub fn set(instance: &mut Instance) -> Result<(), ListError> {
            let v = derive(instance)?;
            set_u16(instance, Model::Sanity, v)
        }
    }

    pub mod dodge {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::Dodge) }
        pub fn derive(instance: &Instance) -> Result<u16, ListError> {
            Ok(dexterity::get(instance)? / 2)
        }
        pub fn set(instance: &mut Instance) -> Result<(), ListError> {
            let v = derive(instance)?;
            set_u16(instance, Model::Dodge, v)
        }
    }

    pub mod language_own {
        use super::*;
        pub fn get(instance: &Instance) -> Result<u16, ListError> { get_u16(instance, Model::LanguageOwn) }
        pub fn derive(instance: &Instance) -> Result<u16, ListError> {
            education::get(instance)
        }
        pub fn set(instance: &mut Instance) -> Result<(), ListError> {
            let v = derive(instance)?;
            set_u16(instance, Model::LanguageOwn, v)
        }
    }

    // --- バックストーリー ---

    pub mod personal_description {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::PersonalDescription) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::PersonalDescription, v) }
    }

    pub mod ideology_and_beliefs {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::IdeologyAndBeliefs) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::IdeologyAndBeliefs, v) }
    }

    pub mod significant_people {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::SignificantPeople) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::SignificantPeople, v) }
    }

    pub mod meaningful_location {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::MeaningfulLocation) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::MeaningfulLocation, v) }
    }

    pub mod treasured_possessions {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::TreasuredPossessions) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::TreasuredPossessions, v) }
    }

    pub mod key_connection {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::KeyConnection) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::KeyConnection, v) }
    }

    pub mod trait_field {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::Trait) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::Trait, v) }
    }

    pub mod phobias_and_manias {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::PhobiasAndManias) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::PhobiasAndManias, v) }
    }

    pub mod arcane_tomes_and_spells {
        use super::*;
        pub fn get(instance: &Instance) -> Result<String, ListError> { get_str(instance, Model::ArcaneTomesAndSpells) }
        pub fn set(instance: &mut Instance, v: &str) -> Result<(), ListError> { set_str(instance, Model::ArcaneTomesAndSpells, v) }
    }

    // --- スキルグルーピング ---

    pub mod skill {
        use super::super::Model;
        use super::*;

        pub fn all() -> &'static [Model] {
            &[
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
            ]
        }

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

        pub fn get(instance: &Instance, field: Model) -> Result<u16, ListError> {
            get_u16(instance, field)
        }

        pub fn set(instance: &mut Instance, field: Model, v: u16) -> Result<(), ListError> {
            set_u16(instance, field, v)
        }
    }

    pub mod characteristic {
        use super::super::Model;
        use super::*;

        pub fn all() -> &'static [Model] {
            &[
                Model::Strength, Model::Constitution, Model::Size, Model::Dexterity,
                Model::Appearance, Model::Intelligence, Model::Power, Model::Education,
                Model::Luck,
            ]
        }

        // SIZ / INT / EDU は (2d6+6)×5、それ以外は 3d6×5
        pub fn roll(field: Model) -> u16 {
            match field {
                Model::Size | Model::Intelligence | Model::Education =>
                    (crate::n_d_n(2, 6) + 6) as u16 * 5,
                _ =>
                    crate::n_d_n(3, 6) as u16 * 5,
            }
        }

        // 全能力値をロールしてinstanceに書き込み、導出値も更新する
        pub fn roll_all(instance: &mut Instance) -> Result<(), ListError> {
            for &field in all() {
                let v = roll(field);
                set_u16(instance, field, v)?;
            }
            hit_points::set(instance)?;
            magic_points::set(instance)?;
            sanity::set(instance)?;
            dodge::set(instance)?;
            language_own::set(instance)?;
            Ok(())
        }
    }

    pub mod backstory {
        use super::super::Model;

        pub fn all() -> &'static [Model] {
            &[
                Model::KeyConnection, Model::PersonalDescription, Model::IdeologyAndBeliefs,
                Model::SignificantPeople, Model::MeaningfulLocation, Model::TreasuredPossessions,
                Model::Trait, Model::PhobiasAndManias, Model::ArcaneTomesAndSpells,
            ]
        }
    }
}
