use alloc::{collections::VecDeque, vec, vec::Vec};
use core::{
    default::Default,
    iter::Extend,
    option::Option::{None, Some},
    primitive::{bool, f64, u8, u32},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    arena::{APP, ARENA, COMMAND_CAPACITY, EVENT_CAPACITY, RUNNING, emit},
    event::{Event, Handler, decode_event},
    js_client::{
        Command, CommandError, EventType, Thresholds, TouchTracker, detect_device, encode_command,
    },
};

// ============================================================
// App
// ============================================================

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct App {
    touch:      TouchTracker,
    thresholds: Thresholds,
    events:     VecDeque<Event>,
    handler:    Handler,
    commands:   Vec<u8>,
    parameter:  u32,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl App {
    /// 非同期コンストラクタ。app repository の `App::init` と同じ形である。
    ///
    /// `Handler::ready(..).await` が `FileStore::new` を待つ。`await` が
    /// 要るのはここだけで、以降 `get` / `set` / `save` は同期に呼べる。
    ///
    /// worker では `run_loop` に入る前にこれを済ませる。`run_loop` は
    /// `memory_atomic_wait32` で thread ごとブロックし、その間 JavaScript の
    /// イベントループが回らないため Promise が解決しないからである。
    ///
    /// 呼べるのは dedicated worker 上に限る。`FileStore::new` が要求する
    /// `FileSystemSyncAccessHandle` は worker でしか取得できない。
    ///
    /// `App` を `static mut APP` へ格納する。`Result` を返さないのは、
    /// `wasm_bindgen` の境界を越える誤り型が `JsValue` へ変換できる必要が
    /// あり、`FileStoreError` にその変換が無いためである。panic は
    /// `#[panic_handler]` が `Command::Error` として JavaScript へ送り、
    /// worker が作り直される。起動時に store を開けない状態は続行できない
    /// ので、これでよい。
    ///
    /// 戻り値を持たないのは `App` が `Clone` を持たず (`Handler` が
    /// `FileStore` を抱える)、`APP` への格納と JavaScript への返却を
    /// 両立できないためである。`poll` / `run_loop` は `APP` 越しに駆動する。
    /// JavaScript 側 (`init.js` の `attach` / `worker.js`) はこの戻り値を
    /// 使わず `await` するだけである。
    pub async fn init(pointer_coarse: bool, viewport_width: f64, viewport_height: f64) {
        let mut app = App {
            touch:      TouchTracker::default(),
            thresholds: Thresholds::for_device(detect_device(pointer_coarse)),
            events:     VecDeque::with_capacity(EVENT_CAPACITY),
            handler:    Handler::ready(viewport_width, viewport_height).await,
            commands:   Vec::with_capacity(COMMAND_CAPACITY),
            parameter:  0,
        };

        // app repository は `Event::Ready` を積み、`dispatch` が
        // `initial_draw` を呼ぶ。ここでは待つ状態が無いため直接呼ぶ。
        let (_events, commands) = app.handler.initial_draw();
        for command in &commands {
            encode_command(&mut app.commands, command);
        }

        // `poll` / `run_loop` から取り出せるように commands を先に emit
        // してから APP へ格納する。emit 後は commands を空にしておく
        // (次の process が積む分と混ざらないようにする)。
        if !app.commands.is_empty() {
            emit(&app.commands);
            app.commands.clear();
        }

        #[allow(clippy::deref_addrof)]
        unsafe {
            *(&raw mut APP) = Some(app)
        };
    }

    /// 終了処理のコマンド列を生成する。
    ///
    /// app repository の `App::close` は `Handler::close` を呼ぶだけで
    /// 戻り値を持たない。ここでは `Handler::close` が返すコマンド列を
    /// `commands` へ書き出し、JavaScript 側が `commands` で取り出す。
    pub fn close(&mut self) {
        self.commands.clear();
        for command in &self.handler.close() {
            encode_command(&mut self.commands, command);
        }
    }

    /// process(Event) and FIFO queue command (layout `[event:u8][payload...]`)
    ///
    /// ```no_run
    /// # async fn example() {
    /// # use app::app::App;
    /// # use app::arena::APP;
    /// # use app::js_client::OPERATION_ERROR;
    /// App::init(false, 0.0, 0.0).await;
    /// let app = unsafe { (*(&raw mut APP)).as_mut() }.unwrap();
    /// app.clear();
    /// // 空フレームはデコードに失敗し、Command::Error として報告される。
    /// app.process(&[]);
    /// assert_eq!(app.commands()[0], OPERATION_ERROR);
    /// # }
    /// ```
    ///
    /// ```no_run
    /// # async fn example() {
    /// # use app::app::App;
    /// # use app::arena::APP;
    /// # use app::event::EVENT_RENDER;
    /// # use app::js_client::OPERATION_FRAME_READY;
    /// App::init(false, 0.0, 0.0).await;
    /// let app = unsafe { (*(&raw mut APP)).as_mut() }.unwrap();
    /// app.clear();
    /// app.process(&[EVENT_RENDER]);
    /// // 描画して FrameReady まで届く。
    /// assert_eq!(app.commands()[0], OPERATION_FRAME_READY);
    /// # }
    /// ```
    pub fn process(&mut self, frame: &[u8]) {
        // デコードに失敗したフレームは捨てる。1 フレーム落ちるだけで
        // 復旧できるため、報告はするが再起動は求めない。
        let Some(event) = decode_event(frame) else {
            encode_command(&mut self.commands, &Command::Error { error: CommandError::Decode });
            return;
        };

        self.events.push_back(event);

        while let Some(event) = self.events.pop_front() {
            let (new_events, new_commands) = self.dispatch(event);
            self.events.extend(new_events);
            for command in &new_commands {
                encode_command(&mut self.commands, command);
            }
        }
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    fn dispatch(&mut self, event: Event) -> (Vec<Event>, Vec<Command>) {
        let Self { handler, touch, thresholds, .. } = self;

        match event {
            Event::Canvas(canvas_event) => {
                match touch.handle(
                    &canvas_event.event_type,
                    canvas_event.pointer_id,
                    canvas_event.x,
                    canvas_event.y,
                    canvas_event.time,
                    thresholds,
                ) {
                    Some(gesture) => handler.process_gesture(&gesture, touch.active_state()),
                    // ignore PointerMove / PointerUp / PointerCancel
                    None => match canvas_event.event_type {
                        EventType::PointerMove
                        | EventType::PointerUp
                        | EventType::PointerCancel => (vec![], vec![]),
                        _ => handler.process(&canvas_event, touch.active_state()),
                    },
                }
            }
            Event::Gesture(gesture) => handler.process_gesture(&gesture, touch.active_state()),
            Event::Viewport { width, height } => handler.process_viewport(width, height),
            Event::Scroll { id, x, y } => handler.process_scroll(&id, x, y),
            Event::SetParameter { value } => {
                self.parameter = value;
                (vec![], vec![])
            }
            Event::Render => {
                // CPU render
                let _destination = unsafe { ARENA.frame_back_mut() };
                ARENA.frame_commit();
                (vec![], vec![Command::FrameReady])
            }
            Event::Shutdown => {
                unsafe { RUNNING = false };
                (vec![], self.handler.close())
            }
        }
    }
}

/// `wasm_bindgen` の境界に載せないメソッド。
///
/// `commands` は借用を返すため `#[wasm_bindgen]` を付けた impl には置けない。
/// wasm-bindgen は JavaScript 側が保持するビューの生存期間を追えず、
/// `Vec` の再確保でメモリが動くと無効なビューが残るためである。
///
/// JavaScript はこのメソッドを呼ばない。コマンド列は `poll` / `run_loop` が
/// 内部で取り出してアリーナのコマンドリングへ `emit` し、JavaScript へは
/// `arena_pointer` が返すオフセット越しに届く。
impl App {
    /// 前回 `clear` 以降に積まれたコマンド列を返す。
    ///
    /// 返るスライスは次の `clear` または `process` までのみ有効である。
    pub fn commands(&self) -> &[u8] {
        &self.commands
    }
}
