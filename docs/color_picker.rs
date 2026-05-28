// ─── JS側統合メモ（worker.jsより）────────────────────────────
//
// カレンダー本体の worker.js に統合する際の注意点:
//
// 1. スライダーは既存の dispatch() を使わず input イベントで value を直接送る
//    ['cp-h-slider','cp-s-slider','cp-l-slider'].forEach(id => {
//      document.getElementById(id)?.addEventListener('input', (e) => {
//        worker.postMessage({ type: 'event', payload: {
//          event_type: 0, target_id: e.target.id, value: +e.target.value,
//          ctrl: 0, meta: 0,
//        }});
//      });
//    });
//
// 2. HEX入力は 7文字 && validity.valid の両方を満たした時のみ送信
//    document.getElementById('cp-hex-input')?.addEventListener('input', (e) => {
//      const v = e.target.value;
//      if (v.length === 7 && e.target.validity.valid) {
//        worker.postMessage({ type: 'hex_input', payload: { value: v } });
//      }
//    });
//
// 3. swatch クリックは既存の click dispatch() で自動ルーティング済み
//    worker側で id が /^cp-swatch-(\d)$/ にマッチすれば swatchIndex() が処理する
//
// ──────────────────────────────────────────────────────────────

use wasm_bindgen::prelude::*;

// ─── op コード ────────────────────────────────────────────────
const OP_SET_ATTR: u8 = 0b01;
const OP_SET_TEXT: u8 = 0b10;

// ─── Color ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Color { r: u8, g: u8, b: u8 }

impl Color {
    fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 { return None; }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    fn to_hsl(&self) -> [f64; 3] {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let l = (max + min) / 2.0;
        let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
        let h = if d == 0.0 { 0.0 }
            else if max == r { 60.0 * (((g - b) / d) % 6.0) }
            else if max == g { 60.0 * ((b - r) / d + 2.0) }
            else             { 60.0 * ((r - g) / d + 4.0) };
        let h = if h < 0.0 { h + 360.0 } else { h };
        [h, s * 100.0, l * 100.0]
    }

    fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        let s = s / 100.0;
        let l = l / 100.0;
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
        let (r, g, b) = if h < 60.0       { (c, x, 0.0) }
            else if h < 120.0 { (x, c, 0.0) }
            else if h < 180.0 { (0.0, c, x) }
            else if h < 240.0 { (0.0, x, c) }
            else if h < 300.0 { (x, 0.0, c) }
            else              { (c, 0.0, x)  };
        Self {
            r: ((r + m) * 255.0).round() as u8,
            g: ((g + m) * 255.0).round() as u8,
            b: ((b + m) * 255.0).round() as u8,
        }
    }

    fn contrast_text(&self) -> &'static str {
        let lum = 0.2126 * (self.r as f64 / 255.0)
                + 0.7152 * (self.g as f64 / 255.0)
                + 0.0722 * (self.b as f64 / 255.0);
        if lum > 0.5 { "#000000" } else { "#FFFFFF" }
    }

    fn palette(&self) -> [Self; 7] {
        let [h, s, _] = self.to_hsl();
        [10.0, 20.0, 35.0, 50.0, 65.0, 80.0, 90.0].map(|l| Self::from_hsl(h, s, l))
    }
}

// ─── DomOpPod（内部）→ serialize_ops で JsValue へ ───────────

struct Pod { op: u8, id: &'static str, attr: &'static str, value: String }

fn sa(id: &'static str, attr: &'static str, value: String) -> Pod {
    Pod { op: OP_SET_ATTR, id, attr, value }
}
fn st(id: &'static str, value: String) -> Pod {
    Pod { op: OP_SET_TEXT, id, attr: "", value }
}

fn to_js(pods: &[Pod]) -> JsValue {
    let arr = js_sys::Array::new();
    for p in pods {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"op".into(),    &JsValue::from(p.op)).unwrap();
        js_sys::Reflect::set(&obj, &"id".into(),    &JsValue::from_str(p.id)).unwrap();
        js_sys::Reflect::set(&obj, &"attr".into(),  &JsValue::from_str(p.attr)).unwrap();
        js_sys::Reflect::set(&obj, &"value".into(), &JsValue::from_str(&p.value)).unwrap();
        arr.push(&obj);
    }
    arr.into()
}

// ─── PickerState ─────────────────────────────────────────────

#[wasm_bindgen]
pub struct PickerState {
    color: Color,
    h: f64, s: f64, l: f64,
}

#[wasm_bindgen]
impl PickerState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let color = Color { r: 99, g: 102, b: 241 };
        let [h, s, l] = color.to_hsl();
        Self { color, h, s, l }
    }

    // ── 入力ハンドラ（全部 JsValue を返す）──────────────────

    pub fn on_hex_input(&mut self, hex: &str) -> JsValue {
        if let Some(c) = Color::from_hex(hex) {
            self.color = c;
            let [h, s, l] = c.to_hsl();
            self.h = h; self.s = s; self.l = l;
            self.render()
        } else {
            JsValue::NULL // pattern属性でフロント側ブロック済み
        }
    }

    pub fn on_h_change(&mut self, h: f64) -> JsValue {
        self.h = h;
        self.color = Color::from_hsl(self.h, self.s, self.l);
        self.render()
    }

    pub fn on_s_change(&mut self, s: f64) -> JsValue {
        self.s = s;
        self.color = Color::from_hsl(self.h, self.s, self.l);
        self.render()
    }

    pub fn on_l_change(&mut self, l: f64) -> JsValue {
        self.l = l;
        self.color = Color::from_hsl(self.h, self.s, self.l);
        self.render()
    }

    pub fn on_swatch_click(&mut self, index: u8) -> JsValue {
        let palette = self.color.palette();
        if let Some(&c) = palette.get(index as usize) {
            self.color = c;
            let [h, s, l] = c.to_hsl();
            self.h = h; self.s = s; self.l = l;
        }
        self.render()
    }

    pub fn render_init(&self) -> JsValue { self.render() }

    // ── 全DOM命令生成 ─────────────────────────────────────────

    fn render(&self) -> JsValue {
        let hex      = self.color.to_hex();
        let text_col = self.color.contrast_text();
        let hue_hex  = Color::from_hsl(self.h, 100.0, 50.0).to_hex();
        let palette  = self.color.palette();

        let mut ops: Vec<Pod> = vec![
            // プレビュー
            sa("cp-preview",   "style", format!("background:{hex}")),
            sa("cp-hex-input", "style", format!("color:{text_col}")),
            sa("cp-hex-input", "value", hex.clone()),
            // スライダー値
            sa("cp-h-slider", "value", format!("{:.0}", self.h)),
            sa("cp-s-slider", "value", format!("{:.0}", self.s)),
            sa("cp-l-slider", "value", format!("{:.0}", self.l)),
            // スライダーグラデーション
            sa("cp-s-slider", "style",
                format!("background:linear-gradient(to right,#808080,{hue_hex})")),
            // 数値表示
            st("cp-h-val", format!("{:.0}°", self.h)),
            st("cp-s-val", format!("{:.0}%", self.s)),
            st("cp-l-val", format!("{:.0}%", self.l)),
        ];

        // パレット（固定7個）
        let ids: [&'static str; 7] = [
            "cp-swatch-0","cp-swatch-1","cp-swatch-2","cp-swatch-3",
            "cp-swatch-4","cp-swatch-5","cp-swatch-6",
        ];
        let tip_ids: [&'static str; 7] = [
            "cp-swatch-tip-0","cp-swatch-tip-1","cp-swatch-tip-2","cp-swatch-tip-3",
            "cp-swatch-tip-4","cp-swatch-tip-5","cp-swatch-tip-6",
        ];
        for (i, c) in palette.iter().enumerate() {
            let chex = c.to_hex();
            ops.push(sa(ids[i],     "style",   format!("background:{chex}")));
            ops.push(sa(ids[i],     "data-hex", chex.clone()));
            ops.push(st(tip_ids[i],             chex));
        }

        to_js(&ops)
    }
}
