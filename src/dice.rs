use rand::{rng, RngExt};
use crate::{n_d_n, Lang};

use crate::table::{
    Roll,
    FAILED_CASTING_MINOR, FAILED_CASTING_MAJOR,
    MADNESS_REALTIME, MADNESS_SUMMARY,
    MANIAS, PHOBIAS,
};

// ============================================================
// Roll
// ============================================================

// ============================================================
// Bonus/Penalty dice
// ============================================================

/// ボーナス/ペナルティダイスを考慮した 1d100 ロール
/// bonus > 0 → min選択（ボーナス）、bonus < 0 → max選択（ペナルティ）、0 → 通常
/// 戻り値: (採用値, 全候補リスト)
fn roll_with_bonus(bonus: i32) -> (u32, Vec<u32>) {
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
// ResultLevel
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultLevel {
    Fumble,
    Failure,
    Regular,
    Hard,
    Extreme,
    Critical,
}

impl ResultLevel {
    /// fumbleable: 連射でhard以上の難易度段階に入った場合 true（ファンブル閾値が96固定になる）
    pub fn from_values(total: u32, difficulty: u32, fumbleable: bool) -> Self {
        let fumble_at = if difficulty < 50 || fumbleable { 96 } else { 100 };
        if total == 1                   { Self::Critical }
        else if total >= fumble_at      { Self::Fumble }
        else if total <= difficulty / 5 { Self::Extreme }
        else if total <= difficulty / 2 { Self::Hard }
        else if total <= difficulty     { Self::Regular }
        else                            { Self::Failure }
    }

    /// 難易度指定判定（R/H/E/C 指定時）
    /// 成功段階は出さず、成功/失敗/クリティカル/ファンブルのみ返す
    pub fn with_difficulty_level(total: u32, difficulty: u32) -> Self {
        let fumble_at = if difficulty < 50 { 96 } else { 100 };
        if total == 1              { Self::Critical }
        else if total >= fumble_at { Self::Fumble }
        else if total <= difficulty { Self::Regular }
        else                       { Self::Failure }
    }

    pub fn is_success(self) -> bool { self >= Self::Regular }
    pub fn is_failure(self) -> bool { self <= Self::Failure }

    pub fn label(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Critical, Lang::Ja) => "クリティカル",
            (Self::Critical, Lang::En) => "Critical",
            (Self::Extreme,  Lang::Ja) => "イクストリーム成功",
            (Self::Extreme,  Lang::En) => "Extreme Success",
            (Self::Hard,     Lang::Ja) => "ハード成功",
            (Self::Hard,     Lang::En) => "Hard Success",
            (Self::Regular,  Lang::Ja) => "レギュラー成功",
            (Self::Regular,  Lang::En) => "Regular Success",
            (Self::Failure,  Lang::Ja) => "失敗",
            (Self::Failure,  Lang::En) => "Failure",
            (Self::Fumble,   Lang::Ja) => "ファンブル",
            (Self::Fumble,   Lang::En) => "Fumble",
        }
    }
}

impl std::fmt::Display for ResultLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label(Lang::Ja))
    }
}

// ============================================================
// 判定結果型
// ============================================================

#[derive(Debug)]
pub enum SkillRollError {
    BonusDiceOutOfRange,
}

/// 技能判定の結果
#[derive(Debug)]
pub struct SkillRollResult {
    pub total: u32,
    pub dice_candidates: Vec<u32>,
    pub bonus_dice: i32,
    pub effective_difficulty: Option<u32>,
    pub level: Option<ResultLevel>,
}

impl SkillRollResult {
    pub fn is_success(&self) -> bool { self.level.map_or(false, |l| l.is_success()) }
    pub fn is_critical(&self) -> bool { self.level == Some(ResultLevel::Critical) }
    pub fn is_fumble(&self)   -> bool { self.level == Some(ResultLevel::Fumble) }
}

impl std::fmt::Display for SkillRollResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let candidates = self.dice_candidates.iter()
            .map(|d| d.to_string()).collect::<Vec<_>>().join(", ");
        write!(f, "1D100 bonus=[{}] candidates=[{}] total={}", self.bonus_dice, candidates, self.total)?;
        if let Some(d) = self.effective_difficulty {
            write!(f, " difficulty={}", d)?;
        }
        if let Some(lvl) = self.level {
            write!(f, " result={:?}", lvl)?;
        }
        Ok(())
    }
}

/// 組み合わせ判定の結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombineOutcome {
    FullSuccess,
    PartialSuccess,
    Failure,
}

#[derive(Debug)]
pub struct CombineRollResult {
    pub total: u32,
    pub level_1: ResultLevel,
    pub level_2: ResultLevel,
    pub difficulty_1: u32,
    pub difficulty_2: u32,
    pub outcome: CombineOutcome,
}

impl std::fmt::Display for CombineRollResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "1D100 total={} skill1={} skill2={} level1={:?} level2={:?} outcome={:?}",
            self.total, self.difficulty_1, self.difficulty_2,
            self.level_1, self.level_2, self.outcome)
    }
}

/// 連射の1ボレー分の命中結果
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

impl std::fmt::Display for VolleyResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let candidates = self.dice_candidates.iter()
            .map(|d| d.to_string()).collect::<Vec<_>>().join(", ");
        write!(f, "[{}] stage={} total={} candidates=[{}] result={:?} hit={} impale={} jammed={}",
            self.loop_index, self.stage, self.total, candidates,
            self.level, self.hit, self.impale, self.jammed)
    }
}

/// 連射全体の結果
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

#[derive(Debug)]
pub enum FullAutoWarning {
    BulletsClamped { original: u32 },
    BrokenNumberNegated,
    BulletSetCapClampedLow { clamped_to: u32 },
    BulletSetCapClampedHigh { clamped_to: u32, low_skill: bool },
}

impl std::fmt::Display for FullAutoWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum FullAutoError {
    NoBullets,
    NoSkill,
    BonusDiceOutOfRange,
    BulletSetCapNonPositive,
}

impl std::fmt::Display for FullAutoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::fmt::Display for FullAutoResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for w in &self.warnings {
            writeln!(f, "{}", w)?;
        }
        write!(f, "bonus=[{}]", self.bonus_dice)?;
        for v in &self.volleys {
            write!(f, "\n{}", v)?;
        }
        if self.stopped_by_difficulty {
            write!(f, "\nstopped_by_difficulty=true")?;
        }
        write!(f, "\nhit_total={} impale_total={} remaining={}",
            self.hit_total, self.impale_total, self.remaining_bullets)
    }
}

/// ランダム表の結果
#[derive(Debug)]
pub struct TableResult {
    pub roll_type: Roll,
    pub roll: u32,
    pub label: &'static str,
}

impl std::fmt::Display for TableResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "roll_type={:?} roll={} label={}", self.roll_type, self.roll, self.label)
    }
}

/// 狂気表の継続時間単位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Rounds,
    Hours,
}

/// 狂気表の結果
#[derive(Debug)]
pub struct MadnessResult {
    pub roll_type: Roll,
    pub roll: u32,
    pub label: &'static str,
    pub duration_roll: u32,
    pub duration_unit: DurationUnit,
}

impl std::fmt::Display for MadnessResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "roll_type={:?} roll={} label={} duration={}({:?})",
            self.roll_type, self.roll, self.label, self.duration_roll, self.duration_unit)
    }
}

// ============================================================
// 技能判定
// ============================================================

#[derive(Debug, Clone, Copy)]
pub enum DifficultySpec {
    None,
    Regular,
    Hard,
    Extreme,
    Critical,
}

/// 技能判定
///
/// # 境界・特殊挙動
/// - bonus_dice の絶対値が 100 超 → Err
/// - difficulty == None かつ bonus_dice == 0 → 出目のみ（level=None）
/// - difficulty == Some(0) → None 扱い
/// - Hard → difficulty / 2、Extreme → difficulty / 5、Critical → difficulty = 0
pub fn skill_roll(
    bonus_dice: i32,
    difficulty: Option<u32>,
    difficulty_spec: DifficultySpec,
) -> Result<SkillRollResult, SkillRollError> {
    if bonus_dice.unsigned_abs() > 100 {
        return Err(SkillRollError::BonusDiceOutOfRange);
    }

    let difficulty = difficulty.filter(|&d| d > 0);

    let effective_diff: Option<u32> = difficulty.map(|d| match difficulty_spec {
        DifficultySpec::Hard     => d / 2,
        DifficultySpec::Extreme  => d / 5,
        DifficultySpec::Critical => 0,
        _                        => d,
    });

    let (total, dice_candidates) = roll_with_bonus(bonus_dice);

    let level: Option<ResultLevel> = effective_diff.map(|d| match difficulty_spec {
        DifficultySpec::None => ResultLevel::from_values(total, d, false),
        _                    => ResultLevel::with_difficulty_level(total, d),
    });

    Ok(SkillRollResult { total, dice_candidates, bonus_dice, effective_difficulty: effective_diff, level })
}

// ============================================================
// 組み合わせ判定
// ============================================================

/// 1回の 1d100 を 2技能値に対してそれぞれ独立に判定する
pub fn combine_roll(difficulty_1: u32, difficulty_2: u32) -> CombineRollResult {
    let total = n_d_n(1, 100);
    let level_1 = ResultLevel::from_values(total, difficulty_1, false);
    let level_2 = ResultLevel::from_values(total, difficulty_2, false);
    let outcome = if level_1.is_success() && level_2.is_success() {
        CombineOutcome::FullSuccess
    } else if level_1.is_success() || level_2.is_success() {
        CombineOutcome::PartialSuccess
    } else {
        CombineOutcome::Failure
    };
    CombineRollResult { total, level_1, level_2, difficulty_1, difficulty_2, outcome }
}

// ============================================================
// 自動火器射撃判定
// ============================================================

/// 連射停止難易度
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StopAt {
    None,
    Regular,
    Hard,
    Extreme,
}

/// ボレーあたり弾数上限の指定
#[derive(Debug, Clone, Copy)]
pub enum BulletSetCap {
    Auto,
    Specified(u32),
}

/// 自動火器射撃判定
///
/// # 境界・特殊挙動
/// - bullet_count > 100 → クランプ+warning
/// - bullet_count == 0 / skill == 0 → Err
/// - broken_number < 0 → 絶対値補正+warning
/// - bonus_dice 絶対値 > 2 → Err
/// - BulletSetCap::Specified(0) → Err
/// - BulletSetCap::Specified(1〜2) → 下限3にクランプ+warning
/// - skill <= 39 → BulletSetCap 上限は3固定
/// - skill >= 40 → BulletSetCap 上限は skill/10
/// - ジャム（total >= broken_number） → 即時終了
/// - 難易度段階: レギュラー→ハード→イクストリーム→クリティカルの順に4段階
/// - ハード以降は fumbleable=true（ファンブル閾値96固定）
pub fn full_auto(
    bullet_count: u32,
    skill: u32,
    broken_number: i32,
    bonus_dice: i32,
    stop_at: StopAt,
    bullet_set_cap: BulletSetCap,
) -> Result<FullAutoResult, FullAutoError> {
    let mut warnings: Vec<FullAutoWarning> = Vec::new();
    let mut bullet_count = bullet_count;

    if bullet_count > 100 {
        warnings.push(FullAutoWarning::BulletsClamped { original: bullet_count });
        bullet_count = 100;
    }
    if bullet_count == 0 { return Err(FullAutoError::NoBullets); }
    if skill == 0        { return Err(FullAutoError::NoSkill); }

    let broken_number = if broken_number < 0 {
        warnings.push(FullAutoWarning::BrokenNumberNegated);
        broken_number.unsigned_abs()
    } else {
        broken_number as u32
    };

    if bonus_dice.unsigned_abs() > 2 {
        return Err(FullAutoError::BonusDiceOutOfRange);
    }

    let bullet_set_cap: u32 = match bullet_set_cap {
        BulletSetCap::Auto => {
            if skill <= 39 { 3 } else { skill / 10 }
        }
        BulletSetCap::Specified(v) => {
            if v == 0 { return Err(FullAutoError::BulletSetCapNonPositive); }
            let cap_max = if skill <= 39 { 3 } else { skill / 10 };
            if v > cap_max {
                warnings.push(FullAutoWarning::BulletSetCapClampedHigh {
                    clamped_to: cap_max,
                    low_skill: skill <= 39,
                });
                cap_max
            } else if v < 3 {
                warnings.push(FullAutoWarning::BulletSetCapClampedLow { clamped_to: 3 });
                3
            } else {
                v
            }
        }
    };

    let mut volleys: Vec<VolleyResult> = Vec::new();
    let mut loop_count = 0u32;
    let mut hit_total = 0u32;
    let mut impale_total = 0u32;
    let mut current_bonus = bonus_dice;
    let mut stopped_by_difficulty = false;
    let mut prev_stage = 0u32;

    'outer: for stage in 0u32..4 {
        let fumbleable = stage >= 1;
        let mut first_in_stage = stage != prev_stage;
        prev_stage = stage;

        while current_bonus >= -2 {
            loop_count += 1;
            let stage_changed = first_in_stage;
            first_in_stage = false;
            let (total, dice_candidates) = roll_with_bonus(current_bonus);
            let level = ResultLevel::from_values(total, skill, fumbleable);

            if total >= broken_number {
                volleys.push(VolleyResult {
                    stage, stage_changed, loop_index: loop_count, total, dice_candidates, level,
                    hit: 0, impale: 0, jammed: true,
                });
                return Ok(FullAutoResult {
                    warnings, bonus_dice, volleys,
                    hit_total, impale_total,
                    remaining_bullets: bullet_count,
                    stopped_by_difficulty: false,
                    jammed: true,
                });
            }

            let bullet_set = get_bullet_set(skill, bullet_set_cap);
            let is_last = bullet_count < bullet_set;
            let (hit, impale, lost) = bullet_result(bullet_count, level, skill, bullet_set, is_last, stage);

            hit_total += hit;
            impale_total += impale;
            bullet_count = bullet_count.saturating_sub(lost);

            volleys.push(VolleyResult {
                stage, stage_changed, loop_index: loop_count, total, dice_candidates, level,
                hit, impale, jammed: false,
            });

            if bullet_count == 0 { break 'outer; }
            current_bonus -= 1;
        }

        if should_stop(stop_at, stage) {
            stopped_by_difficulty = true;
            break;
        }
        current_bonus += 1;
    }

    Ok(FullAutoResult {
        warnings, bonus_dice, volleys,
        hit_total, impale_total,
        remaining_bullets: bullet_count,
        stopped_by_difficulty,
        jammed: false,
    })
}

fn get_bullet_set(skill: u32, cap: u32) -> u32 {
    let base = if skill < 30 { 3 } else { skill / 10 };
    base.min(cap)
}

fn should_stop(stop_at: StopAt, stage: u32) -> bool {
    match stop_at {
        StopAt::Regular => true,
        StopAt::Hard    => stage >= 1,
        StopAt::Extreme => stage >= 2,
        StopAt::None    => false,
    }
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
        if is_last {
            let h = (bullet_count + 1) / 2;
            (h, 0, bullet_count)
        } else {
            (hit_base, 0, bullet_set)
        }
    } else if is_impale {
        if is_last {
            let i = bullet_count / 2;
            (bullet_count - i, i, bullet_count)
        } else {
            let i = bullet_set / 2;
            (bullet_set - i, i, bullet_set)
        }
    } else {
        (0, 0, bullet_set.min(bullet_count))
    }
}

// ============================================================
// ランダム表ロール
// ============================================================

/// 狂気の発作（リアルタイム） — 1d10 + 継続ラウンド 1d10
pub fn roll_madness_realtime() -> MadnessResult {
    let n = n_d_n(1, 10) as usize;
    MadnessResult {
        roll_type: Roll::BoutOfMadnessRealTime,
        roll: n as u32,
        label: MADNESS_REALTIME[n - 1].label,
        duration_roll: n_d_n(1, 10),
        duration_unit: DurationUnit::Rounds,
    }
}

/// 狂気の発作（サマリー） — 1d10 + 継続時間 1d10
pub fn roll_madness_summary() -> MadnessResult {
    let n = n_d_n(1, 10) as usize;
    MadnessResult {
        roll_type: Roll::BoutOfMadnessSummary,
        roll: n as u32,
        label: MADNESS_SUMMARY[n - 1].label,
        duration_roll: n_d_n(1, 10),
        duration_unit: DurationUnit::Hours,
    }
}

/// キャスティング・ロール失敗（小） — 1d8
pub fn roll_failed_casting_minor() -> TableResult {
    let n = n_d_n(1, 8) as usize;
    TableResult { roll_type: Roll::FailedCastingMinor, roll: n as u32, label: FAILED_CASTING_MINOR[n - 1].label }
}

/// キャスティング・ロール失敗（大） — 1d8
pub fn roll_failed_casting_major() -> TableResult {
    let n = n_d_n(1, 8) as usize;
    TableResult { roll_type: Roll::FailedCastingMajor, roll: n as u32, label: FAILED_CASTING_MAJOR[n - 1].label }
}

/// 恐怖症表 — 1d100
pub fn roll_phobia() -> TableResult {
    let n = n_d_n(1, 100) as usize;
    TableResult { roll_type: Roll::PhobiaTable, roll: n as u32, label: PHOBIAS[n - 1].label }
}

/// マニア表 — 1d100
pub fn roll_mania() -> TableResult {
    let n = n_d_n(1, 100) as usize;
    TableResult { roll_type: Roll::ManiaTable, roll: n as u32, label: MANIAS[n - 1].label }
}
