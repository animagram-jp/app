use rand::{rng, RngExt};
use crate::Lang;
use crate::character::{self, Skill, Characteristic};

// ============================================================
// Percent Roll (1d100 + Bonus/Penalty Dice)
// ============================================================

pub fn percent_roll(bonus: i32) -> (u32, Vec<u32>) {
    let mut rng = rng();
    let roll_tens = |r: &mut _| {
        let d: u32 = RngExt::random_range(r, 1..=10u32);
        if d == 10 { 0 } else { d * 10 }
    };
    let ones: u32 = {
        let d: u32 = rng.random_range(1..=10u32);
        if d == 10 { 0 } else { d }
    };
    let count = (bonus.unsigned_abs() + 1) as usize;
    let tens_list: Vec<u32> = (0..count).map(|_| roll_tens(&mut rng)).collect();
    let dice_list: Vec<u32> = tens_list
        .iter()
        .map(|&t| { let v = t + ones; if v == 0 { 100 } else { v } })
        .collect();
    let total = if bonus >= 0 {
        *dice_list.iter().min().unwrap()
    } else {
        *dice_list.iter().max().unwrap()
    };
    (total, dice_list)
}

// ============================================================
// todo: 未定義型のスタブ（各型が確定次第、対応モジュールに移動する）
// ============================================================

pub type Count = u16;
pub type Side  = u16;

pub type Dice = character::Dice;
pub struct DiceModifier(pub i16);
pub struct Skills<T>(pub Vec<T>);
pub struct Characteristics<T>(pub Vec<T>);
pub struct SkillModifier(pub i32);

pub enum SkillOrCharacteristic {
    Skill(crate::character::Skill),
    Characteristic(crate::character::Characteristic),
}

pub enum SuccessLevel { Regular, Hard, Extreme, Critical }

pub struct SkillRoll;
pub struct SkillRollResult;

#[derive(Debug)]
pub enum SkillRollError { BonusDiceOutOfRange }

pub enum Difficulty { None, Hard, Extreme, Critical }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResultLevel { Regular, Hard, Extreme, Critical, Fumble, Failure }
impl ResultLevel {
    pub fn from_values(total: u32, skill: u32) -> Self {
        if total == 1               { Self::Critical }
        else if total <= skill / 5  { Self::Extreme }
        else if total <= skill / 2  { Self::Hard }
        else if total <= skill      { Self::Regular }
        else if total >= if skill < 50 { 96 } else { 100 } { Self::Fumble }
        else                        { Self::Failure }
    }
    pub fn with_difficulty_level(total: u32, skill: u32) -> Self {
        Self::from_values(total, skill)
    }
}

pub enum DiceRollSelect {}
pub enum Level { Regular, Hard, Extreme }

pub enum BulletSetCap { Auto, Specified(u32) }

// ============================================================
// ロール (Roll)
// ============================================================

pub enum Roll {
    /// 任意ダイス式ロール。dice_terms は character::Dice = (count, sides, modifier) のリスト。
    DiceRoll(Vec<Dice>),
    SkillRoll(Skills<Skill>, Option<SuccessLevel>, Option<i16>), // option(i16)とは補正値(+-i)のこと
    CharacteristicRoll(Characteristics<Characteristic>, Option<SuccessLevel>, SkillModifier),
    SanityRoll(),
    BoutOfMadness(BoutScene),
    PushedRoll(SkillOrCharacteristic, Option<SuccessLevel>, Option<i16>), // todo: pushでも新規技能はありうるので、履歴利用はソートサジェストだけにする
    CombinedSkillRoll(SkillOrCharacteristic, SkillOrCharacteristic),
    PhobiaAndMania(Impulse),
    // todo: 射撃時の連射判定, 射撃時のボーナス・ペナルティダイスのセレクタガイド
    // AutoFireRoll, ロジックが煩雑・そこまで使わないので一時コメントアウト
    FailedCasting(FailureDepth),
    DevelopmentCheck,
}

impl Roll {
    pub fn label(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::DiceRoll(_),            Lang::En) => "Dice Roll (nDn +-n)",
            (Self::DiceRoll(_),            Lang::Ja) => "ダイスロール (nDn +-n)",
            (Self::SkillRoll(..),          Lang::En) => "Skill Roll",
            (Self::SkillRoll(..),          Lang::Ja) => "技能ロール",
            (Self::CharacteristicRoll(..), Lang::En) => "Characteristic Roll",
            (Self::CharacteristicRoll(..), Lang::Ja) => "能力値ロール",
            (Self::SanityRoll(),           Lang::En) => "Sanity Roll",
            (Self::SanityRoll(),           Lang::Ja) => "正気度ロール",
            (Self::BoutOfMadness(_),       Lang::En) => "Bout of Madness",
            (Self::BoutOfMadness(_),       Lang::Ja) => "狂気の発作",
            (Self::PushedRoll(..),         Lang::En) => "Pushed Roll",
            (Self::PushedRoll(..),         Lang::Ja) => "プッシュロール",
            (Self::CombinedSkillRoll(..),  Lang::En) => "Combined Skill Roll",
            (Self::CombinedSkillRoll(..),  Lang::Ja) => "組み合わせ技能ロール",
            (Self::PhobiaAndMania(_),      Lang::En) => "Phobia and Mania",
            (Self::PhobiaAndMania(_),      Lang::Ja) => "恐怖症とマニア",
            // (Self::AutoFireRoll,       Lang::Ja) => "自動火器の連射判定",
            // (Self::AutoFireRoll,       Lang::En) => "Automatic Fire Roll",
            (Self::FailedCasting(_),       Lang::En) => "Failed Casting",
            (Self::FailedCasting(_),       Lang::Ja) => "呪文失敗",
            (Self::DevelopmentCheck,       Lang::En) => "Development Check",
            (Self::DevelopmentCheck,       Lang::Ja) => "上達チェック",
        }
    }

    // NOTE: BoutOfMadness, FailedCasting はバリアント引数を持つため静的スライスに含められない
    // pub fn all() -> &'static [Roll] { ... }
}

pub enum BoutScene { RealTime, Summary }
impl BoutScene {
    pub fn label(self, lang: Lang) -> &'static str {
        match(self, lang) {
            (Self::RealTime, Lang::En) => "real time",
            (Self::RealTime, Lang::Ja) => "リアルタイム",
            (Self::Summary,  Lang::En) => "summary",
            (Self::Summary,  Lang::Ja) => "サマリー",                                          
        }
    }
}

pub enum Impulse { Phobia, Mania }
impl Impulse {
    pub fn label(self, lang: Lang) -> &'static str {
        match(self, lang) {
            (Self::Phobia, Lang::En) => "Phobia",
            (Self::Phobia, Lang::Ja) => "恐怖症",
            (Self::Mania,  Lang::En) => "Mania",
            (Self::Mania,  Lang::Ja) => "マニア",                                          
        }
    }
}

pub enum FailureDepth { Minor, Major }
impl FailureDepth {
    pub fn label(self, lang: Lang) -> &'static str {
        match(self, lang) {
            (Self::Minor, Lang::En) => "minor",
            (Self::Minor, Lang::Ja) => "小",
            (Self::Major,  Lang::En) => "major",
            (Self::Major,  Lang::Ja) => "大",                                          
        }
    }
}

// ============================================================
// ロール結果 (RollResult, RollError, RollJudge)
// ============================================================

// --- 狂気の発作表 結果 (Bout of Madness Result)---
pub struct BoutOfMadnessResult {
    scene: BoutScene,
    total: u8,    // n_d_n(1, 10)
    duration: u8, // n_d_n(1, 10) 持続時間
}

// --- ロール結果 (Roll Result) ---
pub struct RollResult {
    roll_total: Vec<i16>,
    roll_judge: Option<Vec<RollJudge>>,
}

impl RollResult {
    pub fn display(&self) {
        // "[{}: {}] {} {}"
        todo!()
    }
}

// --- ロールエラー (Roll Error) ---
#[derive(Debug)]
pub enum RollError {
    BonusDiceOutOfRange,
}

// --- ロールジャッジ (Roll Judge) ---
pub enum RollJudge {
    Fumble,
    Failure,
    Success,
    Regular,
    Hard,
    Extreme,
    Critical,
    Sane,
    Insane,
    Developed,
    Undeveloped,
}

impl RollJudge {
    pub fn judge(total: u32, target: u32, difficulty: Option<&Self>) -> Self {
        let fumble_at = if target < 50 { 96 } else { 100 };
        if total == 1                                         { Self::Critical }
        else if total >= fumble_at                            { Self::Fumble }
        else if difficulty.is_some() && total <= target       { Self::Success }
        else if total <= target / 5                           { Self::Extreme }
        else if total <= target / 2                           { Self::Hard }
        else if total <= target                               { Self::Regular }
        else                                                  { Self::Failure }
    }
    pub fn label(self, lang: Lang) -> &'static str {
        match(self, lang) {
            (Self::Fumble,      Lang::En) => "fumble",
            (Self::Fumble,      Lang::Ja) => "致命的失敗",
            (Self::Failure,     Lang::En) => "failure",
            (Self::Failure,     Lang::Ja) => "失敗",
            (Self::Success,     Lang::En) => "Success",
            (Self::Success,     Lang::Ja) => "成功",
            (Self::Regular,     Lang::En) => "regular success",
            (Self::Regular,     Lang::Ja) => "レギュラー成功",
            (Self::Hard,        Lang::En) => "hard success",
            (Self::Hard,        Lang::Ja) => "ハード成功",
            (Self::Extreme,     Lang::En) => "extreme success",
            (Self::Extreme,     Lang::Ja) => "イクストリーム成功",  
            (Self::Critical,    Lang::En) => "critical success",
            (Self::Critical,    Lang::Ja) => "クリティカル成功",  
            (Self::Sane,        Lang::En) => "stay sane",
            (Self::Sane,        Lang::Ja) => "発狂しない",  
            (Self::Insane,      Lang::En) => "go insane",
            (Self::Insane,      Lang::Ja) => "発狂",  
            (Self::Developed,   Lang::En) => "developed",
            (Self::Developed,   Lang::Ja) => "上達",
            (Self::Undeveloped, Lang::En) => "undeveloped",
            (Self::Undeveloped, Lang::Ja) => "上達しない",                                            
        }
    }
}

// ============================================================
// 技能ロール (Skill Roll)
// ============================================================

impl SkillRoll {
    /// - bonus_dice の絶対値が 100 超 → Err
    /// - difficulty == None かつ bonus_dice == 0 → 出目のみ（level=None）
    /// - difficulty == Some(0) → None 扱い
    /// - Hard → target / 2、Extreme → target / 5、Critical → target = 1
    pub fn roll(
        target: u32,
        bonus_dice: i32,
        difficulty: Difficulty,
    ) -> Result<SkillRollResult, SkillRollError> {
        if bonus_dice.unsigned_abs() > 100 {
            return Err(SkillRollError::BonusDiceOutOfRange);
        }

        let effective_target: u32 = match difficulty {
            Difficulty::Hard     => target / 2,
            Difficulty::Extreme  => target / 5,
            Difficulty::Critical => 1,
            Difficulty::None     => target,
        };

        let (total, _dice_candidates) = percent_roll(bonus_dice);

        let _level = match difficulty {
            Difficulty::None => ResultLevel::from_values(total, effective_target),
            _                    => ResultLevel::with_difficulty_level(total, effective_target),
        };

        Ok(SkillRollResult)
    }
}

pub struct DiceRoll {
    roll_select:   Roll,
    select:        Vec<DiceRollSelect>,
    bonus_dice:    i32,
    level:         Level,
    target_select: (crate::character::Characteristic, crate::character::Skill),
    /// ダイス項目リスト。character::Dice = (count: i8, sides: u8, modifier: i8)
    dice_terms:    Vec<Dice>,
    result:        RollResult,
}

// ============================================================
// 組み合わせロール (Combined Skill Roll)
// ============================================================

// 1回の1d100を2技能値に対してそれぞれ判定する
pub fn combined_roll(target: (u32, u32)) -> RollResult {
    let (total, _) = percent_roll(0);
    let judges = vec![
        RollJudge::judge(total, target.0, None),
        RollJudge::judge(total, target.1, None),
    ];
    RollResult { roll_total: vec![total as i16], roll_judge: Some(judges) }
}

// ============================================================
// 自動火器射撃判定 (Full Auto Roll) — 未実装、放置中
// ============================================================

/*
#[derive(Debug)]
pub enum FullAutoWarning {
    BulletsClamped { original: u32 },
    BrokenNumberNegated,
    BulletSetCapClampedLow { clamped_to: u32 },
    BulletSetCapClampedHigh { clamped_to: u32, low_skill: bool },
}

#[derive(Debug)]
pub enum FullAutoError {
    NoBullets,
    NoSkill,
    BonusDiceOutOfRange,
    BulletSetCapNonPositive,
}

pub enum BulletSetCap {
    Auto,
    Specified(u32),
}

#[derive(Debug, Clone, Copy)]
pub enum ResultLevel { Regular, Hard, Extreme, Critical, Fumble, Failure }
impl ResultLevel {
    pub fn from_values(_total: u32, _skill: u32) -> Self { Self::Failure }
}

#[derive(Debug)]
pub struct VolleyResult {
    pub stage: u32,
    pub stage_changed: bool,
    pub loop_index: u32,
    pub total: u32,
    pub dice_candidates: Vec<u32>,
    pub level: ResultLevel,
    pub hit: u32,
    pub impale: u32,
    pub jammed: bool,
}

#[derive(Debug)]
pub struct FullAutoResult {
    pub warnings: Vec<FullAutoWarning>,
    pub bonus_dice: i32,
    pub volleys: Vec<VolleyResult>,
    pub hit_total: u32,
    pub impale_total: u32,
    pub remaining_bullets: u32,
    pub stopped_by_difficulty: bool,
    pub jammed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StopAt { None, Regular, Hard, Extreme }

/// - bullet_count > 100 → クランプ+warning
/// - bullet_count == 0 / skill == 0 → Err
/// - broken_number < 0 → 絶対値補正+warning
/// - bonus_dice 絶対値 > 2 → Err
/// - BulletSetCap::Specified(0) → Err
/// - BulletSetCap::Specified(1〜2) → 下限3にクランプ+warning
/// - skill <= 39 → BulletSetCap 上限は3固定
/// - skill >= 40 → BulletSetCap 上限は skill/10
/// - ジャム（total >= broken_number） → 即時終了
/// - 難易度段階: レギュラー→ハード→イクストリーム→クリティカル の4段階
pub fn full_auto(
    bullet_count: u32,
    skill: u32,
    broken_number: i32,
    bonus_dice: i32,
    stop_at: StopAt,
    bullet_set_cap: Option<BulletSetCap>,
) -> Result<FullAutoResult, FullAutoError> {
    todo!()
}

fn bullet_result(
    bullet_count: u32,
    level: ResultLevel,
    skill: u32,
    bullet_set: u32,
    is_last: bool,
    stage: u32,
) -> (u32, u32, u32) {
    let hit_base = if skill < 30 { 1 } else { bullet_set / 2 };
    let is_hit = match stage {
        0 => matches!(level, ResultLevel::Hard | ResultLevel::Regular),
        1 => matches!(level, ResultLevel::Hard),
        2 => false,
        _ => matches!(level, ResultLevel::Critical),
    };
    let is_impale = match stage {
        0..=2 => matches!(level, ResultLevel::Critical | ResultLevel::Extreme),
        _     => false,
    };
    if is_hit {
        if is_last { let h = (bullet_count + 1) / 2; (h, 0, bullet_count) }
        else       { (hit_base, 0, bullet_set) }
    } else if is_impale {
        if is_last { let i = bullet_count / 2; (bullet_count - i, i, bullet_count) }
        else       { let i = bullet_set / 2; (bullet_set - i, i, bullet_set) }
    } else {
        (0, 0, bullet_set.min(bullet_count))
    }
}
*/

// ============================================================
// ランダム表
// ============================================================

/// ルールブック 日本語訳版 153頁
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
    pub fn get(&self, _index: u8) -> Self {
        todo!()
    }
    pub fn index(&self) -> u8 {
        *self as u8 + 1
    }
    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Amnesia,                Lang::En) => "Amnesia",
            (Self::Amnesia,                Lang::Ja) => "健忘症",
            (Self::PsychosomaticDisability,Lang::En) => "Psychosomatic Disability",
            (Self::PsychosomaticDisability,Lang::Ja) => "身体症状症",
            (Self::Violence,               Lang::En) => "Violence",
            (Self::Violence,               Lang::Ja) => "暴力衝動",
            (Self::Paranoia,               Lang::En) => "Paranoia",
            (Self::Paranoia,               Lang::Ja) => "偏執症",
            (Self::SignificantPerson,      Lang::En) => "Significant Person",
            (Self::SignificantPerson,      Lang::Ja) => "重要な人々",
            (Self::Faint,                  Lang::En) => "Faint",
            (Self::Faint,                  Lang::Ja) => "失神",
            (Self::FleeInPanic,            Lang::En) => "Flee in Panic",
            (Self::FleeInPanic,            Lang::Ja) => "パニックになって逃亡する",
            (Self::PhysicalHysterics,      Lang::En) => "Physical Hysterics or Emotional Outburst",
            (Self::PhysicalHysterics,      Lang::Ja) => "身体的ヒステリーもしくは感情爆発",
            (Self::Phobia,                 Lang::En) => "Phobia",
            (Self::Phobia,                 Lang::Ja) => "恐怖症",
            (Self::Mania,                  Lang::En) => "Mania",
            (Self::Mania,                  Lang::Ja) => "マニア",
        }
    }
}

/// ルールブック 日本語訳版 155頁
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
    pub fn get(&self, _index: u8) -> Self {
        todo!()
    }
    pub fn index(&self) -> u8 {
        *self as u8 + 1
    }
    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Amnesia,          Lang::En) => "Amnesia",
            (Self::Amnesia,          Lang::Ja) => "健忘症",
            (Self::Robbed,           Lang::En) => "Robbed",
            (Self::Robbed,           Lang::Ja) => "盗難",
            (Self::Battered,         Lang::En) => "Battered",
            (Self::Battered,         Lang::Ja) => "暴行",
            (Self::Violence,         Lang::En) => "Violence",
            (Self::Violence,         Lang::Ja) => "暴力",
            (Self::IdeologyBeliefs,  Lang::En) => "Ideology/Beliefs",
            (Self::IdeologyBeliefs,  Lang::Ja) => "イデオロギー／信念",
            (Self::SignificantPeople,Lang::En) => "Significant People",
            (Self::SignificantPeople,Lang::Ja) => "重要な人々",
            (Self::Institutionalized,Lang::En) => "Institutionalized",
            (Self::Institutionalized,Lang::Ja) => "収容",
            (Self::FleeInPanic,      Lang::En) => "Flee in Panic",
            (Self::FleeInPanic,      Lang::Ja) => "パニック",
            (Self::Phobia,           Lang::En) => "Phobia",
            (Self::Phobia,           Lang::Ja) => "恐怖症",
            (Self::Mania,            Lang::En) => "Mania",
            (Self::Mania,            Lang::Ja) => "マニア",
        }
    }
}

/// ルールブック 日本語訳版 174頁
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
    pub fn get(&self, _index: u8) -> Self {
        todo!()
    }
    pub fn index(&self) -> u8 {
        *self as u8 + 1
    }
    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::BlurredVision,          Lang::En) => "Blurred vision or temporary blindness",
            (Self::BlurredVision,          Lang::Ja) => "視界のかすみ、または一時的な失明",
            (Self::Screaming,              Lang::En) => "Screaming, voices, or other noises",
            (Self::Screaming,              Lang::Ja) => "悲鳴、声、雑音が発せられる",
            (Self::StrongWind,             Lang::En) => "Strong winds or other atmospheric effects",
            (Self::StrongWind,             Lang::Ja) => "強風などの大気現象",
            (Self::Bleeding,               Lang::En) => "Bleeding from the caster, bystanders, or environment",
            (Self::Bleeding,               Lang::Ja) => "術者かその場に居合わせた者、あるいは壁などからの出血",
            (Self::StrangeVisions,         Lang::En) => "Strange visions and hallucinations",
            (Self::StrangeVisions,         Lang::Ja) => "奇妙な幻視と幻覚",
            (Self::SmallAnimalsExplode,    Lang::En) => "Nearby small animals explode",
            (Self::SmallAnimalsExplode,    Lang::Ja) => "付近の小動物たちが爆発する",
            (Self::StenchOfSulphur,        Lang::En) => "Foul stench of sulphur",
            (Self::StenchOfSulphur,        Lang::Ja) => "硫黄の悪臭",
            (Self::MythosCreatureSummoned, Lang::En) => "A Mythos creature is accidentally summoned",
            (Self::MythosCreatureSummoned, Lang::Ja) => "クトゥルフ神話の怪物が偶然召喚される",
        }
    }
}

/// ルールブック 日本語訳版 P175
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
    pub fn get(&self, _index: u8) -> Self {
        todo!()
    }
    pub fn index(&self) -> u8 {
        *self as u8 + 1
    }
    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Earthquake,           Lang::En) => "The earth shakes and walls crack and crumble",
            (Self::Earthquake,           Lang::Ja) => "大地が震え、壁に亀裂が入って崩れる",
            (Self::EpicLightning,        Lang::En) => "Epic lightning strike",
            (Self::EpicLightning,        Lang::Ja) => "叙事詩的な電撃",
            (Self::BloodRain,            Lang::En) => "Blood rains from the sky",
            (Self::BloodRain,            Lang::Ja) => "血が空から降る",
            (Self::CasterHandsWither,    Lang::En) => "The caster's hands wither and are seared",
            (Self::CasterHandsWither,    Lang::Ja) => "術者の手がしなび、焼けただれる",
            (Self::CasterAgesUnaturally, Lang::En) => "The caster ages unnaturally",
            (Self::CasterAgesUnaturally, Lang::Ja) => "術者は不自然に年をとる",
            (Self::MythosCreatureAttacks,Lang::En) => "A Mythos creature appears and attacks those nearby",
            (Self::MythosCreatureAttacks,Lang::Ja) => "クトゥルフ神話存在が現れ、術者や周囲をに被害を与える",
            (Self::SweptAwayInTime,      Lang::En) => "The caster and all nearby are swept away to a distant time or place",
            (Self::SweptAwayInTime,      Lang::Ja) => "術者や近くの全員が遠い時代か場所に吸い込まれる",
            (Self::MythosDeityInvoked,   Lang::En) => "A Mythos deity is accidentally invoked",
            (Self::MythosDeityInvoked,   Lang::Ja) => "クトゥルフ神話の神格が偶然招来される",
        }
    }
}

/// ルールブック 日本語訳版 156頁
#[derive(Clone, Copy)]
pub enum Phobia {
    Ablutophobia,
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
    pub fn get(&self, _index: u8) -> Self {
        todo!()
    }
    pub fn index(&self) -> u8 {
        *self as u8 + 1
    }
    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Ablutophobia,       Lang::En) => "Abluto",
            (Self::Ablutophobia,       Lang::Ja) => "入浴",
            (Self::Acrophobia,         Lang::En) => "Acro",
            (Self::Acrophobia,         Lang::Ja) => "高所",
            (Self::Aerophobia,         Lang::En) => "Aero",
            (Self::Aerophobia,         Lang::Ja) => "飛行",
            (Self::Agoraphobia,        Lang::En) => "Agora",
            (Self::Agoraphobia,        Lang::Ja) => "広場",
            (Self::Alektorophobia,     Lang::En) => "Alektoro",
            (Self::Alektorophobia,     Lang::Ja) => "鶏肉",
            (Self::Alliumphobia,       Lang::En) => "Allium",
            (Self::Alliumphobia,       Lang::Ja) => "ニンニク",
            (Self::Amaxophobia,        Lang::En) => "Amaxo",
            (Self::Amaxophobia,        Lang::Ja) => "乗車",
            (Self::Ancraophobia,       Lang::En) => "Ancrao",
            (Self::Ancraophobia,       Lang::Ja) => "風",
            (Self::Androphobia,        Lang::En) => "Andro",
            (Self::Androphobia,        Lang::Ja) => "男性",
            (Self::Anglophobia,        Lang::En) => "Anglo",
            (Self::Anglophobia,        Lang::Ja) => "イングランド",
            (Self::Anthophobia,        Lang::En) => "Antho",
            (Self::Anthophobia,        Lang::Ja) => "花",
            (Self::Apotemnophobia,     Lang::En) => "Apotemno",
            (Self::Apotemnophobia,     Lang::Ja) => "切断",
            (Self::Arachnophobia,      Lang::En) => "Arachno",
            (Self::Arachnophobia,      Lang::Ja) => "クモ",
            (Self::Astraphobia,        Lang::En) => "Astra",
            (Self::Astraphobia,        Lang::Ja) => "稲妻",
            (Self::Atephobia,          Lang::En) => "Ate",
            (Self::Atephobia,          Lang::Ja) => "廃墟",
            (Self::Aulophobia,         Lang::En) => "Aulo",
            (Self::Aulophobia,         Lang::Ja) => "笛",
            (Self::Bacteriophobia,     Lang::En) => "Bacterio",
            (Self::Bacteriophobia,     Lang::Ja) => "細菌",
            (Self::Ballistophobia,     Lang::En) => "Ballisto",
            (Self::Ballistophobia,     Lang::Ja) => "銃弾",
            (Self::Basophobia,         Lang::En) => "Baso",
            (Self::Basophobia,         Lang::Ja) => "落下",
            (Self::Bibliophobia,       Lang::En) => "Biblio",
            (Self::Bibliophobia,       Lang::Ja) => "書物",
            (Self::Botanophobia,       Lang::En) => "Botano",
            (Self::Botanophobia,       Lang::Ja) => "植物",
            (Self::Caligynephobia,     Lang::En) => "Caligyne",
            (Self::Caligynephobia,     Lang::Ja) => "美女",
            (Self::Cheimaphobia,       Lang::En) => "Cheima",
            (Self::Cheimaphobia,       Lang::Ja) => "低温",
            (Self::Chronomentrophobia, Lang::En) => "Chronomentro",
            (Self::Chronomentrophobia, Lang::Ja) => "時計",
            (Self::Claustrophobia,     Lang::En) => "Claustr",
            (Self::Claustrophobia,     Lang::Ja) => "閉所",
            (Self::Coulrophobia,       Lang::En) => "Coulro",
            (Self::Coulrophobia,       Lang::Ja) => "道化師",
            (Self::Cynophobia,         Lang::En) => "Cyno",
            (Self::Cynophobia,         Lang::Ja) => "犬",
            (Self::Demonophobia,       Lang::En) => "Demono",
            (Self::Demonophobia,       Lang::Ja) => "悪魔",
            (Self::Demophobia,         Lang::En) => "Demo",
            (Self::Demophobia,         Lang::Ja) => "群集",
            (Self::Dentophobia,        Lang::En) => "Dento",
            (Self::Dentophobia,        Lang::Ja) => "歯科医",
            (Self::Disposophobia,      Lang::En) => "Disposo",
            (Self::Disposophobia,      Lang::Ja) => "処分",
            (Self::Doraphobia,         Lang::En) => "Dora",
            (Self::Doraphobia,         Lang::Ja) => "毛皮",
            (Self::Dromophobia,        Lang::En) => "Dromo",
            (Self::Dromophobia,        Lang::Ja) => "構断",
            (Self::Ecclesiophobia,     Lang::En) => "Ecclesio",
            (Self::Ecclesiophobia,     Lang::Ja) => "教会",
            (Self::Eisoptrophobia,     Lang::En) => "Eisoptro",
            (Self::Eisoptrophobia,     Lang::Ja) => "鏡",
            (Self::Enetophobia,        Lang::En) => "Eneto",
            (Self::Enetophobia,        Lang::Ja) => "ピン",
            (Self::Entomophobia,       Lang::En) => "Entomo",
            (Self::Entomophobia,       Lang::Ja) => "昆虫",
            (Self::Felinophobia,       Lang::En) => "Felino",
            (Self::Felinophobia,       Lang::Ja) => "猫",
            (Self::Gephyrophobia,      Lang::En) => "Gephyro",
            (Self::Gephyrophobia,      Lang::Ja) => "橋",
            (Self::Gerontophobia,      Lang::En) => "Geronto",
            (Self::Gerontophobia,      Lang::Ja) => "老人",
            (Self::Gynophobia,         Lang::En) => "Gyno",
            (Self::Gynophobia,         Lang::Ja) => "女性",
            (Self::Haemaphobia,        Lang::En) => "Haema",
            (Self::Haemaphobia,        Lang::Ja) => "血液",
            (Self::Hamartophobia,      Lang::En) => "Hamarto",
            (Self::Hamartophobia,      Lang::Ja) => "過失",
            (Self::Haphophobia,        Lang::En) => "Hapho",
            (Self::Haphophobia,        Lang::Ja) => "接触",
            (Self::Herpetophobia,      Lang::En) => "Herpeto",
            (Self::Herpetophobia,      Lang::Ja) => "爬虫類",
            (Self::Homichlophobia,     Lang::En) => "Homichlo",
            (Self::Homichlophobia,     Lang::Ja) => "霧",
            (Self::Hoplophobia,        Lang::En) => "Hoplo",
            (Self::Hoplophobia,        Lang::Ja) => "銃器",
            (Self::Hydrophobia,        Lang::En) => "Hydro",
            (Self::Hydrophobia,        Lang::Ja) => "水",
            (Self::Hypnophobia,        Lang::En) => "Hypno",
            (Self::Hypnophobia,        Lang::Ja) => "睡眠",
            (Self::Iatrophobia,        Lang::En) => "Iatro",
            (Self::Iatrophobia,        Lang::Ja) => "医師",
            (Self::Ichthyophobia,      Lang::En) => "Ichthyo",
            (Self::Ichthyophobia,      Lang::Ja) => "魚",
            (Self::Katsaridaphobia,    Lang::En) => "Katsarida",
            (Self::Katsaridaphobia,    Lang::Ja) => "ゴキブリ",
            (Self::Keraunophobia,      Lang::En) => "Kerauno",
            (Self::Keraunophobia,      Lang::Ja) => "雷鳴",
            (Self::Lachanophobia,      Lang::En) => "Lachano",
            (Self::Lachanophobia,      Lang::Ja) => "野菜",
            (Self::Ligyrophobia,       Lang::En) => "Ligyro",
            (Self::Ligyrophobia,       Lang::Ja) => "大騒音",
            (Self::Limnophobia,        Lang::En) => "Limno",
            (Self::Limnophobia,        Lang::Ja) => "湖",
            (Self::Mechanophobia,      Lang::En) => "Mechano",
            (Self::Mechanophobia,      Lang::Ja) => "機械",
            (Self::Megalophobia,       Lang::En) => "Megalo",
            (Self::Megalophobia,       Lang::Ja) => "巨大物",
            (Self::Merinthophobia,     Lang::En) => "Merintho",
            (Self::Merinthophobia,     Lang::Ja) => "拘束",
            (Self::Meteorophobia,      Lang::En) => "Meteoro",
            (Self::Meteorophobia,      Lang::Ja) => "隕石",
            (Self::Monophobia,         Lang::En) => "Mono",
            (Self::Monophobia,         Lang::Ja) => "孤独",
            (Self::Mysophobia,         Lang::En) => "Myso",
            (Self::Mysophobia,         Lang::Ja) => "汚染",
            (Self::Myxophobia,         Lang::En) => "Myxo",
            (Self::Myxophobia,         Lang::Ja) => "粘液",
            (Self::Necrophobia,        Lang::En) => "Necro",
            (Self::Necrophobia,        Lang::Ja) => "死体",
            (Self::Octophobia,         Lang::En) => "Octo",
            (Self::Octophobia,         Lang::Ja) => "8",
            (Self::Odontophobia,       Lang::En) => "Odonto",
            (Self::Odontophobia,       Lang::Ja) => "歯",
            (Self::Oneirophobia,       Lang::En) => "Oneiro",
            (Self::Oneirophobia,       Lang::Ja) => "夢",
            (Self::Onomatophobia,      Lang::En) => "Onomato",
            (Self::Onomatophobia,      Lang::Ja) => "名称",
            (Self::Ophidiophobia,      Lang::En) => "Ophidio",
            (Self::Ophidiophobia,      Lang::Ja) => "蛇",
            (Self::Ornithophobia,      Lang::En) => "Ornitho",
            (Self::Ornithophobia,      Lang::Ja) => "鳥",
            (Self::Parasitophobia,     Lang::En) => "Parasito",
            (Self::Parasitophobia,     Lang::Ja) => "寄生生物",
            (Self::Pediophobia,        Lang::En) => "Pedio",
            (Self::Pediophobia,        Lang::Ja) => "人形",
            (Self::Phagophobia,        Lang::En) => "Phago",
            (Self::Phagophobia,        Lang::Ja) => "恐食症",
            (Self::Pharmacophobia,     Lang::En) => "Pharmaco",
            (Self::Pharmacophobia,     Lang::Ja) => "薬物",
            (Self::Phasmophobia,       Lang::En) => "Phasmo",
            (Self::Phasmophobia,       Lang::Ja) => "幽霊",
            (Self::Phenogophobia,      Lang::En) => "Phenogo",
            (Self::Phenogophobia,      Lang::Ja) => "羞明",
            (Self::Pogonophobia,       Lang::En) => "Pogono",
            (Self::Pogonophobia,       Lang::Ja) => "ひげ",
            (Self::Potamophobia,       Lang::En) => "Potamo",
            (Self::Potamophobia,       Lang::Ja) => "河川",
            (Self::Potophobia,         Lang::En) => "Poto",
            (Self::Potophobia,         Lang::Ja) => "アルコール",
            (Self::Pyrophobia,         Lang::En) => "Pyro",
            (Self::Pyrophobia,         Lang::Ja) => "火",
            (Self::Rhabdophobia,       Lang::En) => "Rhabdo",
            (Self::Rhabdophobia,       Lang::Ja) => "魔術",
            (Self::Scotophobia,        Lang::En) => "Scoto",
            (Self::Scotophobia,        Lang::Ja) => "暗黒",
            (Self::Selenophobia,       Lang::En) => "Seleno",
            (Self::Selenophobia,       Lang::Ja) => "月",
            (Self::Siderodromophobia,  Lang::En) => "Siderodromo",
            (Self::Siderodromophobia,  Lang::Ja) => "鉄道",
            (Self::Siderophobia,       Lang::En) => "Sidero",
            (Self::Siderophobia,       Lang::Ja) => "星",
            (Self::Stenophobia,        Lang::En) => "Steno",
            (Self::Stenophobia,        Lang::Ja) => "狭所",
            (Self::Symmetrophobia,     Lang::En) => "Symmetro",
            (Self::Symmetrophobia,     Lang::Ja) => "対称",
            (Self::Taphephobia,        Lang::En) => "Taphe",
            (Self::Taphephobia,        Lang::Ja) => "生き埋め",
            (Self::Taurophobia,        Lang::En) => "Tauro",
            (Self::Taurophobia,        Lang::Ja) => "雄牛",
            (Self::Telephonophobia,    Lang::En) => "Telephono",
            (Self::Telephonophobia,    Lang::Ja) => "電話",
            (Self::Teratophobia,       Lang::En) => "Terato",
            (Self::Teratophobia,       Lang::Ja) => "奇形",
            (Self::Thalassophobia,     Lang::En) => "Thalasso",
            (Self::Thalassophobia,     Lang::Ja) => "海洋",
            (Self::Tomophobia,         Lang::En) => "Tomo",
            (Self::Tomophobia,         Lang::Ja) => "手術",
            (Self::Triskadekaphobia,   Lang::En) => "Triskadeka",
            (Self::Triskadekaphobia,   Lang::Ja) => "13",
            (Self::Vestiphobia,        Lang::En) => "Vesti",
            (Self::Vestiphobia,        Lang::Ja) => "衣類",
            (Self::Wiccaphobia,        Lang::En) => "Wicca",
            (Self::Wiccaphobia,        Lang::Ja) => "魔女",
            (Self::Xanthophobia,       Lang::En) => "Xantho",
            (Self::Xanthophobia,       Lang::Ja) => "黄色",
            (Self::Xenoglossophobia,   Lang::En) => "Xenoglosso",
            (Self::Xenoglossophobia,   Lang::Ja) => "外国語",
            (Self::Xenophobia,         Lang::En) => "Xeno",
            (Self::Xenophobia,         Lang::Ja) => "外国人",
            (Self::Zoophobia,          Lang::En) => "Zoo",
            (Self::Zoophobia,          Lang::Ja) => "動物",
        }
    }
}

/// ルールブック 日本語訳版 157頁
#[derive(Clone, Copy)]
pub enum Mania {
    Ablutomania,
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
    pub fn get(&self, _index: u8) -> Self {
        todo!()
    }
    pub fn index(&self) -> u8 {
        *self as u8 + 1
    }
    pub fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Ablutomania,      Lang::En) => "Abluto",
            (Self::Ablutomania,      Lang::Ja) => "洗浄",
            (Self::Aboulomania,      Lang::En) => "Aboulo",
            (Self::Aboulomania,      Lang::Ja) => "無為",
            (Self::Achluomania,      Lang::En) => "Achluo",
            (Self::Achluomania,      Lang::Ja) => "暗闇",
            (Self::Acromaniaheights, Lang::En) => "Acro",
            (Self::Acromaniaheights, Lang::Ja) => "高所",
            (Self::Agathomania,      Lang::En) => "Agatho",
            (Self::Agathomania,      Lang::Ja) => "善良",
            (Self::Agromania,        Lang::En) => "Agro",
            (Self::Agromania,        Lang::Ja) => "広場",
            (Self::Aichmomania,      Lang::En) => "Aichmo",
            (Self::Aichmomania,      Lang::Ja) => "先鋭",
            (Self::Ailuromania,      Lang::En) => "Ailuro",
            (Self::Ailuromania,      Lang::Ja) => "猫",
            (Self::Algomania,        Lang::En) => "Algo",
            (Self::Algomania,        Lang::Ja) => "疼痛性愛",
            (Self::Alliomania,       Lang::En) => "Allio",
            (Self::Alliomania,       Lang::Ja) => "にんにく",
            (Self::Amaxomania,       Lang::En) => "Amaxo",
            (Self::Amaxomania,       Lang::Ja) => "乗り物",
            (Self::Amenomania,       Lang::En) => "Ameno",
            (Self::Amenomania,       Lang::Ja) => "病的快活",
            (Self::Anthomania,       Lang::En) => "Antho",
            (Self::Anthomania,       Lang::Ja) => "花",
            (Self::Arithmomania,     Lang::En) => "Arithmo",
            (Self::Arithmomania,     Lang::Ja) => "計算",
            (Self::Asoticamania,     Lang::En) => "Asotica",
            (Self::Asoticamania,     Lang::Ja) => "浪費",
            (Self::Eremiomania,      Lang::En) => "Eremio",
            (Self::Eremiomania,      Lang::Ja) => "自己",
            (Self::Balletmania,      Lang::En) => "Ballet",
            (Self::Balletmania,      Lang::Ja) => "バレエ",
            (Self::Biliokleptomania, Lang::En) => "Biliokleptо",
            (Self::Biliokleptomania, Lang::Ja) => "書籍約盗癖",
            (Self::Bibliomania,      Lang::En) => "Biblio",
            (Self::Bibliomania,      Lang::Ja) => "書物",
            (Self::Bruxomania,       Lang::En) => "Bruxo",
            (Self::Bruxomania,       Lang::Ja) => "歯ぎしり",
            (Self::Cacodemomania,    Lang::En) => "Cacodemo",
            (Self::Cacodemomania,    Lang::Ja) => "悪霊",
            (Self::Callomania,       Lang::En) => "Callo",
            (Self::Callomania,       Lang::Ja) => "自己愛",
            (Self::Cartacoethes,     Lang::En) => "Cartacoethes",
            (Self::Cartacoethes,     Lang::Ja) => "地図",
            (Self::Catapedamania,    Lang::En) => "Catapeda",
            (Self::Catapedamania,    Lang::Ja) => "飛び降り",
            (Self::Cheimatomania,    Lang::En) => "Cheimato",
            (Self::Cheimatomania,    Lang::Ja) => "寒冷",
            (Self::Choreomania,      Lang::En) => "Choreo",
            (Self::Choreomania,      Lang::Ja) => "舞踏",
            (Self::Clinomania,       Lang::En) => "Clino",
            (Self::Clinomania,       Lang::Ja) => "睡眠",
            (Self::Coimetormania,    Lang::En) => "Coimetor",
            (Self::Coimetormania,    Lang::Ja) => "墓地",
            (Self::Coloromania,      Lang::En) => "Coloro",
            (Self::Coloromania,      Lang::Ja) => "色彩",
            (Self::Coulromania,      Lang::En) => "Coulro",
            (Self::Coulromania,      Lang::Ja) => "ピエロ",
            (Self::Countermania,     Lang::En) => "Counter",
            (Self::Countermania,     Lang::Ja) => "遭遇",
            (Self::Dacnomania,       Lang::En) => "Dacno",
            (Self::Dacnomania,       Lang::Ja) => "殺害",
            (Self::Demonomania,      Lang::En) => "Demono",
            (Self::Demonomania,      Lang::Ja) => "悪魔",
            (Self::Dermatillomania,  Lang::En) => "Dermatillo",
            (Self::Dermatillomania,  Lang::Ja) => "皮膚",
            (Self::Dikemania,        Lang::En) => "Dike",
            (Self::Dikemania,        Lang::Ja) => "正義",
            (Self::Dipsomania,       Lang::En) => "Dipso",
            (Self::Dipsomania,       Lang::Ja) => "アルコール",
            (Self::Doramania,        Lang::En) => "Dora",
            (Self::Doramania,        Lang::Ja) => "毛皮",
            (Self::Doromania,        Lang::En) => "Doro",
            (Self::Doromania,        Lang::Ja) => "贈り物",
            (Self::Drapetomania,     Lang::En) => "Drapeto",
            (Self::Drapetomania,     Lang::Ja) => "逃走",
            (Self::Ecdemiomania,     Lang::En) => "Ecdemio",
            (Self::Ecdemiomania,     Lang::Ja) => "外出",
            (Self::Egomania,         Lang::En) => "Ego",
            (Self::Egomania,         Lang::Ja) => "自己中心",
            (Self::Empleomania,      Lang::En) => "Empleo",
            (Self::Empleomania,      Lang::Ja) => "公職",
            (Self::Enosimania,       Lang::En) => "Enosi",
            (Self::Enosimania,       Lang::Ja) => "戦慄",
            (Self::Epistemomania,    Lang::En) => "Epistemo",
            (Self::Epistemomania,    Lang::Ja) => "知識",
            (Self::Eremiomaniaquiet, Lang::En) => "Eremio (quiet)",
            (Self::Eremiomaniaquiet, Lang::Ja) => "静寂",
            (Self::Etheromania,      Lang::En) => "Ethero",
            (Self::Etheromania,      Lang::Ja) => "エーテル",
            (Self::Gamomania,        Lang::En) => "Gamo",
            (Self::Gamomania,        Lang::Ja) => "求婚",
            (Self::Geliomania,       Lang::En) => "Gelio",
            (Self::Geliomania,       Lang::Ja) => "笑い",
            (Self::Goetomania,       Lang::En) => "Goeto",
            (Self::Goetomania,       Lang::Ja) => "魔術",
            (Self::Graphomania,      Lang::En) => "Grapho",
            (Self::Graphomania,      Lang::Ja) => "筆記",
            (Self::Gymnomania,       Lang::En) => "Gymno",
            (Self::Gymnomania,       Lang::Ja) => "裸体",
            (Self::Habromania,       Lang::En) => "Habro",
            (Self::Habromania,       Lang::Ja) => "幻想",
            (Self::Helminthomania,   Lang::En) => "Helmintho",
            (Self::Helminthomania,   Lang::Ja) => "蟲",
            (Self::Hoplomania,       Lang::En) => "Hoplo",
            (Self::Hoplomania,       Lang::Ja) => "火器",
            (Self::Hydromania,       Lang::En) => "Hydro",
            (Self::Hydromania,       Lang::Ja) => "水",
            (Self::Ichthyomania,     Lang::En) => "Ichthyo",
            (Self::Ichthyomania,     Lang::Ja) => "魚",
            (Self::Iconomania,       Lang::En) => "Icono",
            (Self::Iconomania,       Lang::Ja) => "アイコン",
            (Self::Idolomania,       Lang::En) => "Idolo",
            (Self::Idolomania,       Lang::Ja) => "アイドル",
            (Self::Infomania,        Lang::En) => "Info",
            (Self::Infomania,        Lang::Ja) => "情報",
            (Self::Klazomania,       Lang::En) => "Klazo",
            (Self::Klazomania,       Lang::Ja) => "絶叫",
            (Self::Kleptomania,      Lang::En) => "Klepto",
            (Self::Kleptomania,      Lang::Ja) => "窃盗",
            (Self::Ligyromania,      Lang::En) => "Ligyro",
            (Self::Ligyromania,      Lang::Ja) => "騒音",
            (Self::Linonomania,      Lang::En) => "Linono",
            (Self::Linonomania,      Lang::Ja) => "ひも",
            (Self::Lotterymania,     Lang::En) => "Lottery",
            (Self::Lotterymania,     Lang::Ja) => "宝くじ",
            (Self::Lypemania,        Lang::En) => "Lype",
            (Self::Lypemania,        Lang::Ja) => "うつ",
            (Self::Megalithomania,   Lang::En) => "Megalitho",
            (Self::Megalithomania,   Lang::Ja) => "巨石",
            (Self::Melomania,        Lang::En) => "Melo",
            (Self::Melomania,        Lang::Ja) => "音楽",
            (Self::Metromania,       Lang::En) => "Metro",
            (Self::Metromania,       Lang::Ja) => "作詩",
            (Self::Misomania,        Lang::En) => "Miso",
            (Self::Misomania,        Lang::Ja) => "憎悪",
            (Self::Monomania,        Lang::En) => "Mono",
            (Self::Monomania,        Lang::Ja) => "偏執",
            (Self::Mythomania,       Lang::En) => "Mytho",
            (Self::Mythomania,       Lang::Ja) => "虚言",
            (Self::Nosomania,        Lang::En) => "Noso",
            (Self::Nosomania,        Lang::Ja) => "疾病",
            (Self::Notomania,        Lang::En) => "Noto",
            (Self::Notomania,        Lang::Ja) => "記録",
            (Self::Onomamania,       Lang::En) => "Onoma",
            (Self::Onomamania,       Lang::Ja) => "名前",
            (Self::Onomatomania,     Lang::En) => "Onomato",
            (Self::Onomatomania,     Lang::Ja) => "単語",
            (Self::Onychotillomania, Lang::En) => "Onychotillo",
            (Self::Onychotillomania, Lang::Ja) => "爪損傷",
            (Self::Opsomania,        Lang::En) => "Opso",
            (Self::Opsomania,        Lang::Ja) => "美食",
            (Self::Paramania,        Lang::En) => "Para",
            (Self::Paramania,        Lang::Ja) => "不平",
            (Self::Personamania,     Lang::En) => "Persona",
            (Self::Personamania,     Lang::Ja) => "仮面",
            (Self::Phasmomania,      Lang::En) => "Phasmo",
            (Self::Phasmomania,      Lang::Ja) => "幽霊",
            (Self::Phonomania,       Lang::En) => "Phono",
            (Self::Phonomania,       Lang::Ja) => "殺人",
            (Self::Photomania,       Lang::En) => "Photo",
            (Self::Photomania,       Lang::Ja) => "光線",
            (Self::Antinomiamania,   Lang::En) => "Antinomia",
            (Self::Antinomiamania,   Lang::Ja) => "放浪",
            (Self::Plutomania,       Lang::En) => "Pluto",
            (Self::Plutomania,       Lang::Ja) => "長者",
            (Self::Pseudomania,      Lang::En) => "Pseudo",
            (Self::Pseudomania,      Lang::Ja) => "病的虚言",
            (Self::Pyromania,        Lang::En) => "Pyro",
            (Self::Pyromania,        Lang::Ja) => "放火",
            (Self::QuestionAsking,   Lang::En) => "Question-Asking",
            (Self::QuestionAsking,   Lang::Ja) => "質問",
            (Self::Rhinotillexomania,Lang::En) => "Rhinotillex",
            (Self::Rhinotillexomania,Lang::Ja) => "鼻",
            (Self::Scribbleomania,   Lang::En) => "Scribbleo",
            (Self::Scribbleomania,   Lang::Ja) => "落書き",
            (Self::Siderodromomania, Lang::En) => "Siderodromo",
            (Self::Siderodromomania, Lang::Ja) => "列車",
            (Self::Sophomania,       Lang::En) => "Sopho",
            (Self::Sophomania,       Lang::Ja) => "知性",
            (Self::Technomania,      Lang::En) => "Techno",
            (Self::Technomania,      Lang::Ja) => "テクノ",
            (Self::Thanatomania,     Lang::En) => "Thanato",
            (Self::Thanatomania,     Lang::Ja) => "タナトス",
            (Self::Theomania,        Lang::En) => "Theo",
            (Self::Theomania,        Lang::Ja) => "宗教",
            (Self::Titillomaniac,    Lang::En) => "Titillomaniac",
            (Self::Titillomaniac,    Lang::Ja) => "かき傷",
            (Self::Tomomania,        Lang::En) => "Tomo",
            (Self::Tomomania,        Lang::Ja) => "手術",
            (Self::Trichotillomania, Lang::En) => "Trichotillo",
            (Self::Trichotillomania, Lang::Ja) => "抜毛",
            (Self::Typhlomania,      Lang::En) => "Typhlo",
            (Self::Typhlomania,      Lang::Ja) => "失明",
            (Self::Xenomania,        Lang::En) => "Xeno",
            (Self::Xenomania,        Lang::Ja) => "異国",
            (Self::Zoomania,         Lang::En) => "Zoo",
            (Self::Zoomania,         Lang::Ja) => "動物",
        }
    }
}