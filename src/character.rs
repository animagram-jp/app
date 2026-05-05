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

pub enum Occupation { // p.38
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

impl ArtCraftSpec {
    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Acting,      Lang::Ja) => "演劇",
            (Self::Acting,      Lang::En) => "Acting",
            (Self::Barber,      Lang::Ja) => "理容",
            (Self::Barber,      Lang::En) => "Barber",
            (Self::Calligraphy, Lang::Ja) => "書道",
            (Self::Calligraphy, Lang::En) => "Calligraphy",
            (Self::Carpentry,   Lang::Ja) => "大工仕事",
            (Self::Carpentry,   Lang::En) => "Carpentry",
            (Self::Cobbling,    Lang::Ja) => "靴製造",
            (Self::Cobbling,    Lang::En) => "Cobbling",
            (Self::Cook,        Lang::Ja) => "料理",
            (Self::Cook,        Lang::En) => "Cook",
            (Self::Dancing,     Lang::Ja) => "踊り",
            (Self::Dancing,     Lang::En) => "Dancing",
            (Self::FineArt,     Lang::Ja) => "絵画",
            (Self::FineArt,     Lang::En) => "Fine Art",
            (Self::Forgery,     Lang::Ja) => "文書偽造",
            (Self::Forgery,     Lang::En) => "Forgery",
            (Self::Photography, Lang::Ja) => "写真術",
            (Self::Photography, Lang::En) => "Photography",
            (Self::Pottery,     Lang::Ja) => "陶芸",
            (Self::Pottery,     Lang::En) => "Pottery",
            (Self::Sculpting,   Lang::Ja) => "彫刻",
            (Self::Sculpting,   Lang::En) => "Sculpting",
            (Self::Writing,     Lang::Ja) => "執筆",
            (Self::Writing,     Lang::En) => "Writing",
            (Self::Custom(s),   _)        => s.as_str(),
        }
    }
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

impl FightingSpec {
    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Axe,       Lang::Ja) => "斧",
            (Self::Axe,       Lang::En) => "Axe",
            (Self::Brawl,     Lang::Ja) => "格闘",
            (Self::Brawl,     Lang::En) => "Brawl",
            (Self::Chainsaw,  Lang::Ja) => "チェーンソー",
            (Self::Chainsaw,  Lang::En) => "Chainsaw",
            (Self::Flail,     Lang::Ja) => "フレイル",
            (Self::Flail,     Lang::En) => "Flail",
            (Self::Garrote,   Lang::Ja) => "絞殺ひも",
            (Self::Garrote,   Lang::En) => "Garrote",
            (Self::Spear,     Lang::Ja) => "槍",
            (Self::Spear,     Lang::En) => "Spear",
            (Self::Sword,     Lang::Ja) => "刀剣",
            (Self::Sword,     Lang::En) => "Sword",
            (Self::Whip,      Lang::Ja) => "鞭",
            (Self::Whip,      Lang::En) => "Whip",
            (Self::Custom(s), _)        => s.as_str(),
        }
    }
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

impl FirearmsSpec {
    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Bow,           Lang::Ja) => "弓",
            (Self::Bow,           Lang::En) => "Bow",
            (Self::Handgun,       Lang::Ja) => "拳銃",
            (Self::Handgun,       Lang::En) => "Handgun",
            (Self::HeavyWeapons,  Lang::Ja) => "重火器",
            (Self::HeavyWeapons,  Lang::En) => "Heavy Weapons",
            (Self::MachineGun,    Lang::Ja) => "機関銃",
            (Self::MachineGun,    Lang::En) => "Machine Gun",
            (Self::RifleShotgun,  Lang::Ja) => "ライフル/ショットガン",
            (Self::RifleShotgun,  Lang::En) => "Rifle/Shotgun",
            (Self::SubmachineGun, Lang::Ja) => "サブマシンガン",
            (Self::SubmachineGun, Lang::En) => "Submachine Gun",
            (Self::Custom(s),     _)        => s.as_str(),
        }
    }
}

// --- ほかの言語 (Language Other) 専門分野 ---
// 言語名を自由記入。母国語 (LanguageOwn) は専門分野なし (初期値 = EDU)。
enum LanguageSpec {
    Custom(String),
}

// --- 操縦 (Pilot) 専門分野 ---
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

impl PilotSpec {
    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            // --- 両時代共通 ---
            (Self::Boat,       Lang::Ja) => "ボート",
            (Self::Boat,       Lang::En) => "Boat",
            (Self::SteamShip,  Lang::Ja) => "汽船",
            (Self::SteamShip,  Lang::En) => "Steam Ship",
            (Self::Sailboat,   Lang::Ja) => "帆船",
            (Self::Sailboat,   Lang::En) => "Sailboat",
            (Self::CivilProp,  Lang::Ja) => "民間プロペラ機",
            (Self::CivilProp,  Lang::En) => "Civil Prop",
            // --- 1920s のみ ---
            (Self::Balloon,    Lang::Ja) => "気球",
            (Self::Balloon,    Lang::En) => "Balloon",
            (Self::Dirigible,  Lang::Ja) => "飛行船",
            (Self::Dirigible,  Lang::En) => "Dirigible",
            // --- Modern (1990s) のみ ---
            (Self::CivilJet,   Lang::Ja) => "民間ジェット機",
            (Self::CivilJet,   Lang::En) => "Civil Jet",
            (Self::Airliner,   Lang::Ja) => "定期旅客機",
            (Self::Airliner,   Lang::En) => "Airliner",
            (Self::JetFighter, Lang::Ja) => "ジェット戦闘機",
            (Self::JetFighter, Lang::En) => "Jet Fighter",
            (Self::Helicopter, Lang::Ja) => "ヘリコプター",
            (Self::Helicopter, Lang::En) => "Helicopter",
            (Self::Custom(s),  _)        => s.as_str(),
        }
    }
}

// --- 科学 (Science) 専門分野 --- p.59
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

impl ScienceSpec {
    pub fn label(&self, lang: Lang) -> &str {
        match (spec, lang) {
            (Self::Astronomy,    Lang::Ja) => "天文学",
            (Self::Astronomy,    Lang::En) => "Astronomy",
            (Self::Biology,      Lang::Ja) => "生物学",
            (Self::Biology,      Lang::En) => "Biology",
            (Self::Botany,       Lang::Ja) => "植物学",
            (Self::Botany,       Lang::En) => "Botany",
            (Self::Chemistry,    Lang::Ja) => "化学",
            (Self::Chemistry,    Lang::En) => "Chemistry",
            (Self::Cryptography, Lang::Ja) => "暗号学",
            (Self::Cryptography, Lang::En) => "Cryptography",
            (Self::Engineering,  Lang::Ja) => "工学",
            (Self::Engineering,  Lang::En) => "Engineering",
            (Self::Forensics,    Lang::Ja) => "法医学",
            (Self::Forensics,    Lang::En) => "Forensics",
            (Self::Geology,      Lang::Ja) => "地質学",
            (Self::Geology,      Lang::En) => "Geology",
            (Self::Mathematics,  Lang::Ja) => "数学",
            (Self::Mathematics,  Lang::En) => "Mathematics",
            (Self::Meteorology,  Lang::Ja) => "気象学",
            (Self::Meteorology,  Lang::En) => "Meteorology",
            (Self::Pharmacy,     Lang::Ja) => "薬学",
            (Self::Pharmacy,     Lang::En) => "Pharmacy",
            (Self::Physics,      Lang::Ja) => "物理学",
            (Self::Physics,      Lang::En) => "Physics",
            (Self::Zoology,      Lang::Ja) => "動物学",
            (Self::Zoology,      Lang::En) => "Zoology",
            (Self::Custom(s), _)  => s.as_str(),
        }
    }
}

// --- サバイバル (Survival) 専門分野 --- p.63
enum SurvivalSpec {
    Arctic,
    Desert,
    Sea,
    Custom(String),
}

impl SurvivalSpec {
    pub fn label(self, lang: Lang) -> &str {
        match (spec, lang) {
            (Self::Arctic,    Lang::Ja) => "極地",
            (Self::Arctic,    Lang::En) => "Arctic",
            (Self::Desert,    Lang::Ja) => "砂漠",
            (Self::Desert,    Lang::En) => "Desert",
            (Self::Sea,       Lang::Ja) => "海",
            (Self::Sea,       Lang::En) => "Sea",
            (Self::Custom(s),        _) => s.as_str(),
        }
    }
}

// --- スキル (Skill) --- p.54
enum Skill {
    Accounting,
    Anthropology,
    Archaeology,
    Appraise,
    // 専門分野あり（ルールブック定義済み選択肢 + 自由記入）
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
    Custom { name: String, spec: Option<String> },
}

impl Skill {
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
            (Self::ArtCraft(spec),       _)        => format!("芸術/製作 ({})", spec.label(lang)),
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
            (Self::Fighting(spec),       _)        => format!("近接戦闘 ({})", spec.label(lang)),
            (Self::Firearms(spec),       _)        => format!("射撃 ({})", spec.label(lang)),
            (Self::FirstAid,             Lang::Ja) => "応急手当".into(),
            (Self::FirstAid,             Lang::En) => "First Aid".into(),
            (Self::History,              Lang::Ja) => "歴史".into(),
            (Self::History,              Lang::En) => "History".into(),
            (Self::Intimidate,           Lang::Ja) => "威圧".into(),
            (Self::Intimidate,           Lang::En) => "Intimidate".into(),
            (Self::Jump,                 Lang::Ja) => "跳躍".into(),
            (Self::Jump,                 Lang::En) => "Jump".into(),
            (Self::LanguageOther(spec),  _)        => format!("ほかの言語 ({})", spec.label(lang)),
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
            (Self::Pilot(spec),          _)        => format!("操縦 ({})", spec.label(lang)),
            (Self::Psychoanalysis,       Lang::Ja) => "精神分析".into(),
            (Self::Psychoanalysis,       Lang::En) => "Psychoanalysis".into(),
            (Self::Psychology,           Lang::Ja) => "心理学".into(),
            (Self::Psychology,           Lang::En) => "Psychology".into(),
            (Self::Ride,                 Lang::Ja) => "乗馬".into(),
            (Self::Ride,                 Lang::En) => "Ride".into(),
            (Self::Science(spec),        _)        => format!("科学 ({})", spec.label(lang)),
            (Self::SleightOfHand,        Lang::Ja) => "手さばき".into(),
            (Self::SleightOfHand,        Lang::En) => "Sleight of Hand".into(),
            (Self::SpotHidden,           Lang::Ja) => "目星".into(),
            (Self::SpotHidden,           Lang::En) => "Spot Hidden".into(),
            (Self::Stealth,              Lang::Ja) => "隠密".into(),
            (Self::Stealth,              Lang::En) => "Stealth".into(),
            (Self::Survival(spec),       _)        => format!("サバイバル ({})", spec.label(lang)),
            (Self::Swim,                 Lang::Ja) => "水泳".into(),
            (Self::Swim,                 Lang::En) => "Swim".into(),
            (Self::Throw,                Lang::Ja) => "投擲".into(),
            (Self::Throw,                Lang::En) => "Throw".into(),
            (Self::Track,                Lang::Ja) => "追跡".into(),
            (Self::Track,                Lang::En) => "Track".into(),
            (Self::Custom { name, spec: Some(s) }, _) => format!("{} ({})", name, s),
            (Self::Custom { name, spec: None },   _) => name.clone(),
        }
    }
}

// ============================================================
// --- CoC 7th 導出値・判定カテゴリ ---
// ============================================================

// --- ビルド (Build) ---
// STR + SIZ の合計値から決定される離散段階。DamageBonus と 1対1 対応する。
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

    pub fn value(&self) -> i8 {
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

    pub fn damage_bonus(&self) -> DamageBonus {
        match self {
            Self::NegTwo   => DamageBonus::NegTwo,
            Self::NegOne   => DamageBonus::NegOne,
            Self::Zero     => DamageBonus::None,
            Self::PosOne   => DamageBonus::PosOnD4,
            Self::PosTwo   => DamageBonus::PosOnD6,
            Self::PosThree => DamageBonus::PosTwD6,
            Self::PosFour  => DamageBonus::PosThrD6,
            Self::PosFive  => DamageBonus::PosForD6,
        }
    }
}

// --- ダメージボーナス (DamageBonus) ---
enum DamageBonus {
    NegTwo,   // -2    (Build -2)
    NegOne,   // -1    (Build -1)
    Zero,     // なし   (Build  0)
    PosOnD4,  // +1D4  (Build +1)
    PosOnD6,  // +1D6  (Build +2)
    PosTwD6,  // +2D6  (Build +3)
    PosThrD6, // +3D6  (Build +4)
    PosForD6, // +4D6  (Build +5)
}

impl DamageBonus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NegTwo   => "-2",
            Self::NegOne   => "-1",
            Self::Zero     => "0",
            Self::PosOnD4  => "+1D4",
            Self::PosOnD6  => "+1D6",
            Self::PosTwD6  => "+2D6",
            Self::PosThrD6 => "+3D6",
            Self::PosForD6 => "+4D6",
        }
    }
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

    pub fn label(&self) -> &'static str {
        match self {
            Seven => "7",
            Eight => "8",
            Nine  => "9",
        }
    }
}

// --- 生活水準 (Standard of Living) ---
// 信用 (Credit Rating) の値から決定される区分。
enum StandardOfLiving {
    Pauper,    // 無一文  (CR: 0     )
    Poor,      // 貧乏    (CR: 1-  9 )
    Average,   // 平均    (CR: 10- 49)
    Wealthy,   // 裕福    (CR: 50- 89)
    Rich,      // 富豪    (CR: 90- 98)
    SuperRich, // 大富豪  (CR: 99    )
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
    pub fn label(self, lang: Lang) -> &'static str {
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

    pub fn label(&self, lang: Lang) -> &'static str {
        match (cat, lang) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    
    // --- system ---
    Identity: u32,
    Timestamp::Created: u64, // datetime.rs
    Timestamp::Updated: u64, // datetime.rs

    // --- primary but not required ---
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

}


