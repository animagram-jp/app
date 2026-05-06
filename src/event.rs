use crate::Lang;
use crate::character::{Profile, Characteristic, Skill};
use crate::js_client::{DomCmd, Operation};

// ============================================================
// ログスタック (Log Stack)
// ============================================================

pub enum LogStack {
    Skill {
        // todo: ロール結果など
    },
    Characteristic {
        // todo: ロール結果など
    },
    Message(String),
}

impl std::fmt::Display for LogStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // todo: format!
        write!(f, "")
    }
}

// ============================================================
// modal open: キャラクターシート編集画面の展開
// ============================================================

const LANG: Lang = Lang::Ja;


pub fn open_modal() -> Vec<DomCmd> {
    let mut cmds = Vec::new();

    // --- header ---
    cmds.push(DomCmd::new(Operation::SetText, "modal_header_h4", None, Some("キャラクターシート")));

    // --- fieldset-1: Profile ---
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-1_legend_h5", None, Some("プロフィール")));

    let profiles = [
        Profile::Name,
        Profile::Birthpalce,
        Profile::Pronoun,
        Profile::Occupation,
        Profile::Residence,
        Profile::Age,
    ];
    for (i, profile) in profiles.iter().enumerate() {
        let row = i + 1;
        let th_id = format!("modal_fieldset-1_table_tr-{}_th", row);
        cmds.push(DomCmd::new(Operation::SetText, &th_id, None, Some(profile.label(LANG))));
    }

    // --- fieldset-2: Characteristic ---
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-2_legend_h5", None, Some("能力値")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-1", None, Some("")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-2", None, Some("初期値")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-3", None, Some("変動")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-4", None, Some("補正値")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-5", None, Some("合計")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-6", None, Some("")));

    for (i, ch) in Characteristic::all().iter().enumerate() {
        let row = i + 1;
        let th_id = format!("modal_fieldset-2_table_tr-{}_th", row);
        cmds.push(DomCmd::new(Operation::SetText, &th_id, None, Some(ch.label(LANG))));
    }

    // --- fieldset-3: Skill ---
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_legend_h5", None, Some("技能")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-1", None, Some("")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-2", None, Some("初期")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-3", None, Some("職業")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-4", None, Some("興味")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-5", None, Some("補正値")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-6", None, Some("合計")));

    for (i, skill) in Skill::default_rows().iter().enumerate() {
        let row = i + 1;
        let th_id    = format!("modal_fieldset-3_table_tr-{}_th", row);
        let base_id  = format!("modal_fieldset-3_table_tr-{}_span-1", row);
        let total_id = format!("modal_fieldset-3_table_tr-{}_span-2", row);
        let base_val = skill.base_value().to_string();
        cmds.push(DomCmd::new(Operation::SetText, &th_id,    None, Some(&skill.label(LANG))));
        cmds.push(DomCmd::new(Operation::SetText, &base_id,  None, Some(&base_val)));
        cmds.push(DomCmd::new(Operation::SetText, &total_id, None, Some(&base_val)));
    }

    cmds
}

// ============================================================
// input: リアルタイム合計計算
// ============================================================

// Characteristic: input-1(初期値) + input-2(変動値) + input-3(補正値) → span(合計) をリアルタイム更新
pub fn on_characteristic_input(row: usize, base: i32, delta: i32, bonus: i32) -> Vec<DomCmd> {
    let total = (base + delta + bonus).max(1);
    let span_id = format!("modal_fieldset-2_table_tr-{}_span", row);
    vec![DomCmd::new(Operation::SetText, &span_id, None, Some(&total.to_string()))]
}

// Skill: 職業pt(input-1) か 興味pt(input-2) か 補正値(input-3) が変わったら合計spanを更新する
pub fn on_skill_input(row: usize, base: u16, occ_pt: u16, int_pt: u16, bonus: i32) -> Vec<DomCmd> {
    let total = (base as i32 + occ_pt as i32 + int_pt as i32 + bonus).max(0) as u32;
    let span_id = format!("modal_fieldset-3_table_tr-{}_span-2", row);
    vec![DomCmd::new(Operation::SetText, &span_id, None, Some(&total.to_string()))]
}

// ============================================================
// サイコロロール → キャッシュ更新 + input/span反映
// ============================================================

// fieldset-2 の1行: ロール値をキャッシュに書き込み、input-1とspanをSetValue/SetTextで更新
pub fn roll_characteristic(row: usize, char_vals: &mut [[i32; 3]; 9]) -> Vec<DomCmd> {
    if row == 0 || row > Characteristic::all().len() { return vec![]; }
    let val = Characteristic::all()[row - 1].generate() as i32;
    char_vals[row - 1][0] = val; // 初期値を上書き、変動値・補正値は据え置き
    let total = (val + char_vals[row - 1][1] + char_vals[row - 1][2]).max(1);
    let input_id = format!("modal_fieldset-2_table_tr-{}_input-1", row);
    let span_id  = format!("modal_fieldset-2_table_tr-{}_span", row);
    vec![
        DomCmd::new(Operation::SetValue, &input_id, None, Some(&val.to_string())),
        DomCmd::new(Operation::SetText,  &span_id,  None, Some(&total.to_string())),
    ]
}

// legend button: 全Characteristicを一括ロール
pub fn roll_all_characteristics(char_vals: &mut [[i32; 3]; 9]) -> Vec<DomCmd> {
    (1..=Characteristic::all().len())
        .flat_map(|row| roll_characteristic(row, char_vals))
        .collect()
}

// ============================================================
// toast通知
// ============================================================

pub fn toast_saved() -> Vec<DomCmd> {
    vec![
        DomCmd::new(Operation::SetText,   "output_article-1_span", None, Some("💾")),
        DomCmd::new(Operation::SetText,   "output_article-1_p",    None, Some("保存しました")),
        DomCmd::new(Operation::JsClass,   "output_article-1",      None, Some("show")),
    ]
}

// ============================================================
// modal リセット: save以外のclose時にinput・キャッシュを初期化
// ============================================================

pub fn reset_modal(char_vals: &mut [[i32; 3]; 9], skill_pts: &mut [[i32; 3]; 20]) -> Vec<DomCmd> {
    *char_vals = [[0; 3]; 9];
    *skill_pts = [[0; 3]; 20];

    let mut cmds = Vec::new();

    // fieldset-1: Profile input クリア
    for row in 1..=6 {
        let id = format!("modal_fieldset-1_table_tr-{}_input", row);
        cmds.push(DomCmd::new(Operation::SetValue, &id, None, Some("")));
    }

    // fieldset-2: Characteristic input クリア
    for row in 1..=9 {
        let input1 = format!("modal_fieldset-2_table_tr-{}_input-1", row);
        let input2 = format!("modal_fieldset-2_table_tr-{}_input-2", row);
        let input3 = format!("modal_fieldset-2_table_tr-{}_input-3", row);
        cmds.push(DomCmd::new(Operation::SetValue, &input1, None, Some("")));
        cmds.push(DomCmd::new(Operation::SetValue, &input2, None, Some("")));
        cmds.push(DomCmd::new(Operation::SetValue, &input3, None, Some("")));
    }

    // fieldset-3: Skill pt input クリア
    for row in 1..=20 {
        let input1 = format!("modal_fieldset-3_table_tr-{}_input-1", row);
        let input2 = format!("modal_fieldset-3_table_tr-{}_input-2", row);
        let input3 = format!("modal_fieldset-3_table_tr-{}_input-3", row);
        cmds.push(DomCmd::new(Operation::SetValue, &input1, None, Some("")));
        cmds.push(DomCmd::new(Operation::SetValue, &input2, None, Some("")));
        cmds.push(DomCmd::new(Operation::SetValue, &input3, None, Some("")));
    }

    cmds
}
