use crate::Lang;

// ============================================================
// --- キャラクター (Character) ---
// ============================================================

pub enum Character {
    Profile(Profile),
    Characteristic(Characteristic),
    Derived(Derived),
    Skill(Skill),
    Equipment(Equipment),
    Backstory(Backstory),
}

impl Character {
    pub fn label(&self, lang: Lang) -> String {
        match self {
            Self::Profile(p)        => p.label(lang).to_string(),
            Self::Characteristic(c) => c.label(lang).to_string(),
            Self::Derived(d)        => d.label(lang).to_string(),
            Self::Skill(s)          => s.label(lang),
            Self::Equipment(e)      => e.label(lang).to_string(),
            Self::Backstory(b)      => b.label(lang).to_string(),
        }
    }
    pub fn id(&self) -> usize {
        match self {
            Self::Profile(p)        => p.id( 10),  //  10- 15 (6件)
            Self::Characteristic(c) => c.id( 20),  //  20- 28 (9件)
            Self::Derived(d)        => d.id( 30),  //  30- 37 (8件)
            Self::Skill(s)          => s.id( 40),  //  40- 86 (47件)
            Self::Equipment(e)      => e.id( 90),  //  90-... (拡張余地)
            Self::Backstory(b)      => b.id(100),  // 100-109 (10件)
        }
    }
}


// ============================================================
// --- プロフィール (Name, Birthppalce, Pronoun, Occupation, Residence, Age) ---
// ============================================================

#[derive(Clone, Copy)]
pub enum Profile {
    Name, // todo: 「名前」と「Option(呼び方)」の二値構成に拡充。labelは format!"{} ({})"。
    Birthpalce,
    Pronoun,
    Occupation, // todo: 「ルール上の職業」と「Option(肩書 title)」の二値構成に拡充。 labelは format!"{} ({})"。
    Residence,
    Age,
}

impl Profile {
    pub fn id(&self, base: usize) -> usize {
        base + match self {
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
    Luck,
}

impl Characteristic {
    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::Strength     => 0,
            Self::Constitution => 1,
            Self::Size         => 2,
            Self::Dexterity    => 3,
            Self::Appearance   => 4,
            Self::Intelligence => 5,
            Self::Power        => 6,
            Self::Education    => 7,
            Self::Luck         => 8,
        }
    }

    /// [base, delta, bonus] → 12バイト (各4バイト LE i32)
    pub fn encode(vals: [i32; 3]) -> Vec<u8> {
        vals.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    /// 12バイト → [base, delta, bonus]
    pub fn decode(bytes: &[u8]) -> [i32; 3] {
        std::array::from_fn(|i| {
            let s = i * 4;
            bytes.get(s..s + 4)
                .and_then(|b| b.try_into().ok())
                .map(i32::from_le_bytes)
                .unwrap_or(0)
        })
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
            (Self::Luck,  Lang::Ja) => "幸運",
            (Self::Luck,  Lang::En) => "Luck",
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
            Self::Luck,
        ]
    }

    pub fn generate(&self) -> u16 {
        // SIZ / INT / EDU は (2d6+6)×5、それ以外は 3d6×5
        match self {
            Self::Size | Self::Intelligence | Self::Education => 
                (crate::n_d_n(2, 6) + 6) as u16 * 5,
            _ => 
                crate::n_d_n(3, 6) as u16 * 5,
        }
    }
}

// ============================================================
// --- スキル (Skill) ---
// ============================================================


// --- 信用 (Credit Rating) ---
#[derive(Clone, Copy)]
pub struct CreditRating;

impl CreditRating {
    pub fn standard_of_living(&self, value: u16) -> StandardOfLiving {
        StandardOfLiving::from_cr(value)
    }
}

/// 各Spec enumのCustomスロット数。追加ルールブック対応時にここだけ変える。
pub const SPEC_CUSTOM_SLOTS: usize = 4;

/// selectのoption生成用。固定variantのlabelを返す。
pub trait SpecLabel {
    fn spec_label(&self, lang: Lang) -> &str;
}

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
    CreditRating(CreditRating),
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
            Self::ArtCraft(ArtCraftSpec::Acting),   // spec持ち代表
            Self::Charm,
            Self::Climb,
            Self::ComputerUse,
            Self::CreditRating(CreditRating),
            Self::CthulhuMythos,
            Self::Disguise,
            Self::Dodge,
            Self::DriveAuto,
            Self::ElecRepair,
            Self::Electronics,
            Self::FastTalk,
            Self::Fighting(FightingSpec::Brawl),    // spec持ち代表
            Self::Firearms(FirearmsSpec::Handgun),  // spec持ち代表
            Self::FirstAid,
            Self::History,
            Self::Intimidate,
            Self::Jump,
            Self::LanguageOther(LanguageSpec::Custom0(String::new())), // 全部自由記入
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
            Self::Pilot(PilotSpec::Boat),           // spec持ち代表
            Self::Psychoanalysis,
            Self::Psychology,
            Self::Ride,
            Self::Science(ScienceSpec::Biology),    // spec持ち代表
            Self::SleightOfHand,
            Self::SpotHidden,
            Self::Stealth,
            Self::Survival(SurvivalSpec::Sea),      // spec持ち代表
            Self::Swim,
            Self::Throw,
            Self::Track,
            Self::Custom { slot: 0, name: String::new(), spec: None },
        ]
    }

    pub fn id(&self, base: usize) -> usize {
        // spec有り: Specにbaseを伝播してSpec内でIDを確定する
        // spec無し: base+100 以降に配置（Specオフセット域 0..100 と衝突しない）
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
            Self::CreditRating(_)     => base + 107,
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
            Self::Custom { slot, .. } => base + 140 + slot,
        }
    }

    /// [occ: u16 LE][int: u16 LE][bonus: i32 LE][spec_len: u16 LE][spec: utf8...]
    pub fn encode(occ: u16, int: u16, bonus: i32, spec: Option<&str>) -> Vec<u8> {
        let spec_bytes = spec.unwrap_or("").as_bytes();
        let mut b = Vec::with_capacity(10 + spec_bytes.len());
        b.extend_from_slice(&occ.to_le_bytes());
        b.extend_from_slice(&int.to_le_bytes());
        b.extend_from_slice(&bonus.to_le_bytes());
        b.extend_from_slice(&(spec_bytes.len() as u16).to_le_bytes());
        b.extend_from_slice(spec_bytes);
        b
    }

    /// → (occ, int, bonus, spec)
    pub fn decode(bytes: &[u8]) -> (u16, u16, i32, String) {
        let occ   = bytes.get(0..2).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0);
        let int   = bytes.get(2..4).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0);
        let bonus = bytes.get(4..8).and_then(|b| b.try_into().ok()).map(i32::from_le_bytes).unwrap_or(0);
        let spec_len = bytes.get(8..10).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes).unwrap_or(0) as usize;
        let spec = bytes.get(10..10 + spec_len)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        (occ, int, bonus, spec)
    }

    /// spec有りvariantのデフォルト専門分野ラベルを返す。spec無しはNone。
    pub fn spec_label(&self, lang: Lang) -> Option<String> {
        match self {
            Self::ArtCraft(spec)      => Some(spec.label(lang).to_string()),
            Self::Fighting(spec)      => Some(spec.label(lang).to_string()),
            Self::Firearms(spec)      => Some(spec.label(lang).to_string()),
            Self::LanguageOther(spec) => Some(spec.label(lang).to_string()),
            Self::Pilot(spec)         => Some(spec.label(lang).to_string()),
            Self::Science(spec)       => Some(spec.label(lang).to_string()),
            Self::Survival(spec)      => Some(spec.label(lang).to_string()),
            Self::Custom { spec, .. } => spec.as_deref().map(str::to_string),
            // CreditRating も spec 表示なし（label()内で処理済み）
            _                         => None,
        }
    }

    /// spec文字列を受け取り "スキル名 (spec)" を返す。specが空ならスキル名のみ。
    pub fn label_with_spec(&self, lang: Lang, spec: &str) -> String {
        let base = self.label(lang);
        if spec.is_empty() { base } else { format!("{} ({})", base, spec) }
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
            Self::CreditRating(_)      =>  0,
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
            Self::LanguageOther(spec)  =>  1,
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
            (Self::ArtCraft(spec),       _)        => { let s = spec.label(lang); if s.is_empty() { "芸術/製作".into() } else { format!("芸術/製作 ({})", s) } }
            (Self::Charm,                Lang::Ja) => "魅惑".into(),
            (Self::Charm,                Lang::En) => "Charm".into(),
            (Self::Climb,                Lang::Ja) => "登攀".into(),
            (Self::Climb,                Lang::En) => "Climb".into(),
            (Self::ComputerUse,          Lang::Ja) => "コンピューター".into(),
            (Self::ComputerUse,          Lang::En) => "Computer Use".into(),
            (Self::CreditRating(_),      Lang::Ja) => "信用".into(),
            (Self::CreditRating(_),      Lang::En) => "Credit Rating".into(),
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
            (Self::Fighting(spec),       _)        => { let s = spec.label(lang); if s.is_empty() { "近接戦闘".into() } else { format!("近接戦闘 ({})", s) } }
            (Self::Firearms(spec),       _)        => { let s = spec.label(lang); if s.is_empty() { "射撃".into() } else { format!("射撃 ({})", s) } }
            (Self::FirstAid,             Lang::Ja) => "応急手当".into(),
            (Self::FirstAid,             Lang::En) => "First Aid".into(),
            (Self::History,              Lang::Ja) => "歴史".into(),
            (Self::History,              Lang::En) => "History".into(),
            (Self::Intimidate,           Lang::Ja) => "威圧".into(),
            (Self::Intimidate,           Lang::En) => "Intimidate".into(),
            (Self::Jump,                 Lang::Ja) => "跳躍".into(),
            (Self::Jump,                 Lang::En) => "Jump".into(),
            (Self::LanguageOther(spec),  _)        => { let s = spec.label(lang); if s.is_empty() { "ほかの言語".into() } else { format!("ほかの言語 ({})", s) } }
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
            (Self::Pilot(spec),          _)        => { let s = spec.label(lang); if s.is_empty() { "操縦".into() } else { format!("操縦 ({})", s) } }
            (Self::Psychoanalysis,       Lang::Ja) => "精神分析".into(),
            (Self::Psychoanalysis,       Lang::En) => "Psychoanalysis".into(),
            (Self::Psychology,           Lang::Ja) => "心理学".into(),
            (Self::Psychology,           Lang::En) => "Psychology".into(),
            (Self::Ride,                 Lang::Ja) => "乗馬".into(),
            (Self::Ride,                 Lang::En) => "Ride".into(),
            (Self::Science(spec),        _)        => { let s = spec.label(lang); if s.is_empty() { "科学".into() } else { format!("科学 ({})", s) } }
            (Self::SleightOfHand,        Lang::Ja) => "手さばき".into(),
            (Self::SleightOfHand,        Lang::En) => "Sleight of Hand".into(),
            (Self::SpotHidden,           Lang::Ja) => "目星".into(),
            (Self::SpotHidden,           Lang::En) => "Spot Hidden".into(),
            (Self::Stealth,              Lang::Ja) => "隠密".into(),
            (Self::Stealth,              Lang::En) => "Stealth".into(),
            (Self::Survival(spec),       _)        => { let s = spec.label(lang); if s.is_empty() { "サバイバル".into() } else { format!("サバイバル ({})", s) } }
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

// --- 芸術/製作 専門分野 (Art/Craft Specialization)  --- p.62
#[derive(Clone)]
pub enum ArtCraftSpec {
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
    Custom0(String), Custom1(String), Custom2(String), Custom3(String),
}

impl crate::character::SpecLabel for ArtCraftSpec {
    fn spec_label(&self, lang: Lang) -> &str { self.label(lang) }
}

impl ArtCraftSpec {
    pub fn list() -> &'static [Self] {
        &[
            Self::Acting, Self::Barber, Self::Calligraphy, Self::Carpentry,
            Self::Cobbling, Self::Cook, Self::Dancing, Self::FineArt,
            Self::Forgery, Self::Photography, Self::Pottery, Self::Sculpting,
            Self::Writing,
        ]
    }

    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::Acting      =>  0,
            Self::Barber      =>  1,
            Self::Calligraphy =>  2,
            Self::Carpentry   =>  3,
            Self::Cobbling    =>  4,
            Self::Cook        =>  5,
            Self::Dancing     =>  6,
            Self::FineArt     =>  7,
            Self::Forgery     =>  8,
            Self::Photography =>  9,
            Self::Pottery     => 10,
            Self::Sculpting   => 11,
            Self::Writing     => 12,
            Self::Custom0(_)  => 13,
            Self::Custom1(_)  => 14,
            Self::Custom2(_)  => 15,
            Self::Custom3(_)  => 16,
        }
    }

    pub fn base_value(&self) -> u16 { 5 }

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
            (Self::Custom0(s) | Self::Custom1(s) | Self::Custom2(s) | Self::Custom3(s), _) => s.as_str(),
        }
    }
}

// --- 近接戦闘 専門分野 (Fighting Specialization) --- p.61
#[derive(Clone)]
pub enum FightingSpec {
    Axe,          // 斧          15%
    Brawl,        // 格闘        25%
    Chainsaw,     // チェーンソー  10%
    Flail,        // フレイル     10%
    Garrote,      // 絞殺ひも     15%
    Spear,        // 槍          20%
    Sword,        // 刀剣        20%
    Whip,         // 鞭          05%
    Custom0 { name: String, base_value: u16 },
    Custom1 { name: String, base_value: u16 },
    Custom2 { name: String, base_value: u16 },
    Custom3 { name: String, base_value: u16 },
}

impl crate::character::SpecLabel for FightingSpec {
    fn spec_label(&self, lang: Lang) -> &str { self.label(lang) }
}

impl FightingSpec {
    pub fn list() -> &'static [Self] {
        &[Self::Axe, Self::Brawl, Self::Chainsaw, Self::Flail,
          Self::Garrote, Self::Spear, Self::Sword, Self::Whip]
    }

    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::Axe        => 0,
            Self::Brawl      => 1,
            Self::Chainsaw   => 2,
            Self::Flail      => 3,
            Self::Garrote    => 4,
            Self::Spear      => 5,
            Self::Sword      => 6,
            Self::Whip       => 7,
            Self::Custom0 { .. } => 8,
            Self::Custom1 { .. } => 9,
            Self::Custom2 { .. } => 10,
            Self::Custom3 { .. } => 11,
        }
    }

    pub fn base_value(&self) -> u16 {
        match self {
            Self::Axe                               => 15,
            Self::Brawl                             => 25,
            Self::Chainsaw                          => 10,
            Self::Flail                             => 10,
            Self::Garrote                           => 15,
            Self::Spear                             => 20,
            Self::Sword                             => 20,
            Self::Whip                              =>  5,
            Self::Custom0 { base_value, .. }
            | Self::Custom1 { base_value, .. }
            | Self::Custom2 { base_value, .. }
            | Self::Custom3 { base_value, .. }      => *base_value,
        }
    }

    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Axe,      Lang::Ja) => "斧",
            (Self::Axe,      Lang::En) => "Axe",
            (Self::Brawl,    Lang::Ja) => "格闘",
            (Self::Brawl,    Lang::En) => "Brawl",
            (Self::Chainsaw, Lang::Ja) => "チェーンソー",
            (Self::Chainsaw, Lang::En) => "Chainsaw",
            (Self::Flail,    Lang::Ja) => "フレイル",
            (Self::Flail,    Lang::En) => "Flail",
            (Self::Garrote,  Lang::Ja) => "絞殺ひも",
            (Self::Garrote,  Lang::En) => "Garrote",
            (Self::Spear,    Lang::Ja) => "槍",
            (Self::Spear,    Lang::En) => "Spear",
            (Self::Sword,    Lang::Ja) => "刀剣",
            (Self::Sword,    Lang::En) => "Sword",
            (Self::Whip,     Lang::Ja) => "鞭",
            (Self::Whip,     Lang::En) => "Whip",
            (Self::Custom0 { name, .. } | Self::Custom1 { name, .. }
            | Self::Custom2 { name, .. } | Self::Custom3 { name, .. }, _) => name.as_str(),
        }
    }
}

// --- 射撃 専門分野 (Firearms Specialization) --- p.64
#[derive(Clone)]
pub enum FirearmsSpec {
    Bow,           // 弓                   15%
    Handgun,       // 拳銃                 20%
    HeavyWeapons,  // 重火器               10%
    MachineGun,    // 機関銃               10%
    RifleShotgun,  // ライフル/ショットガン  25%
    SubmachineGun, // サブマシンガン         15%
    Custom0 { name: String, base_value: u16 },
    Custom1 { name: String, base_value: u16 },
    Custom2 { name: String, base_value: u16 },
    Custom3 { name: String, base_value: u16 },
}

impl crate::character::SpecLabel for FirearmsSpec {
    fn spec_label(&self, lang: Lang) -> &str { self.label(lang) }
}

impl FirearmsSpec {
    pub fn list() -> &'static [Self] {
        &[Self::Bow, Self::Handgun, Self::HeavyWeapons,
          Self::MachineGun, Self::RifleShotgun, Self::SubmachineGun]
    }

    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::Bow           => 0,
            Self::Handgun       => 1,
            Self::HeavyWeapons  => 2,
            Self::MachineGun    => 3,
            Self::RifleShotgun  => 4,
            Self::SubmachineGun => 5,
            Self::Custom0 { .. } => 6,
            Self::Custom1 { .. } => 7,
            Self::Custom2 { .. } => 8,
            Self::Custom3 { .. } => 9,
        }
    }

    pub fn base_value(&self) -> u16 {
        match self {
            Self::Bow                               => 15,
            Self::Handgun                           => 20,
            Self::HeavyWeapons                      => 10,
            Self::MachineGun                        => 10,
            Self::RifleShotgun                      => 25,
            Self::SubmachineGun                     => 15,
            Self::Custom0 { base_value, .. }
            | Self::Custom1 { base_value, .. }
            | Self::Custom2 { base_value, .. }
            | Self::Custom3 { base_value, .. }      => *base_value,
        }
    }

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
            (Self::Custom0 { name, .. } | Self::Custom1 { name, .. }
            | Self::Custom2 { name, .. } | Self::Custom3 { name, .. }, _) => name.as_str(),
        }
    }
}

// --- ほかの言語 専門分野 (Language Other Specialization) ---
#[derive(Clone)]
pub enum LanguageSpec {
    Custom0(String), Custom1(String), Custom2(String), Custom3(String),
}

impl LanguageSpec {
    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::Custom0(_) => 0,
            Self::Custom1(_) => 1,
            Self::Custom2(_) => 2,
            Self::Custom3(_) => 3,
        }
    }

    pub fn label(&self, _lang: Lang) -> &str {
        match self {
            Self::Custom0(s) | Self::Custom1(s)
            | Self::Custom2(s) | Self::Custom3(s) => s.as_str(),
        }
    }
}

// --- 操縦 専門分野 (Pilot Specialization) --- p.67
#[derive(Clone)]
pub enum PilotSpec {
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
    Custom0(String), Custom1(String), Custom2(String), Custom3(String),
}

impl crate::character::SpecLabel for PilotSpec {
    fn spec_label(&self, lang: Lang) -> &str { self.label(lang) }
}

impl PilotSpec {
    pub fn list() -> &'static [Self] {
        &[Self::Boat, Self::SteamShip, Self::Sailboat, Self::CivilProp,
          Self::Balloon, Self::Dirigible, Self::CivilJet, Self::Airliner,
          Self::JetFighter, Self::Helicopter]
    }

    pub fn id(&self, base: usize) -> usize {
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
            Self::Custom0(_) => 10,
            Self::Custom1(_) => 11,
            Self::Custom2(_) => 12,
            Self::Custom3(_) => 13,
        }
    }

    pub fn base_value(&self) -> u16 { 1 }

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
            (Self::Airliner,   Lang::Ja) => "旅客機",
            (Self::Airliner,   Lang::En) => "Airliner",
            (Self::JetFighter, Lang::Ja) => "ジェット戦闘機",
            (Self::JetFighter, Lang::En) => "Jet Fighter",
            (Self::Helicopter, Lang::Ja) => "ヘリコプター",
            (Self::Helicopter, Lang::En) => "Helicopter",
            (Self::Custom0(s) | Self::Custom1(s)
            | Self::Custom2(s) | Self::Custom3(s), _) => s.as_str(),
        }
    }
}

// --- 科学 専門分野 (Science Specialization) --- p.59
#[derive(Clone)]
pub enum ScienceSpec {
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
    Custom0(String), Custom1(String), Custom2(String), Custom3(String),
}

impl crate::character::SpecLabel for ScienceSpec {
    fn spec_label(&self, lang: Lang) -> &str { self.label(lang) }
}

impl ScienceSpec {
    pub fn list() -> &'static [Self] {
        &[Self::Astronomy, Self::Biology, Self::Botany, Self::Chemistry,
          Self::Cryptography, Self::Engineering, Self::Forensics, Self::Geology,
          Self::Mathematics, Self::Meteorology, Self::Pharmacy, Self::Physics,
          Self::Zoology]
    }

    pub fn id(&self, base: usize) -> usize {
        base + match self {
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
            Self::Custom0(_)   => 13,
            Self::Custom1(_)   => 14,
            Self::Custom2(_)   => 15,
            Self::Custom3(_)   => 16,
        }
    }

    pub fn base_value(&self) -> u16 { 1 }

    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
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
            (Self::Custom0(s) | Self::Custom1(s)
            | Self::Custom2(s) | Self::Custom3(s), _) => s.as_str(),
        }
    }
}

// --- サバイバル 専門分野 (Survival Specialization) --- p.63
#[derive(Clone)]
pub enum SurvivalSpec {
    Arctic,
    Desert,
    Sea,
    Custom0(String), Custom1(String), Custom2(String), Custom3(String),
}

impl crate::character::SpecLabel for SurvivalSpec {
    fn spec_label(&self, lang: Lang) -> &str { self.label(lang) }
}

impl SurvivalSpec {
    pub fn list() -> &'static [Self] {
        &[Self::Arctic, Self::Desert, Self::Sea]
    }

    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::Arctic    => 0,
            Self::Desert    => 1,
            Self::Sea       => 2,
            Self::Custom0(_) => 3,
            Self::Custom1(_) => 4,
            Self::Custom2(_) => 5,
            Self::Custom3(_) => 6,
        }
    }

    pub fn base_value(&self) -> u16 { 10 }

    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Self::Arctic,   Lang::Ja) => "極地",
            (Self::Arctic,   Lang::En) => "Arctic",
            (Self::Desert,   Lang::Ja) => "砂漠",
            (Self::Desert,   Lang::En) => "Desert",
            (Self::Sea,      Lang::Ja) => "海",
            (Self::Sea,      Lang::En) => "Sea",
            (Self::Custom0(s) | Self::Custom1(s)
            | Self::Custom2(s) | Self::Custom3(s), _) => s.as_str(),
        }
    }
}

// ============================================================
// --- 装備 (Equipment) ---
// ============================================================

pub enum Equipment {
    // todo: 装備・武器・所持品の定義
    Custom(String),
}

impl Equipment {
    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::Custom(_) => 0,
        }
    }
    pub fn label(&self, _lang: Lang) -> &str { "" }
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
    TreasuredPossessions,
    Trait,
    PhobiasAndManias,
    ArcaneTomesAndSpells,
    EncountersWithStrangeEntities,
}

impl Backstory {
    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::KeyConnection(_)              => 0,
            Self::PersonalDescription           => 1,
            Self::IdeologyAndBeliefs            => 2,
            Self::SignificantPeople             => 3,
            Self::MeaningfulLocation            => 4,
            Self::TreasuredPossessions          => 5,
            Self::Trait                         => 6,
            Self::PhobiasAndManias              => 7,
            Self::ArcaneTomesAndSpells          => 8,
            Self::EncountersWithStrangeEntities => 9,
        }
    }

    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::KeyConnection(_),              Lang::En) => "Key Connection",
            (Self::KeyConnection(_),              Lang::Ja) => "キーコネクション",
            (Self::PersonalDescription,           Lang::En) => "Personal Description",
            (Self::PersonalDescription,           Lang::Ja) => "個人的な記述",
            (Self::IdeologyAndBeliefs,            Lang::En) => "Ideology & Beliefs",
            (Self::IdeologyAndBeliefs,            Lang::Ja) => "イデオロギーと信念",
            (Self::SignificantPeople,             Lang::En) => "Significant People",
            (Self::SignificantPeople,             Lang::Ja) => "重要な人物",
            (Self::MeaningfulLocation,            Lang::En) => "Meaningful Location",
            (Self::MeaningfulLocation,            Lang::Ja) => "思い出の場所",
            (Self::TreasuredPossessions,          Lang::En) => "Treasured Possessions",
            (Self::TreasuredPossessions,          Lang::Ja) => "大切な持ち物",
            (Self::Trait,                         Lang::En) => "Trait",
            (Self::Trait,                         Lang::Ja) => "特徴・癖",
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
// --- 導出値・判定カテゴリ (Derived) ---
// ============================================================

pub enum Derived {
    HitPoints,
    MagicPoints,
    Build,
    DamageBonus,
    MoveRate,
    Sanity,
    OccupationSkillPoints,
    InterestSkillPoints,
}

impl Derived {
    pub fn id(&self, base: usize) -> usize {
        base + match self {
            Self::HitPoints             => 0,
            Self::MagicPoints           => 1,
            Self::Build                 => 2,
            Self::DamageBonus           => 3,
            Self::MoveRate              => 4,
            Self::Sanity                => 5,
            Self::OccupationSkillPoints => 6,
            Self::InterestSkillPoints   => 7,
        }
    }

    pub fn label(&self, lang: Lang) -> &str {
        match (self, lang){
            (Self::HitPoints,                    _) => "HP",
            (Self::MagicPoints,                  _) => "MP",
            (Self::Build,                 Lang::En) => "Build",
            (Self::Build,                 Lang::Ja) => "ビルド",
            (Self::DamageBonus,           Lang::En) => "Damage Bonus",
            (Self::DamageBonus,           Lang::Ja) => "ダメージボーナス",
            (Self::MoveRate,              Lang::En) => "Move Rate",
            (Self::MoveRate,              Lang::Ja) => "移動率 (MOV)",
            (Self::Sanity,                Lang::En) => "Sanity",
            (Self::Sanity,                Lang::Ja) => "正気度",
            (Self::OccupationSkillPoints, Lang::En) => "Occupation Skill Points",
            (Self::OccupationSkillPoints, Lang::Ja) => "職業技能ポイント",
            (Self::InterestSkillPoints,   Lang::En) => "Interest Skill Points",
            (Self::InterestSkillPoints,   Lang::Ja) => "興味技能ポイント",
        }
    }
    pub fn compute(&self, data_struct: &crate::data_struct::DataStruct) -> Result<Vec<u8>, crate::list::ListError> {
        match self {
            Self::HitPoints => {
                let constitution = data_struct.get(&Character::Characteristic(Characteristic::Constitution))?;
                let size         = data_struct.get(&Character::Characteristic(Characteristic::Size))?;
                let val = (constitution.iter().map(|&b| b as u16).sum::<u16>() + size.iter().map(|&b| b as u16).sum::<u16>()) / 10;
                Ok(val.to_le_bytes().to_vec())
            }
            Self::MagicPoints => {
                let power = data_struct.get(&Character::Characteristic(Characteristic::Power))?;
                let val = power.iter().map(|&b| b as u16).sum::<u16>() / 5;
                Ok(val.to_le_bytes().to_vec())
            }
            Self::Build => {
                // todo: Build計算
                Ok(vec![0])
            }
            Self::DamageBonus => {
                // todo: DamageBonus計算
                Ok(vec![0])
            }
            Self::MoveRate => {
                // todo: 移動率計算 (STR/DEX/SIZ比較)
                Ok(vec![0])
            }
            Self::Sanity => {
                let power = data_struct.get(&Character::Characteristic(Characteristic::Power))?;
                Ok(power.to_vec())
            }
            _ => Ok(vec![]),
        }
    }
    pub fn update(&self) {
        // todo: Derivedを一括再計算する
    }
}

// --- ビルド (Build) ---
enum BuildRank { // STR + SIZ の合計値から決定される離散段階。DamageBonus と 1対1 対応する。
    Neg2, // -2  (STR+SIZ:   2- 64)
    Neg1, // -1  (STR+SIZ:  65- 84)
    Zero, //  0  (STR+SIZ:  85-124)
    Pos1, // +1  (STR+SIZ: 125-164)
    Pos2, // +2  (STR+SIZ: 165-204)
    Pos3, // +3  (STR+SIZ: 205-284)
    Pos4, // +4  (STR+SIZ: 285-364)
    Pos5, // +5  (STR+SIZ: 365+   )
}

impl BuildRank {
    pub fn value(&self) -> i8 {
        match self {
            Self::Neg2 => -2,
            Self::Neg1 => -1,
            Self::Zero =>  0,
            Self::Pos1 =>  1,
            Self::Pos2 =>  2,
            Self::Pos3 =>  3,
            Self::Pos4 =>  4,
            Self::Pos5 =>  5,
        }
    }
}

enum DamageBonusDice {
    Neg2,   // -2   (Build -2)
    Neg1,   // -1   (Build -1)
    Zero,   // 0    (Build  0)
    Pos1D4, // +1D4 (Build +1)
    Pos1D6, // +1D6 (Build +2)
    Pos2D6, // +2D6 (Build +3)
    Pos3D6, // +3D6 (Build +4)
    Pos4D6, // +4D6 (Build +5)
}

impl DamageBonusDice {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Neg2  => "-2",
            Self::Neg1  => "-1",
            Self::Zero  => "0",
            Self::Pos1D4 => "+1D4",
            Self::Pos1D6 => "+1D6",
            Self::Pos2D6 => "+2D6",
            Self::Pos3D6 => "+3D6",
            Self::Pos4D6 => "+4D6",
        }
    }
}

// --- 生活水準 (Standard of Living) ---
pub enum StandardOfLiving {
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

    // Move Rate への減算値
    pub fn move_rate_penalty(&self) -> u8 {
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