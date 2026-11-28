---
lang: ja
translations: none
---

Call of Cthulhu 7th edition(以下、CoC7th)のプレイングツールソフトウェアを設計する。

1. CoC7thをオンラインで遊ぶ時、1: 背景やキャラクター画像の共有 2: 通話 3 チャットやキャラクターなど、数値とテキストデータの編集・共有閲覧 が必要。このうち1,2は運用コストがかかるので、既存サービスに任せ、先ずは3を網羅する。
2. 1の理由で、プレイヤー目線ではソフトウェアを1つ追加することになるので、利便性を損なわないことに注力する。
    1. ブラウザタブの増加を抑えるため、ブラウザの拡張機能として動作出来るようにする
    2. 既存サービス群からのインポート機能を順次追加する(イクスポートはjsonだけに留め、個別対応しない)。
4. . 既存サービスがHTMLを変更したら動かなくなるのは脆いので、ユーザーがクリック・タッチでチャットボックスを指定する 
6. 通信機能について。英語圏では、プライバシーや独立性を重視して、OSSのセルフホストが人気である。日本語圏では、便利さを重視してオンラインプラットフォームが人気である。デフォルトでP2P(webRTC, S)+フォールバック共通サーバー(STUN+TURN)+オプション任意ドメインとすれば、ソフトウェアの提供コストも小さいので、これを目指す。

# References

- [Chaosium: official wiki](https://cthulhuwiki.chaosium.com)

# Function

## キャラクターシート編集機能

- 簡便に入力して保存できることを重視する。
- 既存のキャラクターシートサービスくらいには良い感じの表示になるようにする。
- スマートフォンでも表示(・編集)に支障が無いようにUIを精査する。
- リアルタイム計算, ロール

## チャットボックスでのコマンド実行・保持

予約語(デフォルトは"/")入力をトリガーとして、ダイスロール実行やデータ操作・集計を行う。
既存のサービスが、表示話者の入力DOMとテキストエリアを分けているので、複雑性が許せば話者のDOMも使う。テキストエリア単独でも、改行を使って簡便に表現は可能。

- 予約語入力検知: 以下、「ダイスロール (nDn +n)」～「上達チェック」のセレクトボタンUIをオーバーレイする。
    - テキストエリアの上辺を基準に表示物位置を決定する。mobileで高さが足りない場合は見切れないことを優先して上辺からの座標を正の値にする。
    - セレクタ外(esc)クリック(タッチ): 表示中の重なりUI非表示化、App::State::DisplayとInputを初期化する。テキストエリアの"/"は消去しない。
    - Roll::Resultはデータとして [ロール種アイテム, 判定対象アイテム, 小計値(単一orリスト), 判定結果アイテム]で構成する。
    - 出力テキストとして、 Roll::Result::display()-> "[{ロール種ラベル} {判定対象ラベル} = {目標値}(= からここまで、必要なロール種のみ。リストの場合は{}で囲う。)] {「出目」ラベル}:{出目の数値(重複の無い最終結果のみ)} {判定}: {判定結果のラベル}"で統一。
    - 後で集計するロール種のRoll::Resultのみ、App::State::Stackにスタックする。
    - 実装上の割り切りとして、入力欄の単位などの後置は排除する。単位は" ()""などでラベルに含めてinput外に前置することで、複雑性を抑える。
    - 同様に、インタラクティブUIの1->2->3で1つ前に戻る手段は用意しない。単純なのでescクリアで十分。
    - 必ずすべてのシーンで、appが「初期focus対象」を想定してそこにautofocusを設定しておく。
    - tabやshift+tabで操作可能なdomだけを適切にfocusできるようにする。
- ディスプレイに表示するのは、ルールブック準拠の語(label)であることを徹底する。プレイヤーの知らない実装都合の略称を作らない・使わない・表示しない。ラベルは、1つの変数の属性値(UTF-8)であり、言語(ja,en)別・略称等の引数を取って一意に定まる。
- UIレイアウトの安定のため、単位類は後置せず、ラベルとして前置する。

### Dice Roll (nDn + n)

Calculate {count: i8} *Rand(1..{side: u8}) + {count: i8} *Rand(1..{side: u8}) + {modifier: i8}.
直に呼び出された場合は、ログには残さない。
選択後に表示されるべきインタラクティブUIは、順に

1. Roll::Field::DiceCount(`input[inputmode="numeric"][role="spinbutton"]`): 初期値1、入力範囲は1~100。(focusは直下の「次へ」ボタンに当てる。
2. Roll::Field::DiceSide(`input[inputmode="numeric"][role="spinbutton"]`): 初期値2、入力範囲は3,4,5,6,8,12,16,20,50,100。直下に「戻る」と「次へ」ボタンを置き、focusは「次へ」ボタンに当てる。
3. Roll::Field::Modifier(`input[inputmode="numeric"][role="spinbutton"]`): 初期値0の-100~100)。下部に「戻る」と「決定」ボタン。

### Skill Roll

1. characterに存在する技能を優先ソートしてセレクタとして表示。 text[field](skills: Instance::Fields(attribute: Schema::Attribute::Skill), button[up] button[down], button[next]
- 列指向で表示。1列にまとまる数で無い場合も多いので、画面幅に応じてflexに表示する
2. Roll::Field::Modifier(`input[inputmode="numeric"][role="spinbutton"]`): 初期値0の-100~100), `button[type="submit"]`
3. App::Roll::display()をしつつ結果をApp::Roll::stack(State::Stack(roll: SkillRoll))する。

Characteristic Roll — 能力値判定 (幸運含む)
1. select[characteristic] を表示。nextボタンは無し
2. text[field](Roll::Field::「補正」の英単語), input[number(0(初期値), -100~100), button[submit]をinline表示
 - str~luck。Sanityは含まない (それは狂気判定)

#### 2-2. Sanity Roll — 正気度喪失判定

Bout of Madness (Real Time) — 狂気の発作 (リアルタイム)
intを判定対象としてロール。regularまでの成功で「発狂」が判定結果。failure以下の場合は、「発狂しない」では微妙なので達成度を出して表す。
期間 (ラウンド) (1d10)も同時に実行してBoundOfMadnessResultに含む
regular以上(狂気の発作は)
BoutOfMadnessRealTime,
Bout of Madness (Summary) — 狂気の発作 (サマリー)
RealTimeとの違いは、label文字列と、期間の単位が「時間(hour)」なことだけ
BoutOfMadnessSummary,
Pushed Roll — 失敗後の再挑戦ロール
保持しているskill stack stateの中で、failure以下のものだけ候補化する。この時、新しい順にソートする
既にpush stackに紐づけがあるロールは候補から外すのが正確だが、複雑性が一気に増すので一旦省略。
PushedRoll,
Combined Skill Roll — 2技能(能力値含む)を1ロールで同時判定
1. select[Skill]
2. select[Skill]
3. 出力: "[技能1 技能2] 実値1 実値2 出目 判定1 判定2" (判定1,2: 普通のSkill Rollと同様)。「部分的成功」みたいな組み合わせロール特有の用語は、rulebookに実は無いので、それは扱わない

Development Check - 上達チェック
- ボーナスダイスの無いregular以上のstackのあるskillを候補にする。
- ロールした結果、技能値を超過しているか、96~100の範囲であれば、上達する。1d10を追加で処理して、判定としては 上達 n という出力になる
- 通常の「失敗」「成功」という概念と違うので、Judge::{Developed,Undeveloped}を使う。labelは「上達」「上達なし」

## Specification (仕様)

### Limitation (制限事項)

- 各技能の専門分野(自由記入)の発行は最大4つ。 // 要緩和
- 技能の完全自由欄も最大4つ。 // 要緩和

# キャラクターシート

## 表示最大数

キャラクターシート1枚の中で、各フィールドの表示上限数。tsumugiサンプルを基準に。

| フィールド | たたき台 | 型 | 備考 |
|---|---|---|
| Skill エントリ数 | 60 |  | tsumugiは26技能。Art/Craft等の専門分化が重なると増える |
| Art/Craft等の専門分野サブスキル数 | 10 | | tsumugiは7種 (Acting/Dance/Hanafuda/...) |
| Language (Other) のエントリ数 | 10 | tsumugiは1種 (English) |
| Fighting のエントリ数 | 8 | tsumugiは2種 (Brawl/Sword) |
| Firearms のエントリ数 | 8 | tsumugiは1種 (Handgun) |
| Pilot のエントリ数 | 6 | tsumugiは1種 (Boat) |
| Science のエントリ数 | 8 | tsumugiは1種 (Astronomy) |
| 完全自由技能欄のエントリ数 | 8 | 現Limitationでは4。緩和検討中 |
| Possessions エントリ数 | 30 | tsumugiは13点。武器・防具・その他混在 |
| Possessions 1件あたりのdamageダイスリスト数 | 4 | tsumugiは [[1,10,1],"DB"] で2要素 |
| SignificantPeople の人名リスト文字数 | 2000 | tsumugiは約1700文字。長大なリスト |
| EncountersWithStrangeEntities の文字数 | 3000 | tsumugiは最長フィールド。遭遇ログが蓄積する |
| ArcaneTomesAndSpells 呪文リスト数 | 40 | tsumugiは24種。セッション重ねると増える |
| Backstory 各テキストフィールド文字数 | 1000 | IdeologyAndBeliefsなど |
| PhobiasAndManias エントリ数 | 10 | tsumugiは5件 |
| Memo エントリ数 |  | u8 | |
| Memo 文字数 | |  | |

## 保存最大数

アプリ全体・ユーザー単位での保存上限数。

| フィールド | たたき台 | 備考 |
|---|---|---|
| キャラクターシート総保存数 | 100 | 1ユーザーが複数キャラを管理する想定 |
| 1セッションあたりのキャラクター数 | 10 | PL人数 + NPC想定。KP運用含む |
| Roll::Stack の保持数 | 100 | 1セッション内のロール履歴。古いものから破棄 |
| SkillのDevelopment Check済みフラグ保持数 | 60 | Skill表示最大数と同値 |
| キャラクターのバックアップ(スナップショット)数 | 5 | セッション毎の成長前後比較用 |

# ルール解釈メモ

## 先制の一撃 (奇襲)

- Striking the First Blow - (Surprise)
- 日本語訳版 第2刷 p.102

- 1. KPは対象が攻撃を予期できるか、技能ロールできる。
- 2. 予期できなかった場合、攻撃の技能ロールがファンブルしない限り成功する。
- 3. 奇襲は行動順を意志を持った順に修正する意図がある。つまり、同ラウンドに奇襲を受けた側が行動することは構わないと考えられる。

## Armor Reference

- https://basicroleplaying.org/topic/8902-armor-rules-clarification-coc-7th-ed/

## 職業技能ポイント・個人的な興味ポイント

キャラクターの成長段階での両ポイントの再計算は行わない。
直接の公式見解は無いが、関連ルールの方向性・妥当性の観点から判断した。

- 新クトゥルフ神話TRPGルールブック 第2刷 p.31
- https://cthulhuwiki.chaosium.com/investigators/step-three-occupation-and-skills.html#sample-investigator-occupations
- https://basicroleplaying.org/topic/9511-age-bonus-penalties/

## Language (Own) 母国語

- 母国語の言語名を明記できるべき
- 複数言語対応については、保留

# Unused

```rust
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
    pub fn display(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Pauper,    Language::Ja) => "無一文",
            (Self::Pauper,    Language::En(_)) => "Pauper",
            (Self::Poor,      Language::Ja) => "貧乏",
            (Self::Poor,      Language::En(_)) => "Poor",
            (Self::Average,   Language::Ja) => "平均",
            (Self::Average,   Language::En(_)) => "Average",
            (Self::Wealthy,   Language::Ja) => "裕福",
            (Self::Wealthy,   Language::En(_)) => "Wealthy",
            (Self::Rich,      Language::Ja) => "富豪",
            (Self::Rich,      Language::En(_)) => "Rich",
            (Self::SuperRich, Language::Ja) => "大富豪",
            (Self::SuperRich, Language::En(_)) => "Super Rich",
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
    pub fn from_age(age: u16) -> Self {
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
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Teen,    Language::Ja) => "10代 (15-19)",
            (Self::Teen,    Language::En(_)) => "Teen (15-19)",
            (Self::Young,   Language::Ja) => "若年 (20-39)",
            (Self::Young,   Language::En(_)) => "Young Adult (20-39)",
            (Self::Middle,  Language::Ja) => "中年 (40-49)",
            (Self::Middle,  Language::En(_)) => "Middle-Aged (40-49)",
            (Self::Senior,  Language::Ja) => "熟年 (50-59)",
            (Self::Senior,  Language::En(_)) => "Senior (50-59)",
            (Self::Elderly, Language::Ja) => "老年 (60-69)",
            (Self::Elderly, Language::En(_)) => "Elderly (60-69)",
            (Self::Old,     Language::Ja) => "高齢 (70-79)",
            (Self::Old,     Language::En(_)) => "Old (70-79)",
            (Self::Ancient, Language::Ja) => "超高齢 (80+)",
            (Self::Ancient, Language::En(_)) => "Very Old (80+)",
        }
    }
}
// --- Roll ---
            (Self::AutoFireRoll,    Language::Ja) => "自動火器の連射判定",
            (Self::AutoFireRoll, Language::En(_)) => "Automatic Fire Roll",

// 自動火器射撃判定 (Full Auto Roll)
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

pub enum BulletSetCap {Auto, Specified(u32)}

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
```

# ボーナスダイス・ペナルティダイスのアルゴリズム分析

```python
# python3 -c "from dice import example_judge; example_judge(display=True)"
import random

def dice(level: int = 0):
    arg2 = random.randint(0, 9)
    offset = 1 if arg2 == 0 else 0
    rolls = [random.randint(0, 9) + offset for _ in range(1 + abs(level))]
    if level > 0:
        arg1 = min(rolls)
    elif level < 0:
        arg1 = max(rolls)
    else:
        arg1 = rolls[0]

    return arg1 * 10 + arg2

def judge(target: int = 50, level: int = 0) -> bool:
    if not isinstance(target, int):
        raise TypeError(f"target must be an integer, got {type(target).__name__}")
    if not isinstance(level, int):
        raise TypeError(f"level must be an integer, got {type(level).__name__}")
    if not (1 <= target <= 100):
        raise ValueError(f"target must be between 1 and 100, got {target}")

    result = dice(level)
    return result <= target


def example_judge(trials: int = 100000, display: bool = False):
    targets = [5, 10, 25, 40, 50, 60, 75, 99]
    levels  = [-3, -2, -1, 0, 1, 2, 3]

    matrix = []
    for target in targets:
        row = []
        for level in levels:
            hits = sum(judge(target, level) for _ in range(trials))
            row.append(round(hits / trials, 4))
        matrix.append(row)

    if display:
        RESET  = "\033[0m"
        BOLD   = "\033[1m"
        DIM    = "\033[2m"
        # 値に応じて緑(高)→黄→赤(低)でグラデーション
        def color(v: float) -> str:
            if v >= 0.75: return "\033[92m"    # 明緑
            if v >= 0.50: return "\033[32m"    # 緑
            if v >= 0.25: return "\033[33m"    # 黄
            return "\033[91m"                   # 赤

        col_w  = 8
        tgt_w  = 8
        h_cols = "".join(f"{BOLD}L{l:+d}{RESET}".center(col_w + 9) for l in levels)
        header = f"{DIM}{'target':>{tgt_w}}{RESET} │ {h_cols}"
        sep    = f"{'─' * tgt_w}─┼─{'─' * (col_w * len(levels) + len(levels) - 1 + 18 * len(levels))}"

        print()
        print(f"  {BOLD}judge() True 確率マトリクス{RESET}  ({trials:,} trials/cell)")
        print(f"  {DIM}level: 負=高い目(max), 0=フラット, 正=低い目(min){RESET}")
        print()
        print(f"  {header}")
        print(f"  {DIM}{sep}{RESET}")
        for target, row in zip(targets, matrix):
            cells = " ".join(
                f"{color(v)}{v * 100:5.1f}%{RESET}".rjust(col_w + 9)
                for v in row
            )
            print(f"  {BOLD}{target:>{tgt_w}}{RESET} │ {cells}")
        print()

    return matrix
```

# Schema (draft)

| Term | Meaning |
|-|-|
| identifier | UUID (iCalendar: UID) |
| title | 資源に与えられた名前	シート名。PC名とは別概念（「クトゥルフ神話TRPG 探索者記録用紙」も title たりうる）。混同注意 |	
| creator | 資源を作成した主体。PL名 / HO作成者。値は文字列でも Agent オブジェクトでもよい |
| created | 資源の作成日。紙シートが書かれた日。全体が不明なら YYYY-MM や YYYY でよく、範囲は / 区切り。レコード作成日とは別物 |
| modified | 資源が変更された日。セッション後の更新。配列にして履歴化も可 |
| source | 記述対象がそこから派生した関連資源。保管所URL、元PDF、写真スキャン。変換元の記録として最重要 |
| medium | 資源の物理的な担体。定義域は Physical Resource、値域は Physical Medium（例: 紙、カンヴァス、DVD）	"paper" / "pdf"。 |
| provenance | 作成以降の所有・管理の変遷のうち、真正性・完全性・解釈に重要なもの。歴代の管理者が加えた変更の記述を含みうる。「2026-08に手入力で電子化」「PL交代でGMが引き継ぎ」。自由文でよい |
| conformsTo | 記述対象が準拠する確立された標準	6版/7版の判別。ルールブック版・サプリメント |
| language | 値は BCP 47 の言語タグ、または ISO 639-2/3 の統制語彙。 |
| isVersionOf / hasVersion | 版・改訂・翻案の関係。内容の実質的変更を伴う	成長・ロスト・時代違いの同一探索者を繋ぐ |
| isFormatOf / hasFormat | 実質的に同一だが別フォーマットの既存資源。hasFormat の逆プロパティ / 同じシートのPDF/ココフォリア駒/画像。エクスポート先を持つなら有用 |
| isPartOf / hasPart | 記述対象が物理的または論理的に含まれる関連資源。キャンペーン/卓への所属。isPartOf: "第2回 悪霊の家"。逆に hasPart はシートが下位文書（背景設定、日記）を持つ場合 |
| description | 資源の説明。シートについての注記。PCの設定ではない点に注意（それは投稿者データ側） |
| abstract | 資源の要約。description と役割が被る。両方使うなら「abstract=短い要約、description=詳細」で分ける。片方でよい |
| rights / license / rightsHolder | 権利情報 / ライセンス文書 / 権利保持者	公開・共有するなら。rightsHolder に Chaosium・アークライト、license に自分の設定の扱い |
| publisher | 資源を公開した主体。保管所に上げた、Webに載せた、などの公開行為があるなら |
| contributor | 資源に寄与した主体。共同作成・GM補正・成長の記入者 |
| type | 資源の性質・ジャンル。DCMI Type Vocabulary の統制語（Text など）が推奨で、粒度が粗すぎる。自作の "investigator_sheet" を入れるなら独自キーにすべき。dcterms として使うなら Text 一択で情報量ゼロ |
| format | ファイル形式・媒体・寸法。MIMEタイプ推奨。medium と使い分ける（format=デジタル形式、medium=物理担体）。両方は要らないことが多い |
| extent | 大きさ・継続時間。紙シートなら「A4 2枚」。物理原本を記述するなら意味がある |
| issued | 公式に公開・発行された日。卓で初めて使った日、公開した日 |
| valid | 資源が有効な日付・期間。キャンペーン中の有効期間、「1923年時点の探索者」の作中時制 |
| available | 利用可能になった/なる日付。ほぼ issued と重複。
| spatial | 空間的な適用範囲。舞台となる土地（アーカム、東京）。ただしこれは「資源の内容が扱う場所」なので、シナリオ寄りの情報。PCの出身地は別キーに |
| temporal | 時間的な適用範囲。作中年代（1920s）。spatial と対で使うと収まりがいい |
| subject | 資源の主題。タグ。["CoC7", "現代日本", "秘匿HO"]。統制語彙推奨だが自由タグでも運用可 |
| relation | 関連する資源。上記の関係語すべての親。具体的な関係が言えるならそちらを使う。分類不能な関連だけここ |
| references / isReferencedBy | 参照する / される	シナリオ、参考にした他PC、元ネタ。ゆるい繋がりを書くのに便利 |
| replaces / isReplacedBy | 置き換える / 置き換えられる	作り直し・リメイク版。isVersionOf（同一性が続く）との違いは、こちらは別物への交代。ロスト後の新PCは replaces が適切 |
| requires / isRequiredBy | 機能に必要とする / される	「このシートは追加ルールブックを必要とする」。サプリ依存を書くなら |
| alternative | 別題・副題 | PCの通り名・偽名を入れたくなるが、それはシートの別題ではなくPCの属性。
| accessRights | 誰がアクセスできるかを示すセキュリティ状態	秘匿HOの公開範囲に転用できそうに見えるが、DCMIの想定は所蔵機関のアクセス制限。ゲーム内の秘匿は独自キーにすべき（visibility: "gm_only" など） |