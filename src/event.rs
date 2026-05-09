use crate::Lang;
use crate::character::{
    Profile, Characteristic, Skill,
    ArtCraftSpec, FightingSpec, FirearmsSpec, PilotSpec, ScienceSpec, SurvivalSpec,
};
use crate::js_client::{CanvasCmd, Operation};
use crate::data_struct::DataStruct;
use crate::character::Character;

// ============================================================
// canvas state
// ============================================================

// impl CanvasState {
//     fn new(wal: WalStore) -> Self {
//         Self { dialog: Dialog::default(), buf: DataStruct::new(), wal }
//     }

//     // buf.identity が指すエントリをWALから復元する。0なら何もしない。
//     fn load_saved(&mut self) {
//         let id = self.buf.identity;
//         if id.get().is_none() { return; }
//         if let Some(raw) = self.wal.get(id.0 as u64) {
//             self.buf = DataStruct::from_bytes(&raw);
//             self.buf.identity = id;
//         }
//     }

//     fn save(&mut self) {
//         if self.buf.identity.get().is_none() {
//             self.buf.identity = Id(self.wal.alloc() as u32);
//         }
//         let id = self.buf.identity;
//         self.wal.set(id.0 as u64, &self.buf.to_bytes());
//     }

//     // bufのフィールドをクリアしつつidentityは保持する
//     fn discard_buffer(&mut self) {
//         let id = self.buf.identity;
//         self.buf = DataStruct::new();
//         self.buf.identity = id;
//     }

//     fn saved_name_list(&self) -> Vec<(u32, String)> {
//         self.wal.get_all().into_iter().map(|(id, raw)| {
//             let id = id as u32;
//             let ds = DataStruct::from_bytes(&raw);
//             let name = ds.get(&Character::Profile(Profile::Name)).ok()
//                 .map(|b| String::from_utf8_lossy(b).into_owned())
//                 .filter(|s| !s.is_empty())
//                 .unwrap_or_else(|| format!("#{}", id));
//             (id, name)
//         }).collect()
//     }

//     fn on_click(&mut self, id: &dom::Id, _key: KeyName) -> Vec<CanvasCmd> {
//         let is_backdrop = id.0.len() == 1;

//         match (self.dialog, id.last_tag(), is_backdrop) {
//             (Dialog::Modal, Some(dom::Tag::Modal), true) => {
//                 self.dialog = Dialog::None;
//                 self.discard_buffer();
//                 let mut cmds = event::reset_modal();
//                 cmds.push(CanvasCmd::new(Operation::CloseModal, "modal", None, None));
//                 cmds
//             }
//             (Dialog::Drawer, Some(dom::Tag::Drawer), true) => {
//                 self.dialog = Dialog::None;
//                 vec![CanvasCmd::new(Operation::CloseModal, "drawer", None, None)]
//             }
//             (Dialog::Modal,  _, _) => self.on_click_normal(id, id.last_tag()),
//             (Dialog::Drawer, _, _) => vec![],
//             (Dialog::None, last_tag, _) => self.on_click_normal(id, last_tag),
//             (Dialog::Select { .. } | Dialog::Input { .. }, _, _) => vec![],
//         }
//     }

//     fn on_click_normal(&mut self, id: &dom::Id, last_tag: Option<&dom::Tag>) -> Vec<CanvasCmd> {
//         match last_tag {
//             Some(dom::Tag::Button) if id.encode() == "main_header_button" => {
//                 self.dialog = Dialog::Modal;
//                 self.load_saved();
//                 let mut cmds = event::open_modal();
//                 cmds.extend(event::restore_modal(&self.buf));
//                 cmds.push(CanvasCmd::new(Operation::OpenModal, "modal", None, None));
//                 cmds
//             }
//             Some(dom::Tag::Button) if {
//                 let s = &id.0;
//                 s.len() == 5
//                 && s[0].tag == dom::Tag::Modal
//                 && s[1].tag == dom::Tag::Fieldset && s[1].n == Some(2)
//                 && s[3].tag == dom::Tag::Tr
//                 && s[4].tag == dom::Tag::Button
//             } => {
//                 let row = id.0[3].n.unwrap_or(0) as usize;
//                 event::roll_characteristic(row, &mut self.buf)
//             }
//             Some(dom::Tag::Button) if id.encode() == "modal_fieldset-2_legend_button" => {
//                 event::roll_all_characteristics(&mut self.buf)
//             }
//             Some(dom::Tag::Button) if id.encode() == "modal_footer_button" => {
//                 self.dialog = Dialog::None;
//                 self.save();
//                 let mut cmds = event::toast_saved();
//                 cmds.extend(event::update_debug_select(&self.saved_name_list(), self.buf.identity.get()));
//                 cmds.extend(event::update_character_view(&self.buf));
//                 cmds.push(CanvasCmd::new(Operation::CloseModal, "modal", None, None));
//                 cmds
//             }
//             _ => vec![],
//         }
//     }

//     fn on_keydown(&mut self, _id: &dom::Id, key: KeyName) -> Vec<CanvasCmd> {
//         match key {
//             KeyName::Escape => {
//                 match self.dialog {
//                     Dialog::Modal => {
//                         self.dialog = Dialog::None;
//                         self.discard_buffer();
//                         let mut cmds = event::reset_modal();
//                         cmds.push(CanvasCmd::new(Operation::CloseModal, "modal", None, None));
//                         return cmds;
//                     }
//                     Dialog::Drawer => {
//                         self.dialog = Dialog::None;
//                         return vec![CanvasCmd::new(Operation::CloseModal, "drawer", None, None)];
//                     }
//                     Dialog::None => {}
//                     Dialog::Select { .. } | Dialog::Input { .. } => {
//                         self.dialog = Dialog::None;
//                     }
//                 }
//                 vec![]
//             }
//             KeyName::Enter => {
//                 if self.dialog == Dialog::Modal {
//                     self.dialog = Dialog::None;
//                     self.save();
//                     let mut cmds = event::toast_saved();
//                     cmds.extend(event::update_debug_select(&self.saved_name_list(), self.buf.identity.get()));
//                     cmds.extend(event::update_character_view(&self.buf));
//                     cmds.push(CanvasCmd::new(Operation::CloseModal, "modal", None, None));
//                     return cmds;
//                 }
//                 vec![]
//             }
//             _ => vec![],
//         }
//     }

//     fn on_input(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
//         let segs = &id.0;

//         // 専門分野: "modal_fieldset-3_table_tr-{row}_td-1_input"
//         if segs.len() == 6
//             && segs[0].tag == dom::Tag::Modal
//             && segs[1].tag == dom::Tag::Fieldset && segs[1].n == Some(3)
//             && segs[3].tag == dom::Tag::Tr
//             && segs[4].tag == dom::Tag::Td && segs[4].n == Some(1)
//             && segs[5].tag == dom::Tag::Input
//         {
//             let row = segs[3].n.unwrap_or(0) as usize;
//             if row == 0 || row > Skill::list().len() { return vec![]; }
//             let skills = Skill::list();
//             let skill  = &skills[row - 1];
//             let field  = Character::Skill(Skill::list().remove(row - 1));
//             let (occ, int, bonus, _) = self.buf.get(&field)
//                 .map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
//             let _ = self.buf.set(&field, &Skill::encode(occ, int, bonus, Some(value)));
//             return event::on_skill_spec_input(row, skill, value);
//         }

//         // "modal_fieldset-{fs}_table_tr-{row}_input-{col}"
//         if segs.len() != 5 { return vec![]; }
//         if segs[0].tag != dom::Tag::Modal    { return vec![]; }
//         if segs[1].tag != dom::Tag::Fieldset { return vec![]; }
//         if segs[3].tag != dom::Tag::Tr       { return vec![]; }
//         if segs[4].tag != dom::Tag::Input    { return vec![]; }

//         let fs  = segs[1].n.unwrap_or(0) as usize;
//         let row = segs[3].n.unwrap_or(0) as usize;
//         let col = segs[4].n.unwrap_or(0) as usize;

//         if row == 0 { return vec![]; }

//         match fs {
//             1 if row <= 6 => {
//                 let profiles = [
//                     Profile::Name, Profile::Birthpalce, Profile::Pronoun,
//                     Profile::Occupation, Profile::Residence, Profile::Age,
//                 ];
//                 let field = Character::Profile(profiles[row - 1]);
//                 let _ = self.buf.set(&field, value.as_bytes());
//                 vec![]
//             }
//             2 if row <= 9 => {
//                 let field = Character::Characteristic(Characteristic::list()[row - 1]);
//                 let mut vals = self.buf.get(&field)
//                     .map(Characteristic::decode)
//                     .unwrap_or([0; 3]);
//                 vals[col - 1] = value.parse().unwrap_or(0);
//                 let _ = self.buf.set(&field, &Characteristic::encode(vals));
//                 let [base, delta, bonus] = vals;
//                 event::on_characteristic_input(row, base, delta, bonus)
//             }
//             3 if row <= Skill::list().len() => {
//                 let skills = Skill::list();
//                 let field  = Character::Skill(Skill::list().remove(row - 1));
//                 let (mut occ, mut int, mut bonus, spec) = self.buf.get(&field)
//                     .map(Skill::decode)
//                     .unwrap_or((0, 0, 0, String::new()));
//                 let v: i32 = value.parse().unwrap_or(0);
//                 match col {
//                     1 => occ   = v.max(0) as u16,
//                     2 => int   = v.max(0) as u16,
//                     3 => bonus = v,
//                     _ => {}
//                 }
//                 let _ = self.buf.set(&field, &Skill::encode(occ, int, bonus, Some(&spec)));
//                 let base = skills[row - 1].base_value();
//                 event::on_skill_input(row, base, occ, int, bonus)
//             }
//             _ => vec![],
//         }
//     }

//     fn on_change(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
//         if id.encode() == "main_div_section-1_section-1_select" {
//             let char_id: u32 = value.parse().unwrap_or(0);
//             if char_id == 0 || self.wal.get(char_id as u64).is_none() { return vec![]; }
//             self.buf.identity = Id(char_id);
//             self.load_saved();
//             let mut cmds = event::update_character_view(&self.buf);
//             cmds.extend(event::update_debug_select(&self.saved_name_list(), self.buf.identity.get()));
//             return cmds;
//         }

//         let segs = &id.0;

//         // "modal_fieldset-3_table_tr-{row}_td-1_select"
//         if segs.len() == 5
//             && segs[0].tag == dom::Tag::Modal
//             && segs[1].tag == dom::Tag::Fieldset && segs[1].n == Some(3)
//             && segs[3].tag == dom::Tag::Tr
//             && segs[4].tag == dom::Tag::Select
//         {
//             let row    = segs[3].n.unwrap_or(0) as usize;
//             let inp_id = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
//             if value == "custom" {
//                 // 自由記入モード: inputをshow+focus、specをクリア
//                 let field = Character::Skill(Skill::list().remove(row - 1));
//                 let (occ, int, bonus, _) = self.buf.get(&field)
//                     .map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
//                 let _ = self.buf.set(&field, &Skill::encode(occ, int, bonus, Some("")));
//                 return vec![
//                     CanvasCmd::new(Operation::RemoveClass, &inp_id, None, Some("hidden")),
//                     CanvasCmd::new(Operation::SetValue,    &inp_id, None, Some("")),
//                     CanvasCmd::new(Operation::Focus,       &inp_id, None, None),
//                 ];
//             } else {
//                 // 固定variant選択: inputをhide、specをvalueで保存
//                 let field = Character::Skill(Skill::list().remove(row - 1));
//                 let (occ, int, bonus, _) = self.buf.get(&field)
//                     .map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
//                 let _ = self.buf.set(&field, &Skill::encode(occ, int, bonus, Some(value)));
//                 let skill  = Skill::list().remove(row - 1);
//                 let th_id  = format!("modal_fieldset-3_table_tr-{}_th", row);
//                 let base_id = format!("modal_fieldset-3_table_tr-{}_span-1", row);
//                 let mut cmds = vec![
//                     CanvasCmd::new(Operation::AddClass, &inp_id, None, Some("hidden")),
//                     CanvasCmd::new(Operation::SetText,  &th_id,  None, Some(&skill.label_with_spec(LANG, value))),
//                     CanvasCmd::new(Operation::SetText,  &base_id, None, Some(&skill.base_value().to_string())),
//                 ];
//                 // 合計も更新
//                 let (occ2, int2, bonus2, _) = self.buf.get(&field)
//                     .map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
//                 cmds.extend(event::on_skill_input(row, skill.base_value(), occ2, int2, bonus2));
//                 return cmds;
//             }
//         }

//         vec![]
//     }

//     fn on_blur(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
//         let segs = &id.0;
//         // "modal_fieldset-3_table_tr-{row}_td-1_input" + 空 → selectに戻す
//         if segs.len() == 6
//             && segs[0].tag == dom::Tag::Modal
//             && segs[1].tag == dom::Tag::Fieldset && segs[1].n == Some(3)
//             && segs[3].tag == dom::Tag::Tr
//             && segs[4].tag == dom::Tag::Td && segs[4].n == Some(1)
//             && segs[5].tag == dom::Tag::Input
//             && value.is_empty()
//         {
//             let row    = segs[3].n.unwrap_or(0) as usize;
//             let inp_id = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
//             let sel_id = format!("modal_fieldset-3_table_tr-{}_td-1_select", row);
//             return vec![
//                 CanvasCmd::new(Operation::AddClass,    &inp_id, None, Some("hidden")),
//                 CanvasCmd::new(Operation::RemoveClass, &sel_id, None, Some("hidden")),
//             ];
//         }
//         vec![]
//     }

//     fn on_gesture(&mut self, _gesture: Gesture, _id: &dom::Id) -> Vec<CanvasCmd> {
//         vec![]
//     }
// }

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


pub fn open_modal() -> Vec<CanvasCmd> {
    let mut cmds = Vec::new();

    // --- header ---
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_header_h4", None, Some("キャラクターシート")));

    // --- fieldset-1: Profile ---
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-1_legend_h5", None, Some("プロフィール")));

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
        cmds.push(CanvasCmd::new(Operation::SetText, &th_id, None, Some(profile.label(LANG))));
    }

    // --- fieldset-2: Characteristic ---
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-2_legend_h5", None, Some("能力値")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-1", None, Some("")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-2", None, Some("初期値")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-3", None, Some("変動")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-4", None, Some("補正値")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-5", None, Some("合計")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-2_table_thead_th-6", None, Some("")));

    for (i, ch) in Characteristic::list().iter().enumerate() {
        let row = i + 1;
        let th_id = format!("modal_fieldset-2_table_tr-{}_th", row);
        cmds.push(CanvasCmd::new(Operation::SetText, &th_id, None, Some(ch.label(LANG))));
    }

    // --- fieldset-3: Skill ---
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-3_legend_h5", None, Some("技能")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-1", None, Some("")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-2", None, Some("")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-3", None, Some("初期")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-4", None, Some("職業")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-5", None, Some("興味")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-6", None, Some("補正値")));
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-3_table_thead_th-7", None, Some("合計")));

    for (i, skill) in Skill::list().iter().enumerate() {
        let row      = i + 1;
        let th_id    = format!("modal_fieldset-3_table_tr-{}_th", row);
        let base_id  = format!("modal_fieldset-3_table_tr-{}_span-1", row);
        let total_id = format!("modal_fieldset-3_table_tr-{}_span-2", row);
        let base_val = skill.base_value().to_string();
        // Custom行(最終行)はth内にinputがあるのでSetTextで上書きしない
        if !matches!(skill, Skill::Custom { .. }) {
            cmds.push(CanvasCmd::new(Operation::SetText, &th_id, None, Some(&skill.label(LANG))));
        }
        cmds.push(CanvasCmd::new(Operation::SetText, &base_id,  None, Some(&base_val)));
        cmds.push(CanvasCmd::new(Operation::SetText, &total_id, None, Some(&base_val)));

        // spec持ち行: selectのoptionを生成
        if let Some(html) = spec_select_html(row, LANG) {
            let sel_id = format!("modal_fieldset-3_table_tr-{}_td-1_select", row);
            cmds.push(CanvasCmd::new(Operation::SetHtml, &sel_id, None, Some(&html)));
        }
    }

    cmds
}

/// spec持ち行のselectにセットするoption HTML。spec無し行はNone。
fn spec_select_html(row: usize, lang: Lang) -> Option<String> {
    fn build<T: crate::character::SpecLabel>(items: &[T], lang: Lang) -> String {
        let mut html = String::new();
        for item in items {
            let l = item.spec_label(lang);
            html.push_str(&format!("<option value=\"{}\">{}</option>", l, l));
        }
        html.push_str("<option value=\"custom\">自由記入…</option>");
        html
    }
    match row {
        5  => Some(build(ArtCraftSpec::list(), lang)),
        17 => Some(build(FightingSpec::list(), lang)),
        18 => Some(build(FirearmsSpec::list(), lang)),
        35 => Some(build(PilotSpec::list(), lang)),
        39 => Some(build(ScienceSpec::list(), lang)),
        43 => Some(build(SurvivalSpec::list(), lang)),
        _  => None,
    }
}

// ============================================================
// input: リアルタイム合計計算
// ============================================================

// Characteristic: input-1(初期値) + input-2(変動値) + input-3(補正値) → span(合計) をリアルタイム更新
pub fn on_characteristic_input(row: usize, base: i32, delta: i32, bonus: i32) -> Vec<CanvasCmd> {
    let total = (base + delta + bonus).max(1);
    let span_id = format!("modal_fieldset-2_table_tr-{}_span", row);
    vec![CanvasCmd::new(Operation::SetText, &span_id, None, Some(&total.to_string()))]
}

// Skill: 専門分野(td-1_input)が変わったら th のテキストを更新する
pub fn on_skill_spec_input(row: usize, skill: &Skill, spec: &str) -> Vec<CanvasCmd> {
    let th_id = format!("modal_fieldset-3_table_tr-{}_th", row);
    let label = skill.label_with_spec(LANG, spec);
    vec![CanvasCmd::new(Operation::SetText, &th_id, None, Some(&label))]
}

// Skill: 職業pt(input-1) か 興味pt(input-2) か 補正値(input-3) が変わったら合計spanを更新する
pub fn on_skill_input(row: usize, base: u16, occ_pt: u16, int_pt: u16, bonus: i32) -> Vec<CanvasCmd> {
    let total = (base as i32 + occ_pt as i32 + int_pt as i32 + bonus).max(0) as u32;
    let span_id = format!("modal_fieldset-3_table_tr-{}_span-2", row);
    vec![CanvasCmd::new(Operation::SetText, &span_id, None, Some(&total.to_string()))]
}

// ============================================================
// サイコロロール → キャッシュ更新 + input/span反映
// ============================================================

// fieldset-2 の1行: ロール値をキャッシュに書き込み、input-1とspanをSetValue/SetTextで更新
pub fn roll_characteristic(row: usize, char_data: &mut DataStruct) -> Vec<CanvasCmd> {
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
        CanvasCmd::new(Operation::SetValue, &input_id, None, Some(&val.to_string())),
        CanvasCmd::new(Operation::SetText,  &span_id,  None, Some(&total.to_string())),
    ]
}

// legend button: 全Characteristicを一括ロール
pub fn roll_all_characteristics(char_data: &mut DataStruct) -> Vec<CanvasCmd> {
    (1..=Characteristic::list().len())
        .flat_map(|row| roll_characteristic(row, char_data))
        .collect()
}

// ============================================================
// modal 復元: open時に pool のデータを input/span に反映
// ============================================================

pub fn restore_modal(ds: &DataStruct) -> Vec<CanvasCmd> {
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
            cmds.push(CanvasCmd::new(Operation::SetValue, &input_id, None, Some(&text)));
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
            if base  != 0 { cmds.push(CanvasCmd::new(Operation::SetValue, &input1, None, Some(&base.to_string()))); }
            if delta != 0 { cmds.push(CanvasCmd::new(Operation::SetValue, &input2, None, Some(&delta.to_string()))); }
            if bonus != 0 { cmds.push(CanvasCmd::new(Operation::SetValue, &input3, None, Some(&bonus.to_string()))); }
            cmds.push(CanvasCmd::new(Operation::SetText, &span, None, Some(&total.to_string())));
        }
    }

    // fieldset-3: Skill
    let skills = Skill::list();
    for (i, skill) in skills.iter().enumerate() {
        let row   = i + 1;
        let field = Character::Skill(Skill::list().into_iter().nth(i).unwrap());
        let (occ, int, bonus, spec) = ds.get(&field).map(Skill::decode).unwrap_or((0, 0, 0, String::new()));

        if !spec.is_empty() {
            if spec_select_html(row, LANG).is_some() {
                // spec持ち行: selectに値をセット、自由記入ならinputも復元
                let sel_id = format!("modal_fieldset-3_table_tr-{}_td-1_select", row);
                let inp_id = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
                // 固定variantに一致すればselectにそのまま、なければcustomモード
                let is_fixed = spec_select_html(row, LANG)
                    .map(|h| h.contains(&format!("value=\"{}\"", spec)))
                    .unwrap_or(false);
                if is_fixed {
                    cmds.push(CanvasCmd::new(Operation::SetValue, &sel_id, None, Some(&spec)));
                } else {
                    cmds.push(CanvasCmd::new(Operation::SetValue,    &sel_id, None, Some("custom")));
                    cmds.push(CanvasCmd::new(Operation::RemoveClass, &inp_id, None, Some("hidden")));
                    cmds.push(CanvasCmd::new(Operation::SetValue,    &inp_id, None, Some(&spec)));
                }
            } else {
                // LanguageOther / Custom行: inputに直接
                let inp_id = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
                cmds.push(CanvasCmd::new(Operation::RemoveClass, &inp_id, None, Some("hidden")));
                cmds.push(CanvasCmd::new(Operation::SetValue,    &inp_id, None, Some(&spec)));
            }
            let th_id = format!("modal_fieldset-3_table_tr-{}_th", row);
            cmds.push(CanvasCmd::new(Operation::SetText, &th_id, None, Some(&skill.label_with_spec(LANG, &spec))));
        }

        if occ != 0 || int != 0 || bonus != 0 {
            let base  = skill.base_value();
            let total = (base as i32 + occ as i32 + int as i32 + bonus).max(0) as u32;
            let input1 = format!("modal_fieldset-3_table_tr-{}_input-1", row);
            let input2 = format!("modal_fieldset-3_table_tr-{}_input-2", row);
            let input3 = format!("modal_fieldset-3_table_tr-{}_input-3", row);
            let span   = format!("modal_fieldset-3_table_tr-{}_span-2", row);
            if occ   != 0 { cmds.push(CanvasCmd::new(Operation::SetValue, &input1, None, Some(&occ.to_string()))); }
            if int   != 0 { cmds.push(CanvasCmd::new(Operation::SetValue, &input2, None, Some(&int.to_string()))); }
            if bonus != 0 { cmds.push(CanvasCmd::new(Operation::SetValue, &input3, None, Some(&bonus.to_string()))); }
            cmds.push(CanvasCmd::new(Operation::SetText, &span, None, Some(&total.to_string())));
        }
    }

    cmds
}

// ============================================================
// toast通知
// ============================================================

pub fn toast_saved() -> Vec<CanvasCmd> {
    vec![
        CanvasCmd::new(Operation::SetText,   "output_article-1_span", None, Some("💾")),
        CanvasCmd::new(Operation::SetText,   "output_article-1_p",    None, Some("保存しました")),
        CanvasCmd::new(Operation::JsClass,   "output_article-1",      None, Some("show")),
    ]
}

// ============================================================
// character view: 保存時に閲覧用 span を更新
// ============================================================

pub fn update_character_view(ds: &DataStruct) -> Vec<CanvasCmd> {
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
        cmds.push(CanvasCmd::new(Operation::SetText, &label_id, None, Some(profile.label(LANG))));
        cmds.push(CanvasCmd::new(Operation::SetText, &value_id, None, Some(&text)));
        if text.is_empty() {
            cmds.push(CanvasCmd::new(Operation::AddClass, &outer_id, None, Some("hidden")));
        } else {
            cmds.push(CanvasCmd::new(Operation::RemoveClass, &outer_id, None, Some("hidden")));
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
            cmds.push(CanvasCmd::new(Operation::SetText, &label_id, None, Some(ch.label(LANG))));
            cmds.push(CanvasCmd::new(Operation::SetText, &value_id, None, Some(&total.to_string())));
            cmds.push(CanvasCmd::new(Operation::RemoveClass, &outer_id, None, Some("hidden")));
        } else {
            cmds.push(CanvasCmd::new(Operation::AddClass, &outer_id, None, Some("hidden")));
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
            cmds.push(CanvasCmd::new(Operation::SetText, &label_id, None, Some(label)));
            cmds.push(CanvasCmd::new(Operation::SetText, &value_id, None, Some(&total.to_string())));
            cmds.push(CanvasCmd::new(Operation::RemoveClass, &outer_id, None, Some("hidden")));
        } else {
            cmds.push(CanvasCmd::new(Operation::AddClass, &outer_id, None, Some("hidden")));
        }
    }

    cmds
}

// ============================================================
// debug select: 保存済みキャラ一覧を select に反映
// ============================================================

pub fn update_debug_select(list: &[(u32, String)], selected_id: Option<u32>) -> Vec<CanvasCmd> {
    let html: String = list.iter().map(|(id, name)| {
        let escaped = name
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        let sel = if Some(*id) == selected_id { " selected" } else { "" };
        format!("<option value=\"{}\"{}>{}</option>", id, sel, escaped)
    }).collect();
    vec![CanvasCmd::new(Operation::SetHtml, "main_div_section-1_section-1_select", None, Some(&html))]
}

// ============================================================
// modal リセット: save以外のclose時にinput・キャッシュを初期化
// ============================================================

// DataStructのリセットは呼び出し側(app.rs)でDataStruct::new()に差し替え済み
pub fn reset_modal() -> Vec<CanvasCmd> {

    let mut cmds = Vec::new();

    // fieldset-1: Profile input クリア
    for row in 1..=6 {
        let id = format!("modal_fieldset-1_table_tr-{}_input", row);
        cmds.push(CanvasCmd::new(Operation::SetValue, &id, None, Some("")));
    }

    // fieldset-2: Characteristic input クリア
    for row in 1..=9 {
        let input1 = format!("modal_fieldset-2_table_tr-{}_input-1", row);
        let input2 = format!("modal_fieldset-2_table_tr-{}_input-2", row);
        let input3 = format!("modal_fieldset-2_table_tr-{}_input-3", row);
        cmds.push(CanvasCmd::new(Operation::SetValue, &input1, None, Some("")));
        cmds.push(CanvasCmd::new(Operation::SetValue, &input2, None, Some("")));
        cmds.push(CanvasCmd::new(Operation::SetValue, &input3, None, Some("")));
    }

    // fieldset-3: Skill pt input クリア
    let skill_rows = Skill::list().len();
    for row in 1..=skill_rows {
        let input1 = format!("modal_fieldset-3_table_tr-{}_input-1", row);
        let input2 = format!("modal_fieldset-3_table_tr-{}_input-2", row);
        let input3 = format!("modal_fieldset-3_table_tr-{}_input-3", row);
        cmds.push(CanvasCmd::new(Operation::SetValue, &input1, None, Some("")));
        cmds.push(CanvasCmd::new(Operation::SetValue, &input2, None, Some("")));
        cmds.push(CanvasCmd::new(Operation::SetValue, &input3, None, Some("")));
        // spec持ち行: selectをデフォルトに戻し、inputをhidden
        if spec_select_html(row, LANG).is_some() {
            let sel_id = format!("modal_fieldset-3_table_tr-{}_td-1_select", row);
            let inp_id = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
            cmds.push(CanvasCmd::new(Operation::SetValue,  &sel_id, None, Some("")));
            cmds.push(CanvasCmd::new(Operation::AddClass,  &inp_id, None, Some("hidden")));
            cmds.push(CanvasCmd::new(Operation::SetValue,  &inp_id, None, Some("")));
        }
        // Custom行: th_inputとtd-1_inputをクリア
        if row == skill_rows {
            let th_inp = format!("modal_fieldset-3_table_tr-{}_th_input", row);
            let td_inp = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
            cmds.push(CanvasCmd::new(Operation::SetValue, &th_inp, None, Some("")));
            cmds.push(CanvasCmd::new(Operation::SetValue, &td_inp, None, Some("")));
        }
    }

    cmds
}
