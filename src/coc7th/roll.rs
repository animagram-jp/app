use core::{
    mem::{size_of, transmute_copy, variant_count},
    primitive::{i8, u8},
};

use crate::{Language, coc7th::character::dice};

/// # Safety
/// `T` must be a `#[repr(u8)]` enum whose discriminants are exactly `1..=N`.
const unsafe fn all_variants<T: Copy, const N: usize>(first: T) -> [T; N] {
    assert!(size_of::<T>() == 1 && N == variant_count::<T>());
    let mut all = [first; N];
    let mut i = 0;
    while i < N {
        // SAFETY: the caller guarantees that `T` is a `#[repr(u8)]` enum with discriminants `1..=N`.
        all[i] = unsafe { transmute_copy::<u8, T>(&(i as u8 + 1)) };
        i += 1;
    }
    all
}

fn pick<T: Copy>(list: &[T]) -> T {
    use rand::{RngExt as _, rand_core::UnwrapErr, rngs::SysRng};

    let mut rng = UnwrapErr(SysRng);
    list[rng.random_range(0..list.len())]
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Roll {
    DiceRoll = 1,
    SkillRoll,
    CharacteristicRoll,
    SanityRoll,
    BoutOfMadness,
    PushedRoll,
    CombinedSkillRoll,
    PhobiaAndMania,
    FailedCasting,
    DevelopmentCheck,
}

impl Roll {
    pub fn list() -> &'static [Self] {
        // SAFETY: `Roll` is `#[repr(u8)]` with discriminants `1..=variant_count`.
        const ALL: [Roll; variant_count::<Roll>()] = unsafe { all_variants(Roll::DiceRoll) };
        &ALL
    }
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::DiceRoll, Language::En(_)) => "Dice Roll (nDn +-n)",
            (Self::DiceRoll, Language::Ja) => "ダイスロール (nDn +-n)",
            (Self::SkillRoll, Language::En(_)) => "Skill Roll",
            (Self::SkillRoll, Language::Ja) => "技能ロール",
            (Self::CharacteristicRoll, Language::En(_)) => "Characteristic Roll",
            (Self::CharacteristicRoll, Language::Ja) => "能力値ロール",
            (Self::SanityRoll, Language::En(_)) => "Sanity Roll",
            (Self::SanityRoll, Language::Ja) => "正気度ロール",
            (Self::BoutOfMadness, Language::En(_)) => "Bout of Madness",
            (Self::BoutOfMadness, Language::Ja) => "狂気の発作",
            (Self::PushedRoll, Language::En(_)) => "Pushed Roll",
            (Self::PushedRoll, Language::Ja) => "プッシュロール",
            (Self::CombinedSkillRoll, Language::En(_)) => "Combined Skill Roll",
            (Self::CombinedSkillRoll, Language::Ja) => "組み合わせ技能ロール",
            (Self::PhobiaAndMania, Language::En(_)) => "Phobia and Mania",
            (Self::PhobiaAndMania, Language::Ja) => "恐怖症とマニア",
            (Self::FailedCasting, Language::En(_)) => "Failed Casting",
            (Self::FailedCasting, Language::Ja) => "呪文失敗",
            (Self::DevelopmentCheck, Language::En(_)) => "Development Check",
            (Self::DevelopmentCheck, Language::Ja) => "上達チェック",
        }
    }
}
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum BoutOfMadness {
    RealTime = 1,
    Summary,
}
impl BoutOfMadness {
    pub fn list() -> &'static [Self] {
        const ALL: [BoutOfMadness; variant_count::<BoutOfMadness>()] =
            // SAFETY: `BoutOfMadness` is `#[repr(u8)]` with discriminants `1..=variant_count`.
            unsafe { all_variants(BoutOfMadness::RealTime) };
        &ALL
    }
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::RealTime, Language::En(_)) => "real time",
            (Self::RealTime, Language::Ja) => "リアルタイム",
            (Self::Summary, Language::En(_)) => "summary",
            (Self::Summary, Language::Ja) => "サマリー",
        }
    }
}
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PhobiaAndMania {
    Phobia = 1,
    Mania,
}
impl PhobiaAndMania {
    pub fn list() -> &'static [Self] {
        const ALL: [PhobiaAndMania; variant_count::<PhobiaAndMania>()] =
            // SAFETY: `PhobiaAndMania` is `#[repr(u8)]` with discriminants `1..=variant_count`.
            unsafe { all_variants(PhobiaAndMania::Phobia) };
        &ALL
    }
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Phobia, Language::En(_)) => "Phobia",
            (Self::Phobia, Language::Ja) => "恐怖症",
            (Self::Mania, Language::En(_)) => "Mania",
            (Self::Mania, Language::Ja) => "マニア",
        }
    }
}
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum FailedCasting {
    Minor = 1,
    Major,
}
impl FailedCasting {
    pub fn list() -> &'static [Self] {
        const ALL: [FailedCasting; variant_count::<FailedCasting>()] =
            // SAFETY: `FailedCasting` is `#[repr(u8)]` with discriminants `1..=variant_count`.
            unsafe { all_variants(FailedCasting::Minor) };
        &ALL
    }
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Minor, Language::En(_)) => "minor",
            (Self::Minor, Language::Ja) => "小",
            (Self::Major, Language::En(_)) => "major",
            (Self::Major, Language::Ja) => "大",
        }
    }
}

pub enum FailureOrSuccess {
    Failure,
    Success,
}
impl FailureOrSuccess {
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Failure, Language::En(_)) => "Failure",
            (Self::Failure, Language::Ja) => "失敗",
            (Self::Success, Language::En(_)) => "Success",
            (Self::Success, Language::Ja) => "成功",
        }
    }
}
pub enum SaneOrInsane {
    Sane,
    Insane,
}
impl SaneOrInsane {
    pub fn get(target: u8, rolled: u8) -> Self {
        if target < rolled { Self::Sane } else { Self::Insane }
    }
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Sane, Language::En(_)) => "stay sane",
            (Self::Sane, Language::Ja) => "発狂しない",
            (Self::Insane, Language::En(_)) => "go insane",
            (Self::Insane, Language::Ja) => "発狂",
        }
    }
}
pub enum DevelopedOrUndeveloped {
    Developed,
    Undeveloped,
}
impl DevelopedOrUndeveloped {
    pub fn get(target: u8, rolled: u8) -> Self {
        if rolled > 96 || target < rolled { Self::Developed } else { Self::Undeveloped }
    }
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Developed, Language::En(_)) => "developed",
            (Self::Developed, Language::Ja) => "上達",
            (Self::Undeveloped, Language::En(_)) => "undeveloped",
            (Self::Undeveloped, Language::Ja) => "上達しない",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum SuccessLevel {
    Regular,
    Hard,
    Extreme,
}
impl SuccessLevel {
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Regular, Language::En(_)) => "regular success",
            (Self::Regular, Language::Ja) => "レギュラー成功",
            (Self::Hard, Language::En(_)) => "hard success",
            (Self::Hard, Language::Ja) => "ハード成功",
            (Self::Extreme, Language::En(_)) => "extreme success",
            (Self::Extreme, Language::Ja) => "イクストリーム成功",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Difficulty {
    Success(SuccessLevel),
    Critical,
    Fumble,
    Failure,
}
impl Difficulty {
    pub fn get(target: u8, rolled: u8) -> Self {
        if rolled == 1 {
            Self::Critical
        } else if rolled <= target / 5 {
            Self::Success(SuccessLevel::Extreme)
        } else if rolled <= target / 2 {
            Self::Success(SuccessLevel::Hard)
        } else if rolled <= target {
            Self::Success(SuccessLevel::Regular)
        } else if rolled >= if target < 50 { 96 } else { 100 } {
            Self::Fumble
        } else {
            Self::Failure
        }
    }
    pub fn meets(self, required: SuccessLevel) -> bool {
        match self {
            Self::Critical => true,
            Self::Success(level) => level >= required,
            _ => false,
        }
    }
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Fumble, Language::En(_)) => "fumble",
            (Self::Fumble, Language::Ja) => "致命的失敗",
            (Self::Failure, Language::En(_)) => "failure",
            (Self::Failure, Language::Ja) => "失敗",
            (Self::Critical, Language::En(_)) => "critical success",
            (Self::Critical, Language::Ja) => "クリティカル成功",
            (Self::Success(level), l) => level.display(l),
        }
    }
}

pub fn roll_skill(target: u8, level: i8) -> (u8, Difficulty) {
    let rolled = dice::percent_roll(level);
    (rolled, Difficulty::get(target, rolled))
}

pub fn roll_combined(target: (u8, u8), level: i8) -> (u8, (Difficulty, Difficulty)) {
    let rolled = dice::percent_roll(level);
    (rolled, (Difficulty::get(target.0, rolled), Difficulty::get(target.1, rolled)))
}

/// https://cthulhuwiki.chaosium.com/rules/sanity.html#bouts-of-madness-table
/// Ja p.153
#[derive(Clone, Copy)]
pub enum MadnessRealTime {
    Amnesia,
    PsychosomaticDisability,
    Violence,
    Paranoia,
    SignificantPerson,
    Faint,
    FleeInPanic,
    PhysicalHysterics,
    Phobia,
    Mania,
}

impl MadnessRealTime {
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Amnesia, Language::En(_)) => "Amnesia",
            (Self::Amnesia, Language::Ja) => "健忘症",
            (Self::PsychosomaticDisability, Language::En(_)) => "Psychosomatic Disability",
            (Self::PsychosomaticDisability, Language::Ja) => "身体症状症",
            (Self::Violence, Language::En(_)) => "Violence",
            (Self::Violence, Language::Ja) => "暴力衝動",
            (Self::Paranoia, Language::En(_)) => "Paranoia",
            (Self::Paranoia, Language::Ja) => "偏執症",
            (Self::SignificantPerson, Language::En(_)) => "Significant Person",
            (Self::SignificantPerson, Language::Ja) => "重要な人々",
            (Self::Faint, Language::En(_)) => "Faint",
            (Self::Faint, Language::Ja) => "失神",
            (Self::FleeInPanic, Language::En(_)) => "Flee in Panic",
            (Self::FleeInPanic, Language::Ja) => "パニックになって逃亡する",
            (Self::PhysicalHysterics, Language::En(_)) => {
                "Physical Hysterics or Emotional Outburst"
            }
            (Self::PhysicalHysterics, Language::Ja) => "身体的ヒステリーもしくは感情爆発",
            (Self::Phobia, Language::En(_)) => "Phobia",
            (Self::Phobia, Language::Ja) => "恐怖症",
            (Self::Mania, Language::En(_)) => "Mania",
            (Self::Mania, Language::Ja) => "マニア",
        }
    }
}

#[derive(Clone, Copy)]
pub enum MadnessSummary {
    Amnesia,
    Robbed,
    Battered,
    Violence,
    IdeologyBeliefs,
    SignificantPeople,
    Institutionalized,
    FleeInPanic,
    Phobia,
    Mania,
}

impl MadnessSummary {
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Amnesia, Language::En(_)) => "Amnesia",
            (Self::Amnesia, Language::Ja) => "健忘症",
            (Self::Robbed, Language::En(_)) => "Robbed",
            (Self::Robbed, Language::Ja) => "盗難",
            (Self::Battered, Language::En(_)) => "Battered",
            (Self::Battered, Language::Ja) => "暴行",
            (Self::Violence, Language::En(_)) => "Violence",
            (Self::Violence, Language::Ja) => "暴力",
            (Self::IdeologyBeliefs, Language::En(_)) => "Ideology/Beliefs",
            (Self::IdeologyBeliefs, Language::Ja) => "イデオロギー／信念",
            (Self::SignificantPeople, Language::En(_)) => "Significant People",
            (Self::SignificantPeople, Language::Ja) => "重要な人々",
            (Self::Institutionalized, Language::En(_)) => "Institutionalized",
            (Self::Institutionalized, Language::Ja) => "収容",
            (Self::FleeInPanic, Language::En(_)) => "Flee in Panic",
            (Self::FleeInPanic, Language::Ja) => "パニック",
            (Self::Phobia, Language::En(_)) => "Phobia",
            (Self::Phobia, Language::Ja) => "恐怖症",
            (Self::Mania, Language::En(_)) => "Mania",
            (Self::Mania, Language::Ja) => "マニア",
        }
    }
}

#[derive(Clone, Copy)]
pub enum FailedCastingMinor {
    BlurredVision,
    Screaming,
    StrongWind,
    Bleeding,
    StrangeVisions,
    SmallAnimalsExplode,
    StenchOfSulphur,
    MythosCreatureSummoned,
}

impl FailedCastingMinor {
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::BlurredVision, Language::En(_)) => "Blurred vision or temporary blindness",
            (Self::BlurredVision, Language::Ja) => "視界のかすみ、または一時的な失明",
            (Self::Screaming, Language::En(_)) => "Screaming, voices, or other noises",
            (Self::Screaming, Language::Ja) => "悲鳴、声、雑音が発せられる",
            (Self::StrongWind, Language::En(_)) => "Strong winds or other atmospheric effects",
            (Self::StrongWind, Language::Ja) => "強風などの大気現象",
            (Self::Bleeding, Language::En(_)) => {
                "Bleeding from the caster, bystanders, or environment"
            }
            (Self::Bleeding, Language::Ja) => {
                "術者かその場に居合わせた者、あるいは壁などからの出血"
            }
            (Self::StrangeVisions, Language::En(_)) => "Strange visions and hallucinations",
            (Self::StrangeVisions, Language::Ja) => "奇妙な幻視と幻覚",
            (Self::SmallAnimalsExplode, Language::En(_)) => "Nearby small animals explode",
            (Self::SmallAnimalsExplode, Language::Ja) => "付近の小動物たちが爆発する",
            (Self::StenchOfSulphur, Language::En(_)) => "Foul stench of sulphur",
            (Self::StenchOfSulphur, Language::Ja) => "硫黄の悪臭",
            (Self::MythosCreatureSummoned, Language::En(_)) => {
                "A Mythos creature is accidentally summoned"
            }
            (Self::MythosCreatureSummoned, Language::Ja) => "クトゥルフ神話の怪物が偶然召喚される",
        }
    }
}

#[derive(Clone, Copy)]
pub enum FailedCastingMajor {
    Earthquake,
    EpicLightning,
    BloodRain,
    CasterHandsWither,
    CasterAgesUnaturally,
    MythosCreatureAttacks,
    SweptAwayInTime,
    MythosDeityInvoked,
}

impl FailedCastingMajor {
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Earthquake, Language::En(_)) => "The earth shakes and walls crack and crumble",
            (Self::Earthquake, Language::Ja) => "大地が震え、壁に亀裂が入って崩れる",
            (Self::EpicLightning, Language::En(_)) => "Epic lightning strike",
            (Self::EpicLightning, Language::Ja) => "叙事詩的な電撃",
            (Self::BloodRain, Language::En(_)) => "Blood rains from the sky",
            (Self::BloodRain, Language::Ja) => "血が空から降る",
            (Self::CasterHandsWither, Language::En(_)) => {
                "The caster's hands wither and are seared"
            }
            (Self::CasterHandsWither, Language::Ja) => "術者の手がしなび、焼けただれる",
            (Self::CasterAgesUnaturally, Language::En(_)) => "The caster ages unnaturally",
            (Self::CasterAgesUnaturally, Language::Ja) => "術者は不自然に年をとる",
            (Self::MythosCreatureAttacks, Language::En(_)) => {
                "A Mythos creature appears and attacks those nearby"
            }
            (Self::MythosCreatureAttacks, Language::Ja) => {
                "クトゥルフ神話存在が現れ、術者や周囲をに被害を与える"
            }
            (Self::SweptAwayInTime, Language::En(_)) => {
                "The caster and all nearby are swept away to a distant time or place"
            }
            (Self::SweptAwayInTime, Language::Ja) => {
                "術者や近くの全員が遠い時代か場所に吸い込まれる"
            }
            (Self::MythosDeityInvoked, Language::En(_)) => "A Mythos deity is accidentally invoked",
            (Self::MythosDeityInvoked, Language::Ja) => "クトゥルフ神話の神格が偶然招来される",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Phobia {
    Ablutophobia = 1,
    Acrophobia,
    Aerophobia,
    Agoraphobia,
    Alektorophobia,
    Alliumphobia,
    Amaxophobia,
    Ancraophobia,
    Androphobia,
    Anglophobia,
    Anthophobia,
    Apotemnophobia,
    Arachnophobia,
    Astraphobia,
    Atephobia,
    Aulophobia,
    Bacteriophobia,
    Ballistophobia,
    Basophobia,
    Bibliophobia,
    Botanophobia,
    Caligynephobia,
    Cheimaphobia,
    Chronomentrophobia,
    Claustrophobia,
    Coulrophobia,
    Cynophobia,
    Demonophobia,
    Demophobia,
    Dentophobia,
    Disposophobia,
    Doraphobia,
    Dromophobia,
    Ecclesiophobia,
    Eisoptrophobia,
    Enetophobia,
    Entomophobia,
    Felinophobia,
    Gephyrophobia,
    Gerontophobia,
    Gynophobia,
    Haemaphobia,
    Hamartophobia,
    Haphophobia,
    Herpetophobia,
    Homichlophobia,
    Hoplophobia,
    Hydrophobia,
    Hypnophobia,
    Iatrophobia,
    Ichthyophobia,
    Katsaridaphobia,
    Keraunophobia,
    Lachanophobia,
    Ligyrophobia,
    Limnophobia,
    Mechanophobia,
    Megalophobia,
    Merinthophobia,
    Meteorophobia,
    Monophobia,
    Mysophobia,
    Myxophobia,
    Necrophobia,
    Octophobia,
    Odontophobia,
    Oneirophobia,
    Onomatophobia,
    Ophidiophobia,
    Ornithophobia,
    Parasitophobia,
    Pediophobia,
    Phagophobia,
    Pharmacophobia,
    Phasmophobia,
    Phenogophobia,
    Pogonophobia,
    Potamophobia,
    Potophobia,
    Pyrophobia,
    Rhabdophobia,
    Scotophobia,
    Selenophobia,
    Siderodromophobia,
    Siderophobia,
    Stenophobia,
    Symmetrophobia,
    Taphephobia,
    Taurophobia,
    Telephonophobia,
    Teratophobia,
    Thalassophobia,
    Tomophobia,
    Triskadekaphobia,
    Vestiphobia,
    Wiccaphobia,
    Xanthophobia,
    Xenoglossophobia,
    Xenophobia,
    Zoophobia,
}

impl Phobia {
    pub fn list() -> &'static [Self] {
        const ALL: [Phobia; variant_count::<Phobia>()] =
            // SAFETY: `Phobia` is `#[repr(u8)]` with discriminants `1..=variant_count`.
            unsafe { all_variants(Phobia::Ablutophobia) };
        &ALL
    }
    pub fn id(self) -> u8 {
        self as u8
    }
    pub fn roll() -> Self {
        pick(Self::list())
    }
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Ablutophobia, Language::En(_)) => "Abluto",
            (Self::Ablutophobia, Language::Ja) => "入浴",
            (Self::Acrophobia, Language::En(_)) => "Acro",
            (Self::Acrophobia, Language::Ja) => "高所",
            (Self::Aerophobia, Language::En(_)) => "Aero",
            (Self::Aerophobia, Language::Ja) => "飛行",
            (Self::Agoraphobia, Language::En(_)) => "Agora",
            (Self::Agoraphobia, Language::Ja) => "広場",
            (Self::Alektorophobia, Language::En(_)) => "Alektoro",
            (Self::Alektorophobia, Language::Ja) => "鶏肉",
            (Self::Alliumphobia, Language::En(_)) => "Allium",
            (Self::Alliumphobia, Language::Ja) => "ニンニク",
            (Self::Amaxophobia, Language::En(_)) => "Amaxo",
            (Self::Amaxophobia, Language::Ja) => "乗車",
            (Self::Ancraophobia, Language::En(_)) => "Ancrao",
            (Self::Ancraophobia, Language::Ja) => "風",
            (Self::Androphobia, Language::En(_)) => "Andro",
            (Self::Androphobia, Language::Ja) => "男性",
            (Self::Anglophobia, Language::En(_)) => "Anglo",
            (Self::Anglophobia, Language::Ja) => "イングランド",
            (Self::Anthophobia, Language::En(_)) => "Antho",
            (Self::Anthophobia, Language::Ja) => "花",
            (Self::Apotemnophobia, Language::En(_)) => "Apotemno",
            (Self::Apotemnophobia, Language::Ja) => "切断",
            (Self::Arachnophobia, Language::En(_)) => "Arachno",
            (Self::Arachnophobia, Language::Ja) => "クモ",
            (Self::Astraphobia, Language::En(_)) => "Astra",
            (Self::Astraphobia, Language::Ja) => "稲妻",
            (Self::Atephobia, Language::En(_)) => "Ate",
            (Self::Atephobia, Language::Ja) => "廃墟",
            (Self::Aulophobia, Language::En(_)) => "Aulo",
            (Self::Aulophobia, Language::Ja) => "笛",
            (Self::Bacteriophobia, Language::En(_)) => "Bacterio",
            (Self::Bacteriophobia, Language::Ja) => "細菌",
            (Self::Ballistophobia, Language::En(_)) => "Ballisto",
            (Self::Ballistophobia, Language::Ja) => "銃弾",
            (Self::Basophobia, Language::En(_)) => "Baso",
            (Self::Basophobia, Language::Ja) => "落下",
            (Self::Bibliophobia, Language::En(_)) => "Biblio",
            (Self::Bibliophobia, Language::Ja) => "書物",
            (Self::Botanophobia, Language::En(_)) => "Botano",
            (Self::Botanophobia, Language::Ja) => "植物",
            (Self::Caligynephobia, Language::En(_)) => "Caligyne",
            (Self::Caligynephobia, Language::Ja) => "美女",
            (Self::Cheimaphobia, Language::En(_)) => "Cheima",
            (Self::Cheimaphobia, Language::Ja) => "低温",
            (Self::Chronomentrophobia, Language::En(_)) => "Chronomentro",
            (Self::Chronomentrophobia, Language::Ja) => "時計",
            (Self::Claustrophobia, Language::En(_)) => "Claustr",
            (Self::Claustrophobia, Language::Ja) => "閉所",
            (Self::Coulrophobia, Language::En(_)) => "Coulro",
            (Self::Coulrophobia, Language::Ja) => "道化師",
            (Self::Cynophobia, Language::En(_)) => "Cyno",
            (Self::Cynophobia, Language::Ja) => "犬",
            (Self::Demonophobia, Language::En(_)) => "Demono",
            (Self::Demonophobia, Language::Ja) => "悪魔",
            (Self::Demophobia, Language::En(_)) => "Demo",
            (Self::Demophobia, Language::Ja) => "群集",
            (Self::Dentophobia, Language::En(_)) => "Dento",
            (Self::Dentophobia, Language::Ja) => "歯科医",
            (Self::Disposophobia, Language::En(_)) => "Disposo",
            (Self::Disposophobia, Language::Ja) => "処分",
            (Self::Doraphobia, Language::En(_)) => "Dora",
            (Self::Doraphobia, Language::Ja) => "毛皮",
            (Self::Dromophobia, Language::En(_)) => "Dromo",
            (Self::Dromophobia, Language::Ja) => "構断",
            (Self::Ecclesiophobia, Language::En(_)) => "Ecclesio",
            (Self::Ecclesiophobia, Language::Ja) => "教会",
            (Self::Eisoptrophobia, Language::En(_)) => "Eisoptro",
            (Self::Eisoptrophobia, Language::Ja) => "鏡",
            (Self::Enetophobia, Language::En(_)) => "Eneto",
            (Self::Enetophobia, Language::Ja) => "ピン",
            (Self::Entomophobia, Language::En(_)) => "Entomo",
            (Self::Entomophobia, Language::Ja) => "昆虫",
            (Self::Felinophobia, Language::En(_)) => "Felino",
            (Self::Felinophobia, Language::Ja) => "猫",
            (Self::Gephyrophobia, Language::En(_)) => "Gephyro",
            (Self::Gephyrophobia, Language::Ja) => "橋",
            (Self::Gerontophobia, Language::En(_)) => "Geronto",
            (Self::Gerontophobia, Language::Ja) => "老人",
            (Self::Gynophobia, Language::En(_)) => "Gyno",
            (Self::Gynophobia, Language::Ja) => "女性",
            (Self::Haemaphobia, Language::En(_)) => "Haema",
            (Self::Haemaphobia, Language::Ja) => "血液",
            (Self::Hamartophobia, Language::En(_)) => "Hamarto",
            (Self::Hamartophobia, Language::Ja) => "過失",
            (Self::Haphophobia, Language::En(_)) => "Hapho",
            (Self::Haphophobia, Language::Ja) => "接触",
            (Self::Herpetophobia, Language::En(_)) => "Herpeto",
            (Self::Herpetophobia, Language::Ja) => "爬虫類",
            (Self::Homichlophobia, Language::En(_)) => "Homichlo",
            (Self::Homichlophobia, Language::Ja) => "霧",
            (Self::Hoplophobia, Language::En(_)) => "Hoplo",
            (Self::Hoplophobia, Language::Ja) => "銃器",
            (Self::Hydrophobia, Language::En(_)) => "Hydro",
            (Self::Hydrophobia, Language::Ja) => "水",
            (Self::Hypnophobia, Language::En(_)) => "Hypno",
            (Self::Hypnophobia, Language::Ja) => "睡眠",
            (Self::Iatrophobia, Language::En(_)) => "Iatro",
            (Self::Iatrophobia, Language::Ja) => "医師",
            (Self::Ichthyophobia, Language::En(_)) => "Ichthyo",
            (Self::Ichthyophobia, Language::Ja) => "魚",
            (Self::Katsaridaphobia, Language::En(_)) => "Katsarida",
            (Self::Katsaridaphobia, Language::Ja) => "ゴキブリ",
            (Self::Keraunophobia, Language::En(_)) => "Kerauno",
            (Self::Keraunophobia, Language::Ja) => "雷鳴",
            (Self::Lachanophobia, Language::En(_)) => "Lachano",
            (Self::Lachanophobia, Language::Ja) => "野菜",
            (Self::Ligyrophobia, Language::En(_)) => "Ligyro",
            (Self::Ligyrophobia, Language::Ja) => "大騒音",
            (Self::Limnophobia, Language::En(_)) => "Limno",
            (Self::Limnophobia, Language::Ja) => "湖",
            (Self::Mechanophobia, Language::En(_)) => "Mechano",
            (Self::Mechanophobia, Language::Ja) => "機械",
            (Self::Megalophobia, Language::En(_)) => "Megalo",
            (Self::Megalophobia, Language::Ja) => "巨大物",
            (Self::Merinthophobia, Language::En(_)) => "Merintho",
            (Self::Merinthophobia, Language::Ja) => "拘束",
            (Self::Meteorophobia, Language::En(_)) => "Meteoro",
            (Self::Meteorophobia, Language::Ja) => "隕石",
            (Self::Monophobia, Language::En(_)) => "Mono",
            (Self::Monophobia, Language::Ja) => "孤独",
            (Self::Mysophobia, Language::En(_)) => "Myso",
            (Self::Mysophobia, Language::Ja) => "汚染",
            (Self::Myxophobia, Language::En(_)) => "Myxo",
            (Self::Myxophobia, Language::Ja) => "粘液",
            (Self::Necrophobia, Language::En(_)) => "Necro",
            (Self::Necrophobia, Language::Ja) => "死体",
            (Self::Octophobia, Language::En(_)) => "Octo",
            (Self::Octophobia, Language::Ja) => "8",
            (Self::Odontophobia, Language::En(_)) => "Odonto",
            (Self::Odontophobia, Language::Ja) => "歯",
            (Self::Oneirophobia, Language::En(_)) => "Oneiro",
            (Self::Oneirophobia, Language::Ja) => "夢",
            (Self::Onomatophobia, Language::En(_)) => "Onomato",
            (Self::Onomatophobia, Language::Ja) => "名称",
            (Self::Ophidiophobia, Language::En(_)) => "Ophidio",
            (Self::Ophidiophobia, Language::Ja) => "蛇",
            (Self::Ornithophobia, Language::En(_)) => "Ornitho",
            (Self::Ornithophobia, Language::Ja) => "鳥",
            (Self::Parasitophobia, Language::En(_)) => "Parasito",
            (Self::Parasitophobia, Language::Ja) => "寄生生物",
            (Self::Pediophobia, Language::En(_)) => "Pedio",
            (Self::Pediophobia, Language::Ja) => "人形",
            (Self::Phagophobia, Language::En(_)) => "Phago",
            (Self::Phagophobia, Language::Ja) => "恐食症",
            (Self::Pharmacophobia, Language::En(_)) => "Pharmaco",
            (Self::Pharmacophobia, Language::Ja) => "薬物",
            (Self::Phasmophobia, Language::En(_)) => "Phasmo",
            (Self::Phasmophobia, Language::Ja) => "幽霊",
            (Self::Phenogophobia, Language::En(_)) => "Phenogo",
            (Self::Phenogophobia, Language::Ja) => "羞明",
            (Self::Pogonophobia, Language::En(_)) => "Pogono",
            (Self::Pogonophobia, Language::Ja) => "ひげ",
            (Self::Potamophobia, Language::En(_)) => "Potamo",
            (Self::Potamophobia, Language::Ja) => "河川",
            (Self::Potophobia, Language::En(_)) => "Poto",
            (Self::Potophobia, Language::Ja) => "アルコール",
            (Self::Pyrophobia, Language::En(_)) => "Pyro",
            (Self::Pyrophobia, Language::Ja) => "火",
            (Self::Rhabdophobia, Language::En(_)) => "Rhabdo",
            (Self::Rhabdophobia, Language::Ja) => "魔術",
            (Self::Scotophobia, Language::En(_)) => "Scoto",
            (Self::Scotophobia, Language::Ja) => "暗黒",
            (Self::Selenophobia, Language::En(_)) => "Seleno",
            (Self::Selenophobia, Language::Ja) => "月",
            (Self::Siderodromophobia, Language::En(_)) => "Siderodromo",
            (Self::Siderodromophobia, Language::Ja) => "鉄道",
            (Self::Siderophobia, Language::En(_)) => "Sidero",
            (Self::Siderophobia, Language::Ja) => "星",
            (Self::Stenophobia, Language::En(_)) => "Steno",
            (Self::Stenophobia, Language::Ja) => "狭所",
            (Self::Symmetrophobia, Language::En(_)) => "Symmetro",
            (Self::Symmetrophobia, Language::Ja) => "対称",
            (Self::Taphephobia, Language::En(_)) => "Taphe",
            (Self::Taphephobia, Language::Ja) => "生き埋め",
            (Self::Taurophobia, Language::En(_)) => "Tauro",
            (Self::Taurophobia, Language::Ja) => "雄牛",
            (Self::Telephonophobia, Language::En(_)) => "Telephono",
            (Self::Telephonophobia, Language::Ja) => "電話",
            (Self::Teratophobia, Language::En(_)) => "Terato",
            (Self::Teratophobia, Language::Ja) => "奇形",
            (Self::Thalassophobia, Language::En(_)) => "Thalasso",
            (Self::Thalassophobia, Language::Ja) => "海洋",
            (Self::Tomophobia, Language::En(_)) => "Tomo",
            (Self::Tomophobia, Language::Ja) => "手術",
            (Self::Triskadekaphobia, Language::En(_)) => "Triskadeka",
            (Self::Triskadekaphobia, Language::Ja) => "13",
            (Self::Vestiphobia, Language::En(_)) => "Vesti",
            (Self::Vestiphobia, Language::Ja) => "衣類",
            (Self::Wiccaphobia, Language::En(_)) => "Wicca",
            (Self::Wiccaphobia, Language::Ja) => "魔女",
            (Self::Xanthophobia, Language::En(_)) => "Xantho",
            (Self::Xanthophobia, Language::Ja) => "黄色",
            (Self::Xenoglossophobia, Language::En(_)) => "Xenoglosso",
            (Self::Xenoglossophobia, Language::Ja) => "外国語",
            (Self::Xenophobia, Language::En(_)) => "Xeno",
            (Self::Xenophobia, Language::Ja) => "外国人",
            (Self::Zoophobia, Language::En(_)) => "Zoo",
            (Self::Zoophobia, Language::Ja) => "動物",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Mania {
    Ablutomania = 1,
    Aboulomania,
    Achluomania,
    Acromaniaheights,
    Agathomania,
    Agromania,
    Aichmomania,
    Ailuromania,
    Algomania,
    Alliomania,
    Amaxomania,
    Amenomania,
    Anthomania,
    Arithmomania,
    Asoticamania,
    Eremiomania,
    Balletmania,
    Biliokleptomania,
    Bibliomania,
    Bruxomania,
    Cacodemomania,
    Callomania,
    Cartacoethes,
    Catapedamania,
    Cheimatomania,
    Choreomania,
    Clinomania,
    Coimetormania,
    Coloromania,
    Coulromania,
    Countermania,
    Dacnomania,
    Demonomania,
    Dermatillomania,
    Dikemania,
    Dipsomania,
    Doramania,
    Doromania,
    Drapetomania,
    Ecdemiomania,
    Egomania,
    Empleomania,
    Enosimania,
    Epistemomania,
    Eremiomaniaquiet,
    Etheromania,
    Gamomania,
    Geliomania,
    Goetomania,
    Graphomania,
    Gymnomania,
    Habromania,
    Helminthomania,
    Hoplomania,
    Hydromania,
    Ichthyomania,
    Iconomania,
    Idolomania,
    Infomania,
    Klazomania,
    Kleptomania,
    Ligyromania,
    Linonomania,
    Lotterymania,
    Lypemania,
    Megalithomania,
    Melomania,
    Metromania,
    Misomania,
    Monomania,
    Mythomania,
    Nosomania,
    Notomania,
    Onomamania,
    Onomatomania,
    Onychotillomania,
    Opsomania,
    Paramania,
    Personamania,
    Phasmomania,
    Phonomania,
    Photomania,
    Antinomiamania,
    Plutomania,
    Pseudomania,
    Pyromania,
    QuestionAsking,
    Rhinotillexomania,
    Scribbleomania,
    Siderodromomania,
    Sophomania,
    Technomania,
    Thanatomania,
    Theomania,
    Titillomaniac,
    Tomomania,
    Trichotillomania,
    Typhlomania,
    Xenomania,
    Zoomania,
}

impl Mania {
    pub fn list() -> &'static [Self] {
        // SAFETY: `Mania` is `#[repr(u8)]` with discriminants `1..=variant_count`.
        const ALL: [Mania; variant_count::<Mania>()] = unsafe { all_variants(Mania::Ablutomania) };
        &ALL
    }
    pub fn id(self) -> u8 {
        self as u8
    }
    pub fn roll() -> Self {
        pick(Self::list())
    }
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Ablutomania, Language::En(_)) => "Abluto",
            (Self::Ablutomania, Language::Ja) => "洗浄",
            (Self::Aboulomania, Language::En(_)) => "Aboulo",
            (Self::Aboulomania, Language::Ja) => "無為",
            (Self::Achluomania, Language::En(_)) => "Achluo",
            (Self::Achluomania, Language::Ja) => "暗闇",
            (Self::Acromaniaheights, Language::En(_)) => "Acro",
            (Self::Acromaniaheights, Language::Ja) => "高所",
            (Self::Agathomania, Language::En(_)) => "Agatho",
            (Self::Agathomania, Language::Ja) => "善良",
            (Self::Agromania, Language::En(_)) => "Agro",
            (Self::Agromania, Language::Ja) => "広場",
            (Self::Aichmomania, Language::En(_)) => "Aichmo",
            (Self::Aichmomania, Language::Ja) => "先鋭",
            (Self::Ailuromania, Language::En(_)) => "Ailuro",
            (Self::Ailuromania, Language::Ja) => "猫",
            (Self::Algomania, Language::En(_)) => "Algo",
            (Self::Algomania, Language::Ja) => "疼痛性愛",
            (Self::Alliomania, Language::En(_)) => "Allio",
            (Self::Alliomania, Language::Ja) => "にんにく",
            (Self::Amaxomania, Language::En(_)) => "Amaxo",
            (Self::Amaxomania, Language::Ja) => "乗り物",
            (Self::Amenomania, Language::En(_)) => "Ameno",
            (Self::Amenomania, Language::Ja) => "病的快活",
            (Self::Anthomania, Language::En(_)) => "Antho",
            (Self::Anthomania, Language::Ja) => "花",
            (Self::Arithmomania, Language::En(_)) => "Arithmo",
            (Self::Arithmomania, Language::Ja) => "計算",
            (Self::Asoticamania, Language::En(_)) => "Asotica",
            (Self::Asoticamania, Language::Ja) => "浪費",
            (Self::Eremiomania, Language::En(_)) => "Eremio",
            (Self::Eremiomania, Language::Ja) => "自己",
            (Self::Balletmania, Language::En(_)) => "Ballet",
            (Self::Balletmania, Language::Ja) => "バレエ",
            (Self::Biliokleptomania, Language::En(_)) => "Bilioklepto",
            (Self::Biliokleptomania, Language::Ja) => "書籍約盗癖",
            (Self::Bibliomania, Language::En(_)) => "Biblio",
            (Self::Bibliomania, Language::Ja) => "書物",
            (Self::Bruxomania, Language::En(_)) => "Bruxo",
            (Self::Bruxomania, Language::Ja) => "歯ぎしり",
            (Self::Cacodemomania, Language::En(_)) => "Cacodemo",
            (Self::Cacodemomania, Language::Ja) => "悪霊",
            (Self::Callomania, Language::En(_)) => "Callo",
            (Self::Callomania, Language::Ja) => "自己愛",
            (Self::Cartacoethes, Language::En(_)) => "Cartacoethes",
            (Self::Cartacoethes, Language::Ja) => "地図",
            (Self::Catapedamania, Language::En(_)) => "Catapeda",
            (Self::Catapedamania, Language::Ja) => "飛び降り",
            (Self::Cheimatomania, Language::En(_)) => "Cheimato",
            (Self::Cheimatomania, Language::Ja) => "寒冷",
            (Self::Choreomania, Language::En(_)) => "Choreo",
            (Self::Choreomania, Language::Ja) => "舞踏",
            (Self::Clinomania, Language::En(_)) => "Clino",
            (Self::Clinomania, Language::Ja) => "睡眠",
            (Self::Coimetormania, Language::En(_)) => "Coimetor",
            (Self::Coimetormania, Language::Ja) => "墓地",
            (Self::Coloromania, Language::En(_)) => "Coloro",
            (Self::Coloromania, Language::Ja) => "色彩",
            (Self::Coulromania, Language::En(_)) => "Coulro",
            (Self::Coulromania, Language::Ja) => "ピエロ",
            (Self::Countermania, Language::En(_)) => "Counter",
            (Self::Countermania, Language::Ja) => "遭遇",
            (Self::Dacnomania, Language::En(_)) => "Dacno",
            (Self::Dacnomania, Language::Ja) => "殺害",
            (Self::Demonomania, Language::En(_)) => "Demono",
            (Self::Demonomania, Language::Ja) => "悪魔",
            (Self::Dermatillomania, Language::En(_)) => "Dermatillo",
            (Self::Dermatillomania, Language::Ja) => "皮膚",
            (Self::Dikemania, Language::En(_)) => "Dike",
            (Self::Dikemania, Language::Ja) => "正義",
            (Self::Dipsomania, Language::En(_)) => "Dipso",
            (Self::Dipsomania, Language::Ja) => "アルコール",
            (Self::Doramania, Language::En(_)) => "Dora",
            (Self::Doramania, Language::Ja) => "毛皮",
            (Self::Doromania, Language::En(_)) => "Doro",
            (Self::Doromania, Language::Ja) => "贈り物",
            (Self::Drapetomania, Language::En(_)) => "Drapeto",
            (Self::Drapetomania, Language::Ja) => "逃走",
            (Self::Ecdemiomania, Language::En(_)) => "Ecdemio",
            (Self::Ecdemiomania, Language::Ja) => "外出",
            (Self::Egomania, Language::En(_)) => "Ego",
            (Self::Egomania, Language::Ja) => "自己中心",
            (Self::Empleomania, Language::En(_)) => "Empleo",
            (Self::Empleomania, Language::Ja) => "公職",
            (Self::Enosimania, Language::En(_)) => "Enosi",
            (Self::Enosimania, Language::Ja) => "戦慄",
            (Self::Epistemomania, Language::En(_)) => "Epistemo",
            (Self::Epistemomania, Language::Ja) => "知識",
            (Self::Eremiomaniaquiet, Language::En(_)) => "Eremio (quiet)",
            (Self::Eremiomaniaquiet, Language::Ja) => "静寂",
            (Self::Etheromania, Language::En(_)) => "Ethero",
            (Self::Etheromania, Language::Ja) => "エーテル",
            (Self::Gamomania, Language::En(_)) => "Gamo",
            (Self::Gamomania, Language::Ja) => "求婚",
            (Self::Geliomania, Language::En(_)) => "Gelio",
            (Self::Geliomania, Language::Ja) => "笑い",
            (Self::Goetomania, Language::En(_)) => "Goeto",
            (Self::Goetomania, Language::Ja) => "魔術",
            (Self::Graphomania, Language::En(_)) => "Grapho",
            (Self::Graphomania, Language::Ja) => "筆記",
            (Self::Gymnomania, Language::En(_)) => "Gymno",
            (Self::Gymnomania, Language::Ja) => "裸体",
            (Self::Habromania, Language::En(_)) => "Habro",
            (Self::Habromania, Language::Ja) => "幻想",
            (Self::Helminthomania, Language::En(_)) => "Helmintho",
            (Self::Helminthomania, Language::Ja) => "蟲",
            (Self::Hoplomania, Language::En(_)) => "Hoplo",
            (Self::Hoplomania, Language::Ja) => "火器",
            (Self::Hydromania, Language::En(_)) => "Hydro",
            (Self::Hydromania, Language::Ja) => "水",
            (Self::Ichthyomania, Language::En(_)) => "Ichthyo",
            (Self::Ichthyomania, Language::Ja) => "魚",
            (Self::Iconomania, Language::En(_)) => "Icono",
            (Self::Iconomania, Language::Ja) => "アイコン",
            (Self::Idolomania, Language::En(_)) => "Idolo",
            (Self::Idolomania, Language::Ja) => "アイドル",
            (Self::Infomania, Language::En(_)) => "Info",
            (Self::Infomania, Language::Ja) => "情報",
            (Self::Klazomania, Language::En(_)) => "Klazo",
            (Self::Klazomania, Language::Ja) => "絶叫",
            (Self::Kleptomania, Language::En(_)) => "Klepto",
            (Self::Kleptomania, Language::Ja) => "窃盗",
            (Self::Ligyromania, Language::En(_)) => "Ligyro",
            (Self::Ligyromania, Language::Ja) => "騒音",
            (Self::Linonomania, Language::En(_)) => "Linono",
            (Self::Linonomania, Language::Ja) => "ひも",
            (Self::Lotterymania, Language::En(_)) => "Lottery",
            (Self::Lotterymania, Language::Ja) => "宝くじ",
            (Self::Lypemania, Language::En(_)) => "Lype",
            (Self::Lypemania, Language::Ja) => "うつ",
            (Self::Megalithomania, Language::En(_)) => "Megalitho",
            (Self::Megalithomania, Language::Ja) => "巨石",
            (Self::Melomania, Language::En(_)) => "Melo",
            (Self::Melomania, Language::Ja) => "音楽",
            (Self::Metromania, Language::En(_)) => "Metro",
            (Self::Metromania, Language::Ja) => "作詩",
            (Self::Misomania, Language::En(_)) => "Miso",
            (Self::Misomania, Language::Ja) => "憎悪",
            (Self::Monomania, Language::En(_)) => "Mono",
            (Self::Monomania, Language::Ja) => "偏執",
            (Self::Mythomania, Language::En(_)) => "Mytho",
            (Self::Mythomania, Language::Ja) => "虚言",
            (Self::Nosomania, Language::En(_)) => "Noso",
            (Self::Nosomania, Language::Ja) => "疾病",
            (Self::Notomania, Language::En(_)) => "Noto",
            (Self::Notomania, Language::Ja) => "記録",
            (Self::Onomamania, Language::En(_)) => "Onoma",
            (Self::Onomamania, Language::Ja) => "名前",
            (Self::Onomatomania, Language::En(_)) => "Onomato",
            (Self::Onomatomania, Language::Ja) => "単語",
            (Self::Onychotillomania, Language::En(_)) => "Onychotillo",
            (Self::Onychotillomania, Language::Ja) => "爪損傷",
            (Self::Opsomania, Language::En(_)) => "Opso",
            (Self::Opsomania, Language::Ja) => "美食",
            (Self::Paramania, Language::En(_)) => "Para",
            (Self::Paramania, Language::Ja) => "不平",
            (Self::Personamania, Language::En(_)) => "Persona",
            (Self::Personamania, Language::Ja) => "仮面",
            (Self::Phasmomania, Language::En(_)) => "Phasmo",
            (Self::Phasmomania, Language::Ja) => "幽霊",
            (Self::Phonomania, Language::En(_)) => "Phono",
            (Self::Phonomania, Language::Ja) => "殺人",
            (Self::Photomania, Language::En(_)) => "Photo",
            (Self::Photomania, Language::Ja) => "光線",
            (Self::Antinomiamania, Language::En(_)) => "Antinomia",
            (Self::Antinomiamania, Language::Ja) => "放浪",
            (Self::Plutomania, Language::En(_)) => "Pluto",
            (Self::Plutomania, Language::Ja) => "長者",
            (Self::Pseudomania, Language::En(_)) => "Pseudo",
            (Self::Pseudomania, Language::Ja) => "病的虚言",
            (Self::Pyromania, Language::En(_)) => "Pyro",
            (Self::Pyromania, Language::Ja) => "放火",
            (Self::QuestionAsking, Language::En(_)) => "Question-Asking",
            (Self::QuestionAsking, Language::Ja) => "質問",
            (Self::Rhinotillexomania, Language::En(_)) => "Rhinotillex",
            (Self::Rhinotillexomania, Language::Ja) => "鼻",
            (Self::Scribbleomania, Language::En(_)) => "Scribbleo",
            (Self::Scribbleomania, Language::Ja) => "落書き",
            (Self::Siderodromomania, Language::En(_)) => "Siderodromo",
            (Self::Siderodromomania, Language::Ja) => "列車",
            (Self::Sophomania, Language::En(_)) => "Sopho",
            (Self::Sophomania, Language::Ja) => "知性",
            (Self::Technomania, Language::En(_)) => "Techno",
            (Self::Technomania, Language::Ja) => "テクノ",
            (Self::Thanatomania, Language::En(_)) => "Thanato",
            (Self::Thanatomania, Language::Ja) => "タナトス",
            (Self::Theomania, Language::En(_)) => "Theo",
            (Self::Theomania, Language::Ja) => "宗教",
            (Self::Titillomaniac, Language::En(_)) => "Titillomaniac",
            (Self::Titillomaniac, Language::Ja) => "かき傷",
            (Self::Tomomania, Language::En(_)) => "Tomo",
            (Self::Tomomania, Language::Ja) => "手術",
            (Self::Trichotillomania, Language::En(_)) => "Trichotillo",
            (Self::Trichotillomania, Language::Ja) => "抜毛",
            (Self::Typhlomania, Language::En(_)) => "Typhlo",
            (Self::Typhlomania, Language::Ja) => "失明",
            (Self::Xenomania, Language::En(_)) => "Xeno",
            (Self::Xenomania, Language::Ja) => "異国",
            (Self::Zoomania, Language::En(_)) => "Zoo",
            (Self::Zoomania, Language::Ja) => "動物",
        }
    }
}

#[cfg(test)]
mod variant_tests {
    use alloc::{collections::BTreeSet, vec::Vec};
    use core::mem::{transmute_copy, variant_count};

    use super::*;
    use crate::En;

    fn discriminants<T: Copy>(list: &[T]) -> Vec<u8> {
        // SAFETY: every list passed here is built by `all_variants`, so `T` is a one-byte `#[repr(u8)]` enum.
        list.iter().map(|v| unsafe { transmute_copy::<T, u8>(v) }).collect()
    }

    fn assert_list<T: Copy>(list: &[T], expected: usize) {
        assert_eq!(list.len(), expected);
        assert_eq!(list.len(), variant_count::<T>());
        let expected_discriminants: Vec<u8> = (1..=expected as u8).collect();
        assert_eq!(discriminants(list), expected_discriminants);
    }

    #[test]
    fn list_covers_every_variant_in_declaration_order() {
        assert_list(Roll::list(), 10);
        assert_list(BoutOfMadness::list(), 2);
        assert_list(PhobiaAndMania::list(), 2);
        assert_list(FailedCasting::list(), 2);
        assert_list(Phobia::list(), 100);
        assert_list(Mania::list(), 100);
    }

    #[test]
    fn id_is_one_based_position() {
        for (i, v) in Phobia::list().iter().enumerate() {
            assert_eq!(v.id() as usize, i + 1);
        }
        for (i, v) in Mania::list().iter().enumerate() {
            assert_eq!(v.id() as usize, i + 1);
        }
    }

    #[test]
    fn roll_returns_list_members_and_varies() {
        let mut phobia_ids = BTreeSet::new();
        let mut mania_ids = BTreeSet::new();
        for _ in 0..1000 {
            let p = Phobia::roll();
            let m = Mania::roll();
            assert!((1..=100).contains(&p.id()));
            assert!((1..=100).contains(&m.id()));
            phobia_ids.insert(p.id());
            mania_ids.insert(m.id());
        }
        assert!(phobia_ids.len() >= 50, "phobia variety {}", phobia_ids.len());
        assert!(mania_ids.len() >= 50, "mania variety {}", mania_ids.len());
    }

    #[test]
    fn every_phobia_and_mania_has_names_in_both_languages() {
        for language in [Language::Ja, Language::En(En::Gb)] {
            for v in Phobia::list() {
                assert!(!v.display(language).is_empty());
            }
            for v in Mania::list() {
                assert!(!v.display(language).is_empty());
            }
        }
    }

    #[test]
    fn phobia_and_mania_names_are_distinct() {
        let phobias: BTreeSet<&str> =
            Phobia::list().iter().map(|v| v.display(Language::Ja)).collect();
        let manias: BTreeSet<&str> =
            Mania::list().iter().map(|v| v.display(Language::Ja)).collect();
        assert_eq!(phobias.len(), 100);
        assert_eq!(manias.len(), 100);
    }
}
