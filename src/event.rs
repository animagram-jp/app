use crate::Lang;
use crate::character::{Profile, Characteristic, Skill};
use crate::js_client::{DomCmd, Operation};
use crate::data_struct::DataStruct;
use crate::character::Character;

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

    for (i, ch) in Characteristic::list().iter().enumerate() {
        let row = i + 1;
        let th_id = format!("modal_fieldset-2_table_tr-{}_th", row);
        cmds.push(DomCmd::new(Operation::SetText, &th_id, None, Some(ch.label(LANG))));
    }

    // --- fieldset-3: Skill ---
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_legend_h5", None, Some("技能")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-1", None, Some("")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-2", None, Some("")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-3", None, Some("初期")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-4", None, Some("職業")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-5", None, Some("興味")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-6", None, Some("補正値")));
    cmds.push(DomCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-7", None, Some("合計")));

    for (i, skill) in Skill::list().iter().enumerate() {
        let row = i + 1;
        let th_id    = format!("modal_fieldset-3_table_tr-{}_th", row);
        let spec_id  = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
        let base_id  = format!("modal_fieldset-3_table_tr-{}_span-1", row);
        let total_id = format!("modal_fieldset-3_table_tr-{}_span-2", row);
        let base_val = skill.base_value().to_string();
        cmds.push(DomCmd::new(Operation::SetText,  &th_id,    None, Some(&skill.label(LANG))));
        if let Some(spec) = skill.spec_label(LANG) {
            cmds.push(DomCmd::new(Operation::RemoveClass, &spec_id, None, Some("hidden")));
            cmds.push(DomCmd::new(Operation::SetValue,    &spec_id, None, Some(&spec)));
        }
        cmds.push(DomCmd::new(Operation::SetText,  &base_id,  None, Some(&base_val)));
        cmds.push(DomCmd::new(Operation::SetText,  &total_id, None, Some(&base_val)));
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

// Skill: 専門分野(td-1_input)が変わったら th のテキストを更新する
pub fn on_skill_spec_input(row: usize, skill: &Skill, spec: &str) -> Vec<DomCmd> {
    let th_id = format!("modal_fieldset-3_table_tr-{}_th", row);
    let label = skill.label_with_spec(LANG, spec);
    vec![DomCmd::new(Operation::SetText, &th_id, None, Some(&label))]
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
pub fn roll_characteristic(row: usize, char_data: &mut DataStruct) -> Vec<DomCmd> {
    if row == 0 || row > Characteristic::list().len() { return vec![]; }
    let field = Character::Characteristic(Characteristic::list()[row - 1]);
    let val   = Characteristic::list()[row - 1].generate() as i32;
    let mut vals = char_data.get(&field).map(Characteristic::decode).unwrap_or([0; 3]);
    vals[0] = val; // 初期値を上書き、変動値・補正値は据え置き
    let _ = char_data.set(&field, &Characteristic::encode(vals));
    let total = (vals[0] + vals[1] + vals[2]).max(1);
    let input_id = format!("modal_fieldset-2_table_tr-{}_input-1", row);
    let span_id  = format!("modal_fieldset-2_table_tr-{}_span", row);
    vec![
        DomCmd::new(Operation::SetValue, &input_id, None, Some(&val.to_string())),
        DomCmd::new(Operation::SetText,  &span_id,  None, Some(&total.to_string())),
    ]
}

// legend button: 全Characteristicを一括ロール
pub fn roll_all_characteristics(char_data: &mut DataStruct) -> Vec<DomCmd> {
    (1..=Characteristic::list().len())
        .flat_map(|row| roll_characteristic(row, char_data))
        .collect()
}

// ============================================================
// modal 復元: open時に pool のデータを input/span に反映
// ============================================================

pub fn restore_modal(ds: &DataStruct) -> Vec<DomCmd> {
    let mut cmds = Vec::new();

    // fieldset-1: Profile
    let profiles = [
        Profile::Name, Profile::Birthpalce, Profile::Pronoun,
        Profile::Occupation, Profile::Residence, Profile::Age,
    ];
    for (i, profile) in profiles.iter().enumerate() {
        let row = i + 1;
        let field = Character::Profile(*profile);
        if let Ok(bytes) = ds.get(&field) {
            let text = String::from_utf8_lossy(bytes).into_owned();
            let input_id = format!("modal_fieldset-1_table_tr-{}_input", row);
            cmds.push(DomCmd::new(Operation::SetValue, &input_id, None, Some(&text)));
        }
    }

    // fieldset-2: Characteristic
    for (i, ch) in Characteristic::list().iter().enumerate() {
        let row = i + 1;
        let field = Character::Characteristic(*ch);
        if let Ok(bytes) = ds.get(&field) {
            let [base, delta, bonus] = Characteristic::decode(bytes);
            let total = (base + delta + bonus).max(1);
            let input1 = format!("modal_fieldset-2_table_tr-{}_input-1", row);
            let input2 = format!("modal_fieldset-2_table_tr-{}_input-2", row);
            let input3 = format!("modal_fieldset-2_table_tr-{}_input-3", row);
            let span   = format!("modal_fieldset-2_table_tr-{}_span", row);
            if base  != 0 { cmds.push(DomCmd::new(Operation::SetValue, &input1, None, Some(&base.to_string()))); }
            if delta != 0 { cmds.push(DomCmd::new(Operation::SetValue, &input2, None, Some(&delta.to_string()))); }
            if bonus != 0 { cmds.push(DomCmd::new(Operation::SetValue, &input3, None, Some(&bonus.to_string()))); }
            cmds.push(DomCmd::new(Operation::SetText, &span, None, Some(&total.to_string())));
        }
    }

    // fieldset-3: Skill
    let skills = Skill::list();
    for (i, skill) in skills.iter().enumerate() {
        let row = i + 1;
        let field = Character::Skill(Skill::list().into_iter().nth(i).unwrap());
        let (occ, int, bonus, spec) = ds.get(&field).map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
        if !spec.is_empty() {
            let spec_id = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
            cmds.push(DomCmd::new(Operation::RemoveClass, &spec_id, None, Some("hidden")));
            cmds.push(DomCmd::new(Operation::SetValue,    &spec_id, None, Some(&spec)));
            let th_id = format!("modal_fieldset-3_table_tr-{}_th", row);
            cmds.push(DomCmd::new(Operation::SetText, &th_id, None, Some(&skill.label_with_spec(LANG, &spec))));
        }
        if occ != 0 || int != 0 || bonus != 0 {
            let base  = skill.base_value();
            let total = (base as i32 + occ as i32 + int as i32 + bonus).max(0) as u32;
            let input1 = format!("modal_fieldset-3_table_tr-{}_input-1", row);
            let input2 = format!("modal_fieldset-3_table_tr-{}_input-2", row);
            let input3 = format!("modal_fieldset-3_table_tr-{}_input-3", row);
            let span   = format!("modal_fieldset-3_table_tr-{}_span-2", row);
            if occ   != 0 { cmds.push(DomCmd::new(Operation::SetValue, &input1, None, Some(&occ.to_string()))); }
            if int   != 0 { cmds.push(DomCmd::new(Operation::SetValue, &input2, None, Some(&int.to_string()))); }
            if bonus != 0 { cmds.push(DomCmd::new(Operation::SetValue, &input3, None, Some(&bonus.to_string()))); }
            cmds.push(DomCmd::new(Operation::SetText, &span, None, Some(&total.to_string())));
        }
    }

    cmds
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
// character view: 保存時に閲覧用 span を更新
// ============================================================

pub fn update_character_view(ds: &DataStruct) -> Vec<DomCmd> {
    let mut cmds = Vec::new();

    // section-1: Profile
    let profiles = [
        Profile::Name, Profile::Birthpalce, Profile::Pronoun,
        Profile::Occupation, Profile::Residence, Profile::Age,
    ];
    for (i, profile) in profiles.iter().enumerate() {
        let row = i + 1;
        let field = Character::Profile(*profile);
        let label_id = format!("main_div_section-1_section-1_span-{}_span-1", row);
        let value_id = format!("main_div_section-1_section-1_span-{}_span-2", row);
        let outer_id = format!("main_div_section-1_section-1_span-{}", row);
        let text = ds.get(&field)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        cmds.push(DomCmd::new(Operation::SetText, &label_id, None, Some(profile.label(LANG))));
        cmds.push(DomCmd::new(Operation::SetText, &value_id, None, Some(&text)));
        if text.is_empty() {
            cmds.push(DomCmd::new(Operation::AddClass, &outer_id, None, Some("hidden")));
        } else {
            cmds.push(DomCmd::new(Operation::RemoveClass, &outer_id, None, Some("hidden")));
        }
    }

    // section-2: Characteristic
    for (i, ch) in Characteristic::list().iter().enumerate() {
        let row = i + 1;
        let field = Character::Characteristic(*ch);
        let label_id = format!("main_div_section-1_section-2_span-{}_span-1", row);
        let value_id = format!("main_div_section-1_section-2_span-{}_span-2", row);
        let outer_id = format!("main_div_section-1_section-2_span-{}", row);
        if let Ok(bytes) = ds.get(&field) {
            let [base, delta, bonus] = Characteristic::decode(bytes);
            let total = (base + delta + bonus).max(1);
            cmds.push(DomCmd::new(Operation::SetText, &label_id, None, Some(ch.label(LANG))));
            cmds.push(DomCmd::new(Operation::SetText, &value_id, None, Some(&total.to_string())));
            cmds.push(DomCmd::new(Operation::RemoveClass, &outer_id, None, Some("hidden")));
        } else {
            cmds.push(DomCmd::new(Operation::AddClass, &outer_id, None, Some("hidden")));
        }
    }

    // section-3: Skill 上位10件（pt入りを合計値降順）
    let skills = Skill::list();
    let mut skill_entries: Vec<(String, u32)> = skills.iter().enumerate().filter_map(|(i, skill)| {
        let field = Character::Skill(Skill::list().into_iter().nth(i)?);
        let (occ, int, bonus, spec) = ds.get(&field).map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
        if occ == 0 && int == 0 && bonus == 0 { return None; }
        let label = skill.label_with_spec(LANG, &spec);
        let base  = skill.base_value();
        let total = (base as i32 + occ as i32 + int as i32 + bonus).max(0) as u32;
        Some((label, total))
    }).collect();
    skill_entries.sort_by(|a, b| b.1.cmp(&a.1));

    for row in 1..=10usize {
        let label_id = format!("main_div_section-1_section-3_span-{}_span-1", row);
        let value_id = format!("main_div_section-1_section-3_span-{}_span-2", row);
        let outer_id = format!("main_div_section-1_section-3_span-{}", row);
        if let Some((label, total)) = skill_entries.get(row - 1) {
            cmds.push(DomCmd::new(Operation::SetText, &label_id, None, Some(label)));
            cmds.push(DomCmd::new(Operation::SetText, &value_id, None, Some(&total.to_string())));
            cmds.push(DomCmd::new(Operation::RemoveClass, &outer_id, None, Some("hidden")));
        } else {
            cmds.push(DomCmd::new(Operation::AddClass, &outer_id, None, Some("hidden")));
        }
    }

    cmds
}

// ============================================================
// debug select: 保存済みキャラ一覧を select に反映
// ============================================================

pub fn update_debug_select(list: &[(u32, String)], selected_id: Option<u32>) -> Vec<DomCmd> {
    let html: String = list.iter().map(|(id, name)| {
        let escaped = name
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        let sel = if Some(*id) == selected_id { " selected" } else { "" };
        format!("<option value=\"{}\"{}>{}</option>", id, sel, escaped)
    }).collect();
    vec![DomCmd::new(Operation::SetHtml, "main_div_section-1_section-1_select", None, Some(&html))]
}

// ============================================================
// modal リセット: save以外のclose時にinput・キャッシュを初期化
// ============================================================

// DataStructのリセットは呼び出し側(app.rs)でDataStruct::new()に差し替え済み
pub fn reset_modal() -> Vec<DomCmd> {

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
        let spec   = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
        let input1 = format!("modal_fieldset-3_table_tr-{}_input-1", row);
        let input2 = format!("modal_fieldset-3_table_tr-{}_input-2", row);
        let input3 = format!("modal_fieldset-3_table_tr-{}_input-3", row);
        cmds.push(DomCmd::new(Operation::AddClass,  &spec,   None, Some("hidden")));
        cmds.push(DomCmd::new(Operation::SetValue,  &spec,   None, Some("")));
        cmds.push(DomCmd::new(Operation::SetValue,  &input1, None, Some("")));
        cmds.push(DomCmd::new(Operation::SetValue,  &input2, None, Some("")));
        cmds.push(DomCmd::new(Operation::SetValue,  &input3, None, Some("")));
    }

    cmds
}
