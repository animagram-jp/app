use crate::Lang;
use crate::character,

pub enum Roll { 
    DiceRoll,
    SkillRoll,
    CharacteristicRoll,
    SanityRoll,
    BoutOfMadnessRealTime,
    BoutOfMadnessSummary,
    PushedRoll,
    CombinedSkillRoll,
    PhobiaTable,
    ManiaTable,
    AutoFireRoll,
    FailedCastingMinor,
    FailedCastingMajor,
    DevelopmentCheck,
}

pub struct DiceRoll {
    character:   character::Instance,
    roll_select:   Roll,
    select: Vec<DiceRollSelect>,
    target_select: (character::Schema::Characteristic,character::Schema::Skill),
    input_counts: u16,
    input_sides: [2, 3, 4, 5, 6, 8, 10, 12, 20, 100];
    input_modifier: i16,
    result: RollResult,
}

pub struct RollResult {
    roll_total: Vec<i16>,
    roll_judge: Option(Vev<RollJudge>),
}

pub enum RollJudge {
    Failure,
    Famble,
    Regular,
    Hard,
    Extreme,
    Critical,
    Developed,
    Undeveloped,
}

impl Roll {
    pub fn label(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::DiceRoll,              Lang::Ja) => "ダイスロール (nDn +-n)",
            (Self::DiceRoll,              Lang::En) => "Dice Roll (nDn +-n)",
            (Self::SkillRoll,             Lang::Ja) => "技能判定",
            (Self::SkillRoll,             Lang::En) => "Skill Roll",
            (Self::CharacteristicRoll,    Lang::Ja) => "能力値判定 (幸運含む)",
            (Self::CharacteristicRoll,    Lang::En) => "Characteristic Roll",
            (Self::SanityRoll,            Lang::Ja) => "正気度判定",
            (Self::SanityRoll,            Lang::En) => "Sanity Roll",
            (Self::BoutOfMadnessRealTime, Lang::Ja) => "狂気の発作 (リアルタイム)",
            (Self::BoutOfMadnessRealTime, Lang::En) => "Bout of Madness (Real Time)",
            (Self::BoutOfMadnessSummary,  Lang::Ja) => "狂気の発作 (サマリー)",
            (Self::BoutOfMadnessSummary,  Lang::En) => "Bout of Madness (Summary)",
            (Self::PushedRoll,            Lang::Ja) => "プッシュロール",
            (Self::PushedRoll,            Lang::En) => "Pushed Roll",
            (Self::CombinedSkillRoll,     Lang::Ja) => "組み合わせ判定",
            (Self::CombinedSkillRoll,     Lang::En) => "Combined Skill Roll",
            (Self::PhobiaTable,           Lang::Ja) => "恐怖症表",
            (Self::PhobiaTable,           Lang::En) => "Phobia Table",
            (Self::ManiaTable,            Lang::Ja) => "マニア表",
            (Self::ManiaTable,            Lang::En) => "Mania Table",
            (Self::AutoFireRoll,          Lang::Ja) => "自動火器の連射判定",
            (Self::AutoFireRoll,          Lang::En) => "Automatic Fire Roll",
            (Self::FailedCastingMinor,    Lang::Ja) => "呪文失敗 (小)",
            (Self::FailedCastingMinor,    Lang::En) => "Failed Casting (Minor)",
            (Self::FailedCastingMajor,    Lang::Ja) => "呪文失敗 (大)",
            (Self::FailedCastingMajor,    Lang::En) => "Failed Casting (Major)",
            (Self::DevelopmentCheck,      Lang::En) => "Development Check",
            (Self::DevelopmentCheck,      Lang::Ja) => "上達チェック",
        }
    }

    pub fn all() -> &'static [Roll] {
        &[
            Self::DiceRoll,
            Self::SkillRoll,
            Self::CharacteristicRoll,
            Self::SanityRoll,
            Self::BoutOfMadnessRealTime,
            Self::BoutOfMadnessSummary,
            Self::PushedRoll,
            Self::CombinedSkillRoll,
            Self::PhobiaTable,
            Self::ManiaTable,
            Self::AutoFireRoll,
            Self::FailedCastingMinor,
            Self::FailedCastingMajor,
            Self::DevelopmentCheck,
        ]
    }
}

pub enum BoutOfMadnessRealTime {

}

impl MadnessRealTime {
    pub label(self, lang: Lang) -> & static str{
        match (self, lang) {

        }
    }
}

pub enum MadnessSummary {

}

impl MadnessSummary {
    pub label(self, lang: Lang) -> & static str{
        match (self, lang) {

        }
    }
}

pub enum MadnessSummary {

}

impl MadnessSummary {
    pub label(self, lang: Lang) -> & static str{
        match (self, lang) {

        }
    }
}

/// ルールブック 日本語訳版 153頁
pub static MADNESS_REALTIME: [TableEntry; 10] = [
    TableEntry { id: 1, label: "健忘症" },
    TableEntry { id: 2, label: "身体症状症" },
    TableEntry { id: 3, label: "暴力衝動" },
    TableEntry { id: 4, label: "偏執症" },
    TableEntry { id: 5, label: "重要な人々" },
    TableEntry { id: 6, label: "失神" },
    TableEntry { id: 7, label: "パニックになって逃亡する" },
    TableEntry { id: 8, label: "身体的ヒステリーもしくは感情爆発" },
    TableEntry { id: 9, label: "恐怖症" },
    TableEntry { id: 10, label: "マニア" },
];

/// ルールブック 日本語訳版 155頁
pub static MADNESS_SUMMARY: [TableEntry; 10] = [
    TableEntry { id: 1, label: "健忘症" },
    TableEntry { id: 2, label: "盗難" },
    TableEntry { id: 3, label: "暴行" },
    TableEntry { id: 4, label: "暴力" },
    TableEntry { id: 5, label: "イデオロギー／信念" },
    TableEntry { id: 6, label: "重要な人々" },
    TableEntry { id: 7, label: "収容" },
    TableEntry { id: 8, label: "パニック" },
    TableEntry { id: 9, label: "恐怖症" },
    TableEntry { id: 10, label: "マニア" },
];

/// ルールブック 日本語訳版 174頁
pub static FAILED_CASTING_MINOR: [TableEntry; 8] = [
    TableEntry { id: 1, label: "視界のかすみ、または一時的な失明" },
    TableEntry { id: 2, label: "悲鳴、声、雑音が発せられる" },
    TableEntry { id: 3, label: "強風などの大気現象" },
    TableEntry { id: 4, label: "術者かその場に居合わせた者、あるいは壁などからの出血" },
    TableEntry { id: 5, label: "奇妙な幻視と幻覚" },
    TableEntry { id: 6, label: "付近の小動物たちが爆発する" },
    TableEntry { id: 7, label: "硫黄の悪臭" },
    TableEntry { id: 8, label: "クトゥルフ神話の怪物が偶然召喚される" },
];

/// ルールブック 日本語訳版 P175
pub static FAILED_CASTING_MAJOR: [TableEntry; 8] = [
    TableEntry { id: 1, label: "大地が震え、壁に亀裂が入って崩れる" },
    TableEntry { id: 2, label: "叙事詩的な電撃" },
    TableEntry { id: 3, label: "血が空から降る" },
    TableEntry { id: 4, label: "術者の手がしなび、焼けただれる" },
    TableEntry { id: 5, label: "術者は不自然に年をとる" },
    TableEntry { id: 6, label: "クトゥルフ神話存在が現れ、術者や周囲をに被害を与える" },
    TableEntry { id: 7, label: "術者や近くの全員が遠い時代か場所に吸い込まれる" },
    TableEntry { id: 8, label: "クトゥルフ神話の神格が偶然招来される" },
];

/// ルールブック 日本語訳版 156頁
pub static PHOBIAS: [TableEntry; 100] = [
    TableEntry { id: 1, label: "入浴" },
    TableEntry { id: 2, label: "高所" },
    TableEntry { id: 3, label: "飛行" },
    TableEntry { id: 4, label: "広場" },
    TableEntry { id: 5, label: "鶏肉" },
    TableEntry { id: 6, label: "ニンニク" },
    TableEntry { id: 7, label: "乗車" },
    TableEntry { id: 8, label: "風" },
    TableEntry { id: 9, label: "男性" },
    TableEntry { id: 10, label: "イングランド" },
    TableEntry { id: 11, label: "花" },
    TableEntry { id: 12, label: "切断" },
    TableEntry { id: 13, label: "クモ" },
    TableEntry { id: 14, label: "稲妻" },
    TableEntry { id: 15, label: "廃墟" },
    TableEntry { id: 16, label: "笛" },
    TableEntry { id: 17, label: "細菌" },
    TableEntry { id: 18, label: "銃弾" },
    TableEntry { id: 19, label: "落下" },
    TableEntry { id: 20, label: "書物" },
    TableEntry { id: 21, label: "植物" },
    TableEntry { id: 22, label: "美女" },
    TableEntry { id: 23, label: "低温" },
    TableEntry { id: 24, label: "時計" },
    TableEntry { id: 25, label: "閉所" },
    TableEntry { id: 26, label: "道化師" },
    TableEntry { id: 27, label: "犬" },
    TableEntry { id: 28, label: "悪魔" },
    TableEntry { id: 29, label: "群集" },
    TableEntry { id: 30, label: "歯科医" },
    TableEntry { id: 31, label: "処分" },
    TableEntry { id: 32, label: "毛皮" },
    TableEntry { id: 33, label: "構断" },
    TableEntry { id: 34, label: "教会" },
    TableEntry { id: 35, label: "鏡" },
    TableEntry { id: 36, label: "ピン" },
    TableEntry { id: 37, label: "昆虫" },
    TableEntry { id: 38, label: "猫" },
    TableEntry { id: 39, label: "橋" },
    TableEntry { id: 40, label: "老人" },
    TableEntry { id: 41, label: "女性" },
    TableEntry { id: 42, label: "血液" },
    TableEntry { id: 43, label: "過失" },
    TableEntry { id: 44, label: "接触" },
    TableEntry { id: 45, label: "爬虫類" },
    TableEntry { id: 46, label: "霧" },
    TableEntry { id: 47, label: "銃器" },
    TableEntry { id: 48, label: "水" },
    TableEntry { id: 49, label: "睡眠" },
    TableEntry { id: 50, label: "医師" },
    TableEntry { id: 51, label: "魚" },
    TableEntry { id: 52, label: "ゴキブリ" },
    TableEntry { id: 53, label: "雷鳴" },
    TableEntry { id: 54, label: "野菜" },
    TableEntry { id: 55, label: "大騒音" },
    TableEntry { id: 56, label: "湖" },
    TableEntry { id: 57, label: "機械" },
    TableEntry { id: 58, label: "巨大物" },
    TableEntry { id: 59, label: "拘束" },
    TableEntry { id: 60, label: "隕石" },
    TableEntry { id: 61, label: "孤独" },
    TableEntry { id: 62, label: "汚染" },
    TableEntry { id: 63, label: "粘液" },
    TableEntry { id: 64, label: "死体" },
    TableEntry { id: 65, label: "8" },
    TableEntry { id: 66, label: "歯" },
    TableEntry { id: 67, label: "夢" },
    TableEntry { id: 68, label: "名称" },
    TableEntry { id: 69, label: "蛇" },
    TableEntry { id: 70, label: "鳥" },
    TableEntry { id: 71, label: "寄生生物" },
    TableEntry { id: 72, label: "人形" },
    TableEntry { id: 73, label: "恐食症" },
    TableEntry { id: 74, label: "薬物" },
    TableEntry { id: 75, label: "幽霊" },
    TableEntry { id: 76, label: "羞明" },
    TableEntry { id: 77, label: "ひげ" },
    TableEntry { id: 78, label: "河川" },
    TableEntry { id: 79, label: "アルコール" },
    TableEntry { id: 80, label: "火" },
    TableEntry { id: 81, label: "魔術" },
    TableEntry { id: 82, label: "暗黒" },
    TableEntry { id: 83, label: "月" },
    TableEntry { id: 84, label: "鉄道" },
    TableEntry { id: 85, label: "星" },
    TableEntry { id: 86, label: "狭所" },
    TableEntry { id: 87, label: "対称" },
    TableEntry { id: 88, label: "生き埋め" },
    TableEntry { id: 89, label: "雄牛" },
    TableEntry { id: 90, label: "電話" },
    TableEntry { id: 91, label: "奇形" },
    TableEntry { id: 92, label: "海洋" },
    TableEntry { id: 93, label: "手術" },
    TableEntry { id: 94, label: "13" },
    TableEntry { id: 95, label: "衣類" },
    TableEntry { id: 96, label: "魔女" },
    TableEntry { id: 97, label: "黄色" },
    TableEntry { id: 98, label: "外国語" },
    TableEntry { id: 99, label: "外国人" },
    TableEntry { id: 100, label: "動物" },
];

/// ルールブック 日本語訳版 157頁
pub static MANIAS: [TableEntry; 100] = [
    TableEntry { id: 1, label: "洗浄" },
    TableEntry { id: 2, label: "無為" },
    TableEntry { id: 3, label: "暗闇" },
    TableEntry { id: 4, label: "高所" },
    TableEntry { id: 5, label: "善良" },
    TableEntry { id: 6, label: "広場" },
    TableEntry { id: 7, label: "先鋭" },
    TableEntry { id: 8, label: "猫" },
    TableEntry { id: 9, label: "疼痛性愛" },
    TableEntry { id: 10, label: "にんにく" },
    TableEntry { id: 11, label: "乗り物" },
    TableEntry { id: 12, label: "病的快活" },
    TableEntry { id: 13, label: "花" },
    TableEntry { id: 14, label: "計算" },
    TableEntry { id: 15, label: "浪費" },
    TableEntry { id: 16, label: "自己" },
    TableEntry { id: 17, label: "バレエ" },
    TableEntry { id: 18, label: "書籍約盗癖" },
    TableEntry { id: 19, label: "書物" },
    TableEntry { id: 20, label: "歯ぎしり" },
    TableEntry { id: 21, label: "悪霊" },
    TableEntry { id: 22, label: "自己愛" },
    TableEntry { id: 23, label: "地図" },
    TableEntry { id: 24, label: "飛び降り" },
    TableEntry { id: 25, label: "寒冷" },
    TableEntry { id: 26, label: "舞踏" },
    TableEntry { id: 27, label: "睡眠" },
    TableEntry { id: 28, label: "墓地" },
    TableEntry { id: 29, label: "色彩" },
    TableEntry { id: 30, label: "ピエロ" },
    TableEntry { id: 31, label: "遭遇" },
    TableEntry { id: 32, label: "殺害" },
    TableEntry { id: 33, label: "悪魔" },
    TableEntry { id: 34, label: "皮膚" },
    TableEntry { id: 35, label: "正義" },
    TableEntry { id: 36, label: "アルコール" },
    TableEntry { id: 37, label: "毛皮" },
    TableEntry { id: 38, label: "贈り物" },
    TableEntry { id: 39, label: "逃走" },
    TableEntry { id: 40, label: "外出" },
    TableEntry { id: 41, label: "自己中心" },
    TableEntry { id: 42, label: "公職" },
    TableEntry { id: 43, label: "戦慄" },
    TableEntry { id: 44, label: "知識" },
    TableEntry { id: 45, label: "静寂" },
    TableEntry { id: 46, label: "エーテル" },
    TableEntry { id: 47, label: "求婚" },
    TableEntry { id: 48, label: "笑い" },
    TableEntry { id: 49, label: "魔術" },
    TableEntry { id: 50, label: "筆記" },
    TableEntry { id: 51, label: "裸体" },
    TableEntry { id: 52, label: "幻想" },
    TableEntry { id: 53, label: "蟲" },
    TableEntry { id: 54, label: "火器" },
    TableEntry { id: 55, label: "水" },
    TableEntry { id: 56, label: "魚" },
    TableEntry { id: 57, label: "アイコン" },
    TableEntry { id: 58, label: "アイドル" },
    TableEntry { id: 59, label: "情報" },
    TableEntry { id: 60, label: "絶叫" },
    TableEntry { id: 61, label: "窃盗" },
    TableEntry { id: 62, label: "騒音" },
    TableEntry { id: 63, label: "ひも" },
    TableEntry { id: 64, label: "宝くじ" },
    TableEntry { id: 65, label: "うつ" },
    TableEntry { id: 66, label: "巨石" },
    TableEntry { id: 67, label: "音楽" },
    TableEntry { id: 68, label: "作詩" },
    TableEntry { id: 69, label: "憎悪" },
    TableEntry { id: 70, label: "偏執" },
    TableEntry { id: 71, label: "虚言" },
    TableEntry { id: 72, label: "疾病" },
    TableEntry { id: 73, label: "記録" },
    TableEntry { id: 74, label: "名前" },
    TableEntry { id: 75, label: "単語" },
    TableEntry { id: 76, label: "爪損傷" },
    TableEntry { id: 77, label: "美食" },
    TableEntry { id: 78, label: "不平" },
    TableEntry { id: 79, label: "仮面" },
    TableEntry { id: 80, label: "幽霊" },
    TableEntry { id: 81, label: "殺人" },
    TableEntry { id: 82, label: "光線" },
    TableEntry { id: 83, label: "放浪" },
    TableEntry { id: 84, label: "長者" },
    TableEntry { id: 85, label: "病的虚言" },
    TableEntry { id: 86, label: "放火" },
    TableEntry { id: 87, label: "質問" },
    TableEntry { id: 88, label: "鼻" },
    TableEntry { id: 89, label: "落書き" },
    TableEntry { id: 90, label: "列車" },
    TableEntry { id: 91, label: "知性" },
    TableEntry { id: 92, label: "テクノ" },
    TableEntry { id: 93, label: "タナトス" },
    TableEntry { id: 94, label: "宗教" },
    TableEntry { id: 95, label: "かき傷" },
    TableEntry { id: 96, label: "手術" },
    TableEntry { id: 97, label: "抜毛" },
    TableEntry { id: 98, label: "失明" },
    TableEntry { id: 99, label: "異国" },
    TableEntry { id: 100, label: "動物" },
];