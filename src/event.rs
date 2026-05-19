use crate::Lang;
use crate::character::{
    Profile, Characteristic, Skill,
    ArtCraftSpec, FightingSpec, FirearmsSpec, PilotSpec, ScienceSpec, SurvivalSpec,
};
use crate::js_client::{CanvasCmd, Operation, EventType, Gesture, dom::{Id, Tag}, CanvasEvent};
use crate::data_struct::DataStruct;
use crate::character::Character;
use crate::wal::WalStore;

const LANG: Lang = Lang::Ja;

// ============================================================
// canvas state
// ============================================================

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Dialog {
    #[default]
    None,
    Modal,
    Drawer,
    Select { step: u8, index: usize }, // dialog id="main_div_modal" aria-label="overlay" のセレクトUI表示状態
    Input  { step: u8, value: u32 },   // dialog id="main_div_modal" aria-label="overlay" の入力UI表示状態
}

pub struct CanvasState { // dialog + lang + editing + dom map(static)  -> canvas commands
    pub dialog:    Dialog,
    pub lang:      Lang,
    pub editing:   DataStruct,
}

impl CanvasState {
    pub fn new() -> Self {
        Self { 
            dialog: Dialog::default(), 
            lang: Lang::Ja, 
            editing: DataStruct::new(),
        }
    }
}

// ============================================================
// output commands
// ============================================================

pub fn output_commands(canvas_state: &CanvasState){
    match (canvas_state.dialog, canvas_state.lang, canvas_state.editing) {
        (Dialog::None, _, None)  =>
        (Dialog::Modal, _, None) =>
        (_, _, ) => {}
    }
        
    for 
        id = map_id(parent: )
            => cmds.push(CanvasCmd::new(Operation::SetText, id, None, ));
    cmds
}

pub fn initial_draw(canvas_state: &mut CanvasState, handler: &Coc7th) -> Vec<CanvasCmd> {
    canvas_state.editing = handler. // a data struct instance of character in main view and modal input
}

// ============================================================
// event handlers
// ============================================================

const CHARACTER_SCHEMA_NAME: &str = "character";

pub struct Coc7th {     
    characters: &'a mut Vec<u8>, // wired data of characters walstore has in memory
    log_stack:     Vec<Log>,
}

impl Coc7th {
    pub async fn ready() -> WalStore {
        WalStore::open(CHARACTER_SCHEMA_NAME).await
            .unwrap_or_else(|e| panic!("WalStore::open failed: {}", e))
    }
}

// ============================================================
// handle event
// ============================================================

pub fn handle(state: &mut CanvasState, ev: &CanvasEvent, handler: &mut Coc7th) -> Vec<CanvasCmd> {
    match (&ev.event_type, state.dialog) {
        (EventType::Click,   Dialog::None)  => todo!("normal click"),
        (EventType::Click,   Dialog::Modal) => todo!("dialog click"),
        (EventType::KeyDown, _)             => todo!("keydown"),
        (EventType::Input,   _)             => todo!("input"),
        (EventType::Change,  _)             => todo!("change"),
        (EventType::Blur,    _)             => todo!("blur"),
        (EventType::Submit,  _)             => todo!("submit"),
        _                                   => vec![],
    }
}

pub fn handle_gesture(gesture: Gesture, state: &mut CanvasState) -> Vec<CanvasCmd> {
    todo!("ジェスチャー処理")
}

// ============================================================
// map item to Id
// ============================================================

fn map_id(item: &Character, parent: &Id, n: u32) -> Vec<Id> {
    match parent {
        p if p == &Id::new(&[(Tag::Main, None)]) => {
            let section_n = match item {
                Character::Profile(_)        => 1,
                Character::Characteristic(_) => 2,
                Character::Skill(_)          => 3,
                Character::Derived(_)        => todo!(),
                Character::Equipment(_)      => todo!(),
                Character::Backstory(_)      => todo!(),
            };
            let base: Vec<(Tag, Option<u32>)> = vec![
                (Tag::Main,    None),
                (Tag::Div,     None),
                (Tag::Section, Some(1)),
                (Tag::Section, Some(section_n)),
                (Tag::Span,    Some(n)),
            ];
            vec![
                Id::new(&[base.as_slice(), &[(Tag::Span, Some(1))]].concat()),  // label
                Id::new(&[base.as_slice(), &[(Tag::Span, Some(2))]].concat()),  // value
            ]
        }
        p if p == &Id::new(&[(Tag::Modal, None)]) => {
            let fieldset_n = match item {
                Character::Profile(_)        => 1,
                Character::Characteristic(_) => 2,
                Character::Skill(_)          => 3,
                Character::Derived(_)        => 4,
                Character::Equipment(_)      => 5,
                Character::Backstory(_)      => 6,
            };
            let tr: Vec<(Tag, Option<u32>)> = vec![
                (Tag::Modal,    None),
                (Tag::Fieldset, Some(fieldset_n)),
                (Tag::Table,    None),
                (Tag::Tr,       Some(n)),
            ];
            let s = tr.as_slice();
            match item {
                Character::Profile(_) => vec![
                    // [0] th, [1] input
                    Id::new(&[s, &[(Tag::Th,    None   )]].concat()),
                    Id::new(&[s, &[(Tag::Input, None   )]].concat()),
                ],
                Character::Characteristic(_) => vec![
                    // [0] th, [1] input-1(初期値), [2] input-2(変動), [3] input-3(補正), [4] span(合計)
                    Id::new(&[s, &[(Tag::Th,    None   )]].concat()),
                    Id::new(&[s, &[(Tag::Input, Some(1))]].concat()),
                    Id::new(&[s, &[(Tag::Input, Some(2))]].concat()),
                    Id::new(&[s, &[(Tag::Input, Some(3))]].concat()),
                    Id::new(&[s, &[(Tag::Span,  None   )]].concat()),
                ],
                Character::Skill(_) => vec![
                    // [0] th, [1] span-1(base), [2] input-1(職業), [3] input-2(興味), [4] input-3(補正), [5] span-2(合計), [6] td-1_select
                    Id::new(&[s, &[(Tag::Th,     None   )]].concat()),
                    Id::new(&[s, &[(Tag::Span,   Some(1))]].concat()),
                    Id::new(&[s, &[(Tag::Input,  Some(1))]].concat()),
                    Id::new(&[s, &[(Tag::Input,  Some(2))]].concat()),
                    Id::new(&[s, &[(Tag::Input,  Some(3))]].concat()),
                    Id::new(&[s, &[(Tag::Span,   Some(2))]].concat()),
                    Id::new(&[s, &[(Tag::Td, Some(1)), (Tag::Select, None)]].concat()),
                ],
                _ => todo!(),
            }
        }
        _ => todo!(),
    }
}

// ============================================================
// action
// ============================================================

enum action {
    open_modal,
    compute_on_input,
    update_on_submit,
    toast_saved,
}

pub fn open_modal() -> Vec<CanvasCmd> {
    let mut cmds = Vec::new();

    // --- fieldset-1: Profile ---
    cmds.push(CanvasCmd::new(Operation::SetText, "modal_fieldset-1_legend_h5", None, Some("プロフィール")));

    let modal = Id::new(&[(Tag::Modal, None)]);

    for (i, profile) in character::Profile::list().iter().enumerate() {
        let ids = map_id(&Character::Profile(*profile), &modal, (i + 1) as u32);
        // ids[0]=th
        cmds.push(CanvasCmd::new(Operation::SetText, &ids[0].encode(), None, Some(profile.label(LANG))));
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
        let ids = map_id(&Character::Characteristic(*ch), &modal, (i + 1) as u32);
        // ids[0]=th
        cmds.push(CanvasCmd::new(Operation::SetText, &ids[0].encode(), None, Some(ch.label(LANG))));
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
        let ids = map_id(&Character::Skill(skill.clone()), &modal, (i + 1) as u32);
        let base_val = skill.base_value().to_string();
        // ids[0]=th, [1]=span-1(base), [2]=input-1(職業), [3]=input-2(興味), [4]=input-3(補正), [5]=span-2(合計), [6]=td-1_select
        if !matches!(skill, Skill::Custom { .. }) {
            cmds.push(CanvasCmd::new(Operation::SetText, &ids[0].encode(), None, Some(&skill.label(LANG))));
        }
        cmds.push(CanvasCmd::new(Operation::SetText, &ids[1].encode(), None, Some(&base_val)));
        cmds.push(CanvasCmd::new(Operation::SetText, &ids[5].encode(), None, Some(&base_val)));
        if let Some(html) = spec_select_html(i + 1, LANG) {
            cmds.push(CanvasCmd::new(Operation::SetHtml, &ids[6].encode(), None, Some(&html)));
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
    // todo
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
    cmds
}

// ============================================================
//  select: 保存済みキャラ一覧を select に反映
// ============================================================

pub fn update_select(list: &[(u32, String)], selected_id: Option<u32>) -> Vec<CanvasCmd> {
}

// ============================================================
// modal リセット: save以外のclose時にinput・キャッシュを初期化
// ============================================================

pub fn reset_modal() -> Vec<CanvasCmd> {
    let mut cmds = Vec::new();
    cmds
}
