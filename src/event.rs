use alloc::{vec, vec::Vec};
use core::{
    matches,
    option::Option::{self, None, Some},
    primitive::{f64, u8, u32},
};

use arbitrary_int::u2;

#[cfg(feature = "worker")]
use crate::file_store::FileStore;
#[cfg(feature = "worker")]
use crate::js_client::CommandError;
use crate::{
    Lang,
    arena::Decoder,
    data_struct::DataStruct,
    js_client::{CanvasEvent, Command, EventType, Gesture, KeyName, PointerState, dom, name},
};

// ============================================================
// constant
// ============================================================

/// 失敗しうる操作を続けて試す回数。
///
/// `FileStore` の `save` / `discard` / `compact` はいずれも同じ
/// `FileSystemSyncAccessHandle` を叩き、同じ理由で失敗する。閾値を
/// 操作ごとに分ける理由が無いため、ひとつに揃えてある。
///
/// `InvalidState` は「ハンドルが閉じている」だけでなく「書き込み自体が
/// 何らかの理由で失敗した」も含む (whatwg/fs の仕様)。前者は復帰に
/// `FileStore::new` の `await` が要り `run_loop` の中では待てないが、
/// 後者は一過性であり、そのまま呼び直せば済む。失敗しても未保存の
/// 差分は保持されるため、再試行でデータは失われない。
///
/// この回数を続けて失敗した場合はハンドルの失効とみなし、
/// `CommandError::FileStore` を送って worker を作り直す。
#[cfg(feature = "worker")]
const RETRY_LIMIT: u8 = 3;

// ============================================================
// receive (event frame)
// ============================================================
//
// event 番号は JavaScript 側 (init.js の send) と対応。
// 値を追加/変更する際は両方を揃えて更新する。
//
// バイト列を前から順に読むため、フィールドの順序が protocol の一部になる。

/// pointer / key / input / change / focus など DOM 由来のイベント。
pub const EVENT_CANVAS: u8 = 1;
/// `resize` イベント。
pub const EVENT_VIEWPORT: u8 = 2;
/// `scroll` イベント。
pub const EVENT_SCROLL: u8 = 3;
// 4, 5 は空いている。番号は詰めない。`init.js` と揃える必要があり、
// 既存の番号をずらすと双方を同時に直す羽目になるためである。
/// 描画パラメータを設定する。
pub const EVENT_SET_PARAMETER: u8 = 6;
/// 1 フレーム描画してトリプルバッファへ公開する。
pub const EVENT_RENDER: u8 = 7;
/// `run_loop` を終了させる。
pub const EVENT_SHUTDOWN: u8 = 8;

// ============================================================
// Event
// ============================================================

/// JavaScript から Wasm へ届く入力。
///
/// `Ready` を持たない。`App::init` が `Handler::ready` を直接 `await` して
/// から dispatch ループに入るため、起動完了を dispatch 側で待つ状態が要らない。
/// `Viewport` / `Scroll` は `Canvas` の `event_type` としても表現できるが、
/// payload の形が異なるため別 variant に分けてある。`SetParameter` /
/// `Render` / `Shutdown` はアリーナ由来のイベントを同じ enum に統合した
/// ものである。
pub enum Event {
    /// DOM 由来のイベント。
    Canvas(CanvasEvent),
    /// 認識済みのジェスチャ。`dispatch` が自ら積む。
    Gesture(Gesture),
    /// `resize` イベント。
    Viewport { width: f64, height: f64 },
    /// `scroll` イベント。
    Scroll { id: dom::Id, x: f64, y: f64 },
    /// 描画パラメータを設定する。
    SetParameter { value: u32 },
    /// 1 フレーム描画してトリプルバッファへ公開する。
    Render,
    /// `run_loop` を終了させる。
    Shutdown,
}

/// JavaScript から届いた 1 イベントフレームを解釈する。壊れていれば None。
///
/// 長さが足りなければ欠けたフィールドを補わず None を返し、
/// `App::process` がそのフレームを捨てる。
///
/// ```
/// # use app::event::{decode_event, Event, EVENT_RENDER, EVENT_SHUTDOWN};
/// assert!(matches!(decode_event(&[EVENT_RENDER]), Some(Event::Render)));
/// assert!(matches!(decode_event(&[EVENT_SHUTDOWN]), Some(Event::Shutdown)));
/// // 未知の event 番号は None。
/// assert!(decode_event(&[200]).is_none());
/// // 空フレームも None。
/// assert!(decode_event(&[]).is_none());
/// ```
pub fn decode_event(frame: &[u8]) -> Option<Event> {
    let mut decoder = Decoder::new(frame);
    let kind = decoder.u8()?;
    Some(match kind {
        EVENT_CANVAS => Event::Canvas(CanvasEvent {
            event_type: EventType::decode_u8(decoder.u8()?),
            id:         decoder.id()?,
            key:        KeyName::decode_u8(decoder.u8()?),
            value:      decoder.string()?,
            x:          decoder.f32()? as f64,
            y:          decoder.f32()? as f64,
            time:       decoder.f64()?,
            pointer_id: decoder.u32()?,
        }),
        EVENT_VIEWPORT => {
            Event::Viewport { width: decoder.f32()? as f64, height: decoder.f32()? as f64 }
        }
        EVENT_SCROLL => Event::Scroll {
            id: decoder.id()?,
            x:  decoder.f32()? as f64,
            y:  decoder.f32()? as f64,
        },
        EVENT_SET_PARAMETER => Event::SetParameter { value: decoder.u32()? },
        EVENT_RENDER => Event::Render,
        EVENT_SHUTDOWN => Event::Shutdown,
        _ => return None,
    })
}

// ============================================================
// event handler
// ============================================================

/// キャラクターシートの表示状態。
pub enum CharacterSheet {
    /// 閲覧のみ。
    Immutable,
    /// 編集可。
    Editable,
}

/// 表示中のダイアログ。
pub enum Dialog {
    None,
    Drawer,                          // #drawer
    Select { step: u8, index: u32 }, // #main_modal セレクトUI表示状態
    Input { step: u8, value: u32 },  // #main_modal 入力UI表示状態
}

/// セッションログ。
pub struct Log;

/// 永続化する store の名前。
#[cfg(feature = "worker")]
const CHARACTER_SCHEMA_NAME: &str = "characters";

/// 画面状態を保持し、イベントをコマンド列へ変換する。
///
/// `characters` は `FileStore` をそのまま持つ。OPFS は
/// `FileSystemSyncAccessHandle` という同期ハンドルを返し、取得さえ済めば
/// `get` / `set` / `save` は同期で呼べるため、往復にする必要が無い。
pub struct Handler {
    character_sheet: CharacterSheet,
    dialog:          Dialog,
    lang:            Lang,
    last_toast:      u2,
    character:       DataStruct,
    /// `worker` feature でのみ持つ。OPFS の `FileSystemSyncAccessHandle` は
    /// dedicated worker でしか取得できないため、main thread 構成では
    /// フィールドごと存在しない。`save` を呼ぶコードは型検査で弾かれる。
    #[cfg(feature = "worker")]
    characters:      FileStore,
    logs:            Vec<Log>,
    /// `characters` への操作が続けて失敗した回数。成功すると 0 に戻る。
    #[cfg(feature = "worker")]
    store_failures:  u8,
}

impl Handler {
    /// viewport の寸法を受けて初期状態を作る。
    ///
    /// `async fn` である。`FileStore::new` は OPFS の
    /// ハンドル取得に `await` を要するが、`await` できるのはここだけで足りる。
    ///
    /// `run_loop` は `memory_atomic_wait32` で thread ごとブロックするため、
    /// その中では JavaScript のイベントループが回らず Promise が解決しない。
    /// したがって `await` は `run_loop` に入る前に済ませる。worker の init
    /// フェーズがその場所であり、`FileStore::new` の doc もそう指示している。
    ///
    /// 継続的にコールバックが来る WebAPI (WebSocket / WebRTC / WebGPU) は
    /// この形では扱えない。それらは JavaScript 側に置き、イベントリング
    /// 越しに `Event` として届ける。`FileStore` が例外なのは、OPFS が
    /// 同期ハンドルを返し、取得後は `run_loop` の中から直接呼べるためである。
    /// 取得に失敗した場合は panic する。panic は `#[panic_handler]` が
    /// `Command::Error` として JavaScript へ送る。
    pub async fn ready(_viewport_width: f64, _viewport_height: f64) -> Self {
        Self {
            character_sheet: CharacterSheet::Immutable,
            dialog: Dialog::None,
            lang: Lang::Ja,
            last_toast: u2::new(1),
            character: DataStruct::new(0, 0.0, 256),
            #[cfg(feature = "worker")]
            characters: FileStore::new(CHARACTER_SCHEMA_NAME)
                .await
                .unwrap_or_else(|e| panic!("FileStore::new failed: {e}")),
            logs: Vec::new(),
            #[cfg(feature = "worker")]
            store_failures: 0,
        }
    }

    /// 終了時のコマンド列を返す。
    ///
    /// `Vec<Command>` を返すのは `App::dispatch` の戻り値の形に揃えるため
    /// であり、現状は常に空を返す。
    pub fn close(&self) -> Vec<Command> {
        #[cfg(feature = "worker")]
        self.characters.close();
        vec![]
    }

    /// 未保存の変更を書き出す。
    ///
    /// 失敗しても未保存の差分は `FileStore` 側に残るため、次の呼び出しで
    /// 書き直せる。`RETRY_LIMIT` 回続けて失敗した場合だけ、ハンドルが
    /// 失効したとみなして `Command::Error { error: CommandError::FileStore(e) }`
    /// を返す。
    ///
    /// ハンドルの再取得は wasm 内で完結しない。`FileStore::new` は
    /// `navigator.storage.getDirectory()` から `createSyncAccessHandle()` まで
    /// すべて `await` を要し、`run_loop` は thread ごとブロックしているため
    /// Promise が解決しない。したがって復帰は worker の作り直しに委ねる。
    /// `CommandError::FileStore` は `is_serious()` が真であり、JavaScript 側が
    /// `restart()` する。新しい worker の `App::init` が開き直す。
    ///
    /// 直前の `save` が成功した時点までは残る。`FileStore` は log ベースで
    /// あり、確定していない末尾は次回の `save` が切り落とすためである。
    #[cfg(feature = "worker")]
    pub fn save(&mut self) -> Vec<Command> {
        match self.characters.save() {
            Ok(()) => {
                self.store_failures = 0;
                vec![]
            }
            Err(_) if self.store_failures + 1 < RETRY_LIMIT => {
                // 一過性の書き込み失敗とみなす。差分は保持されている。
                self.store_failures += 1;
                vec![]
            }
            Err(e) => {
                self.store_failures = 0;
                vec![Command::Error { error: CommandError::FileStore(e) }]
            }
        }
    }

    /// viewport の寸法変更を処理する。
    pub fn process_viewport(&mut self, _width: f64, _height: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    /// `scroll` を処理する。
    pub fn process_scroll(
        &mut self,
        _id: &dom::Id,
        _x: f64,
        _y: f64,
    ) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    /// 起動直後の描画。
    ///
    /// `body` の `hidden` を外して画面を見せる。`Handler::ready` の
    /// `await` で `FileStore` は取得済みであり、待つものが無い。
    pub fn initial_draw(&self) -> (Vec<Event>, Vec<Command>) {
        let commands = vec![Command::RemoveAttribute {
            id:        dom::Id::new(&[(dom::Tag::Body, None)]),
            attribute: name::HIDDEN,
        }];
        (vec![], commands)
    }

    /// DOM 由来のイベントを処理する。
    ///
    /// header の 3 番目のボタンで、キャラクターシートの閲覧と編集を
    /// 入れ替える。
    pub fn process(
        &mut self,
        event: &CanvasEvent,
        _state: &PointerState,
    ) -> (Vec<Event>, Vec<Command>) {
        let toggle = dom::Id::new(&[(dom::Tag::Header, None), (dom::Tag::Button, Some(3))]);
        if !matches!(event.event_type, EventType::Click) || event.id != toggle {
            return (vec![], vec![]);
        }

        let section = |n| dom::Id::new(&[(dom::Tag::Main, None), (dom::Tag::Section, Some(n))]);
        let (shown, hidden) = match self.character_sheet {
            CharacterSheet::Immutable => {
                self.character_sheet = CharacterSheet::Editable;
                (1, 2)
            }
            CharacterSheet::Editable => {
                self.character_sheet = CharacterSheet::Immutable;
                (2, 1)
            }
        };
        let commands = vec![
            Command::RemoveClass { id: section(shown), value: name::HIDDEN },
            Command::AddClass { id: section(hidden), value: name::HIDDEN },
        ];
        (vec![], commands)
    }

    /// ジェスチャを処理する。
    pub fn process_gesture(
        &mut self,
        _gesture: &Gesture,
        _state: &PointerState,
    ) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }
}
